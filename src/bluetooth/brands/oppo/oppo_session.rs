// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::compatibility::{self, NoiseMode};
use crate::protocol::{
    AppPacket, BassEngineLevel, BatteryState, CustomEqualizer, EarSide, FEATURE_BASS_ENGINE,
    FEATURE_HI_RES, FirmwareVersions, FrameDecoder, KeyFunction, NoiseCycleState, PairedDevice,
    PairedDeviceAction, PairedDeviceAddress, WearState, capability_bitmap_allows,
    encode_custom_equalizer_command, encode_find_earbuds, encode_fit_test, encode_frame,
    encode_set_head_motion_type, encode_set_key_function, encode_set_noise_cycle,
    encode_set_priority_device, encode_set_prompt_volume, noise_mode_readback_matches_for,
    noise_mode_write_payload_for, parse_bass_engine_event, parse_bass_engine_response,
    parse_battery_event, parse_battery_response, parse_color_response, parse_custom_equalizer_list,
    parse_equalizer_event, parse_equalizer_response, parse_feature_flags_response,
    parse_firmware_response, parse_fit_test_event, parse_head_motion_type_response,
    parse_noise_cycle_response, parse_noise_mode_event, parse_noise_mode_response,
    parse_paired_devices_event, parse_paired_devices_response, parse_priority_device_response,
    parse_prompt_volume_response, parse_spatial_audio_response, parse_touch_controls_response,
    parse_wear_event, parse_wear_response,
};

use crate::bluetooth::transport::{is_socket_timeout, open_rfcomm};
use crate::bluetooth::{DeviceChoice, UiUpdate, WorkerCommand};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub struct OppoDeviceSnapshot {
    pub name: String,
    pub product_id: u32,
    pub color_id: Option<u8>,
    pub model: &'static str,
    pub read_only: bool,
    pub battery: BatteryState,
    pub firmware: FirmwareVersions,
    pub wear: WearState,
    pub supports_wear_detection: bool,
    pub noise_mode: Option<u8>,
    pub noise_mode_indices: Vec<u8>,
    pub noise_mode_support_note: String,
    pub noise_modes: &'static [NoiseMode],
    pub supports_paired_devices: bool,
    pub detected_features: Vec<String>,
    pub probe_errors: Vec<String>,
    pub paired_devices: Result<Vec<crate::protocol::PairedDevice>, String>,
    pub paired_device_controls_enabled: Result<bool, String>,
    pub supports_equalizer: bool,
    pub supports_custom_equalizer: bool,
    pub custom_equalizers: Vec<CustomEqualizer>,
    pub equalizer_preset: Option<u8>,
    pub supports_spatial_audio: bool,
    pub supports_head_tracking: bool,
    pub spatial_audio_mode: Option<u8>,
    pub supports_game_mode: bool,
    pub game_mode_enabled: Option<bool>,
    pub game_mode_feature_id: u8,
    pub supports_bass_engine: bool,
    pub bass_engine_enabled: Option<bool>,
    pub bass_engine_level: Option<BassEngineLevel>,
    pub supports_hi_res: bool,
    pub hi_res_enabled: Option<bool>,
    pub supports_touch_customization: bool,
    pub key_functions: Vec<KeyFunction>,
    pub noise_cycle: NoiseCycleState,
    pub is_finding_earbuds: bool,
    pub supports_prompt_volume: bool,
    pub prompt_volume: Option<u8>,
    pub supports_head_motion: bool,
    pub head_motion_type: Option<u8>,
    pub supports_fit_detection: bool,
    pub supports_find_device: bool,
    pub supports_sleep_detection: bool,
    pub in_ear_detection_enabled: Option<bool>,
    pub sleep_detection_enabled: Option<bool>,
    pub dual_device_enabled: Option<bool>,
    pub preferred_device: Option<PairedDeviceAddress>,
    #[allow(dead_code)]
    pub capabilities: Vec<u8>,
}

pub type DeviceSnapshot = OppoDeviceSnapshot;

pub fn spawn_oppo_monitor(
    device: DeviceChoice,
    updates: Sender<UiUpdate>,
    commands: Receiver<WorkerCommand>,
) -> std::io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("enco-bluetooth".to_string())
        .spawn(move || {
            let result = monitor_device(device, &updates, &commands);
            if let Err(error) = result {
                let _ = updates.send(UiUpdate::Status(error));
            }
            let _ = updates.send(UiUpdate::Finished);
        })
}

pub use spawn_oppo_monitor as spawn_monitor;

fn monitor_device(
    device: DeviceChoice,
    updates: &Sender<UiUpdate>,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let mut stream = open_rfcomm(device.address)
        .map_err(|error| format!("无法打开耳机 RFCOMM 控制通道：{error}"))?;
    let (mut snapshot, mut decoder) = initialize_session(&mut stream, &device.name, Some(updates))?;

    let _ = updates.send(UiUpdate::Connected {
        device: device.clone(),
    });
    let _ = updates.send(UiUpdate::NoiseControlsEnabled(
        !snapshot.read_only && !snapshot.noise_modes.is_empty(),
    ));
    let _ = updates.send(UiUpdate::NoiseModeOptions {
        indices: snapshot.noise_mode_indices.clone(),
        note: snapshot.noise_mode_support_note.clone(),
        modes: snapshot.noise_modes.to_vec(),
    });
    let _ = updates.send(UiUpdate::ProductId {
        product_id: snapshot.product_id,
        color_id: snapshot.color_id,
    });
    let _ = updates.send(UiUpdate::Firmware(snapshot.firmware));
    let _ = updates.send(UiUpdate::Wear(snapshot.wear.clone()));
    let _ = updates.send(UiUpdate::Battery(snapshot.battery));
    let _ = updates.send(UiUpdate::PairedDeviceControlsEnabled(
        snapshot.paired_device_controls_enabled.clone(),
    ));
    let presets = compatibility::equalizer_presets_for_product(snapshot.product_id).to_vec();
    let _ = updates.send(UiUpdate::SoundFeatures {
        supports_equalizer: snapshot.supports_equalizer,
        supports_custom_equalizer: snapshot.supports_custom_equalizer,
        supports_spatial_audio: snapshot.supports_spatial_audio,
        supports_head_tracking: snapshot.supports_head_tracking,
        supports_game_mode: snapshot.supports_game_mode,
        supports_bass_engine: snapshot.supports_bass_engine,
        supports_hi_res: snapshot.supports_hi_res,
        presets,
    });
    if snapshot.supports_custom_equalizer {
        let _ = updates.send(UiUpdate::CustomEqualizers(
            snapshot.custom_equalizers.clone(),
        ));
    }
    if let Some(preset) = snapshot.equalizer_preset {
        let _ = updates.send(UiUpdate::Equalizer(preset));
    }
    if let Some(mode) = snapshot.spatial_audio_mode {
        let _ = updates.send(UiUpdate::SpatialAudio(mode));
    }
    if let Some(enabled) = snapshot.game_mode_enabled {
        let _ = updates.send(UiUpdate::GameMode(enabled));
    }
    if let Some(enabled) = snapshot.bass_engine_enabled {
        let _ = updates.send(UiUpdate::BassEngineEnabled(enabled));
    }
    if let Some(level) = snapshot.bass_engine_level {
        let _ = updates.send(UiUpdate::BassEngineLevel(level));
    }
    if let Some(enabled) = snapshot.hi_res_enabled {
        let _ = updates.send(UiUpdate::HiRes(enabled));
    }
    let mut paired_devices = snapshot.paired_devices.clone();
    let _ = updates.send(UiUpdate::PairedDevices(snapshot.paired_devices));
    if let Some(mode) = snapshot.noise_mode {
        let _ = updates.send(UiUpdate::NoiseMode(mode));
    }
    let gesture_rows = compatibility::gesture_rows_for_product(snapshot.product_id).to_vec();
    let _ = updates.send(UiUpdate::ControlsFeatures {
        supports_touch_customization: snapshot.supports_touch_customization,
        supports_dual_device: snapshot.supports_paired_devices,
        supports_wear_detection: snapshot.supports_wear_detection,
        supports_sleep_detection: snapshot.supports_sleep_detection,
        supports_head_motion: snapshot.supports_head_motion,
        supports_find_device: snapshot.supports_find_device,
        supports_fit_detection: snapshot.supports_fit_detection,
        supports_prompt_volume: snapshot.supports_prompt_volume,
        gesture_rows,
    });
    if !snapshot.key_functions.is_empty() {
        let _ = updates.send(UiUpdate::KeyFunctions(snapshot.key_functions.clone()));
    }
    let _ = updates.send(UiUpdate::NoiseCycle(snapshot.noise_cycle));
    let _ = updates.send(UiUpdate::IsFindingEarbuds(snapshot.is_finding_earbuds));
    if let Some(vol) = snapshot.prompt_volume {
        let _ = updates.send(UiUpdate::PromptVolume(vol));
    }
    if let Some(motion) = snapshot.head_motion_type {
        let _ = updates.send(UiUpdate::HeadMotionType(motion));
    }
    if let Some(enabled) = snapshot.in_ear_detection_enabled {
        let _ = updates.send(UiUpdate::InEarDetectionEnabled(enabled));
    }
    if let Some(enabled) = snapshot.sleep_detection_enabled {
        let _ = updates.send(UiUpdate::SleepDetectionEnabled(enabled));
    }
    if let Some(enabled) = snapshot.dual_device_enabled {
        let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(enabled));
    }
    let _ = updates.send(UiUpdate::PreferredDevice(snapshot.preferred_device));

    let _ = updates.send(UiUpdate::Status(if snapshot.read_only {
        let detected = if snapshot.detected_features.is_empty() {
            "未确认可用的状态查询".to_string()
        } else {
            format!("已检测只读功能：{}", snapshot.detected_features.join("、"))
        };
        let errors = if snapshot.probe_errors.is_empty() {
            String::new()
        } else {
            format!("；部分探测失败：{}", snapshot.probe_errors.join("；"))
        };
        format!("已连接控制通道；{detected}{errors}")
    } else {
        "已连接控制通道，正在同步耳机状态".to_string()
    }));
    let mut current_wear = snapshot.wear.clone();
    let mut current_custom_equalizers = snapshot.custom_equalizers.clone();
    let mut current_key_functions = snapshot.key_functions.clone();
    let mut current_noise_cycle = snapshot.noise_cycle;
    let mut current_prompt_volume = snapshot.prompt_volume;
    let mut current_head_motion = snapshot.head_motion_type;
    let mut current_in_ear_detection = snapshot.in_ear_detection_enabled;
    let mut current_sleep_detection = snapshot.sleep_detection_enabled;
    let mut current_dual_device = snapshot.dual_device_enabled;
    let mut current_preferred_device = snapshot.preferred_device;

    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Disconnect) => {
                let _ = updates.send(UiUpdate::Status("已断开控制通道".to_string()));
                return Ok(());
            }
            Ok(WorkerCommand::SetEqualizer(preset)) => {
                if snapshot.read_only || !snapshot.supports_equalizer {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放均衡器控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在等待耳机确认均衡器预设…".to_string()));
                match set_equalizer(&mut stream, &mut decoder, preset, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::Equalizer(actual));
                        let _ = updates.send(UiUpdate::Status("耳机已应用均衡器预设".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("切换均衡器预设失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SelectCustomEqualizer(id)) => {
                if snapshot.read_only || !snapshot.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放自定义均衡器控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在切换自定义均衡器预设…".to_string()));
                let preset = current_custom_equalizers
                    .iter()
                    .find(|p| p.id == id)
                    .cloned();
                if let Some(p) = preset {
                    let payload = encode_custom_equalizer_command(
                        0x02, p.id, &p.name, p.min_gain, p.max_gain, &p.bands,
                    );
                    match request(&mut stream, &mut decoder, 0x0418, 0, payload, Some(updates)) {
                        Ok(_) => {
                            if let Ok(res) =
                                request(&mut stream, &mut decoder, 0x0122, 0, vec![], Some(updates))
                            {
                                if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                    current_custom_equalizers = list.clone();
                                    let _ = updates.send(UiUpdate::CustomEqualizers(list));
                                }
                            }
                            let _ = updates.send(UiUpdate::Equalizer(id));
                            let _ = updates
                                .send(UiUpdate::Status("耳机已应用自定义均衡器预设".to_string()));
                        }
                        Err(error) => {
                            let _ = updates
                                .send(UiUpdate::Status(format!("切换自定义均衡器失败：{error}")));
                        }
                    }
                }
            }
            Ok(WorkerCommand::AddCustomEqualizer { name, bands }) => {
                if snapshot.read_only || !snapshot.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放自定义均衡器控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在添加自定义均衡器预设…".to_string()));
                let payload = encode_custom_equalizer_command(0x01, 0, &name, -6, 6, &bands);
                match request(&mut stream, &mut decoder, 0x0418, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0122, 0, vec![], Some(updates))
                        {
                            if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                current_custom_equalizers = list.clone();
                                let _ = updates.send(UiUpdate::CustomEqualizers(list));
                            }
                        }
                        let _ =
                            updates.send(UiUpdate::Status("已添加自定义均衡器预设".to_string()));
                    }
                    Err(error) => {
                        let _ = updates
                            .send(UiUpdate::Status(format!("添加自定义均衡器失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SaveCustomEqualizer { id, name, bands }) => {
                if snapshot.read_only || !snapshot.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放自定义均衡器控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在保存自定义均衡器预设…".to_string()));
                let (min_gain, max_gain) = current_custom_equalizers
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| (p.min_gain, p.max_gain))
                    .unwrap_or((-6, 6));
                let payload =
                    encode_custom_equalizer_command(0x02, id, &name, min_gain, max_gain, &bands);
                match request(&mut stream, &mut decoder, 0x0418, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0122, 0, vec![], Some(updates))
                        {
                            if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                current_custom_equalizers = list.clone();
                                let _ = updates.send(UiUpdate::CustomEqualizers(list));
                            }
                        }
                        let _ =
                            updates.send(UiUpdate::Status("已修改自定义均衡器预设".to_string()));
                    }
                    Err(error) => {
                        let _ = updates
                            .send(UiUpdate::Status(format!("修改自定义均衡器失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::DeleteCustomEqualizer(id)) => {
                if snapshot.read_only || !snapshot.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放自定义均衡器控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在删除自定义均衡器预设…".to_string()));
                let (name, min_gain, max_gain, bands) = current_custom_equalizers
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| (p.name.clone(), p.min_gain, p.max_gain, p.bands.clone()))
                    .unwrap_or_else(|| (String::new(), -6, 6, Vec::new()));
                let payload =
                    encode_custom_equalizer_command(0x03, id, &name, min_gain, max_gain, &bands);
                match request(&mut stream, &mut decoder, 0x0418, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0122, 0, vec![], Some(updates))
                        {
                            if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                current_custom_equalizers = list.clone();
                                let _ = updates.send(UiUpdate::CustomEqualizers(list));
                            }
                        }
                        let _ =
                            updates.send(UiUpdate::Status("已删除自定义均衡器预设".to_string()));
                    }
                    Err(error) => {
                        let _ = updates
                            .send(UiUpdate::Status(format!("删除自定义均衡器失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetSpatialAudio(mode)) => {
                if snapshot.read_only || !snapshot.supports_spatial_audio {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放空间音频控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(
                    "正在等待耳机确认空间音频模式…".to_string(),
                ));
                match set_spatial_audio(&mut stream, &mut decoder, mode, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::SpatialAudio(actual));
                        let _ =
                            updates.send(UiUpdate::Status("耳机已应用空间音频模式".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("切换空间音频失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetGameMode(enabled)) => {
                if snapshot.read_only || !snapshot.supports_game_mode {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放游戏模式控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在等待耳机确认游戏模式…".to_string()));
                match set_game_mode(
                    &mut stream,
                    &mut decoder,
                    snapshot.game_mode_feature_id,
                    enabled,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::GameMode(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            "已开启游戏模式".to_string()
                        } else {
                            "已关闭游戏模式".to_string()
                        }));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("切换游戏模式失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetBassEngineEnabled(enabled)) => {
                if snapshot.read_only || !snapshot.supports_bass_engine {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放低音增强控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(
                    "正在等待耳机确认低音增强开关…".to_string(),
                ));
                match set_bass_engine_enabled(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        snapshot.bass_engine_enabled = Some(actual);
                        let _ = updates.send(UiUpdate::BassEngineEnabled(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            "已开启低音增强".to_string()
                        } else {
                            "已关闭低音增强".to_string()
                        }));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("切换低音增强失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetBassEngineLevel(level)) => {
                if snapshot.read_only || !snapshot.supports_bass_engine {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放低音增强档位控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let (min, max) = snapshot
                    .bass_engine_level
                    .as_ref()
                    .map(|b| (b.min, b.max))
                    .unwrap_or((-5, 5));
                let _ = updates.send(UiUpdate::Status(
                    "正在等待耳机确认低音增强档位…".to_string(),
                ));
                match set_bass_engine_level(&mut stream, &mut decoder, min, max, level, updates) {
                    Ok(actual) => {
                        snapshot.bass_engine_level = Some(actual);
                        let _ = updates.send(UiUpdate::BassEngineLevel(actual));
                        let _ = updates.send(UiUpdate::Status(format!(
                            "低音增强档位已设置为 {}",
                            actual.current
                        )));
                    }
                    Err(error) => {
                        let _ = updates
                            .send(UiUpdate::Status(format!("切换低音增强档位失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetHiRes(enabled)) => {
                if snapshot.read_only || !snapshot.supports_hi_res {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放高解析模式控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在等待耳机确认高解析模式…".to_string()));
                match set_hi_res(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        snapshot.hi_res_enabled = Some(actual);
                        let _ = updates.send(UiUpdate::HiRes(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            "已开启高解析模式".to_string()
                        } else {
                            "已关闭高解析模式".to_string()
                        }));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("切换高解析模式失败：{error}")));
                    }
                }
            }
            Ok(WorkerCommand::SetNoiseMode(protocol_index)) => {
                if snapshot.read_only {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放降噪控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在等待耳机确认降噪模式…".to_string()));
                let result = set_noise_mode(
                    &mut stream,
                    &mut decoder,
                    &snapshot.noise_mode_indices,
                    snapshot.noise_modes,
                    &mut current_wear,
                    snapshot.supports_wear_detection,
                    protocol_index,
                    updates,
                );
                match result {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::NoiseMode(actual));
                        let _ = updates.send(UiUpdate::Status("降噪模式已确认并回读".to_string()));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::Status(error));
                    }
                }
                let _ = updates.send(UiUpdate::NoiseControlsEnabled(
                    !snapshot.read_only && !snapshot.noise_modes.is_empty(),
                ));
            }
            Ok(WorkerCommand::SetPairedDevice { address, action }) => {
                if snapshot.read_only || !snapshot.supports_paired_devices {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放设备管理控制，已阻止发送".to_string(),
                    ));
                    let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    continue;
                }
                let result = set_paired_device(
                    &mut stream,
                    &mut decoder,
                    &snapshot.paired_device_controls_enabled,
                    &paired_devices,
                    address,
                    action,
                    updates,
                );
                match result {
                    Ok((devices, changed)) => {
                        paired_devices = Ok(devices.clone());
                        let _ = updates.send(UiUpdate::PairedDevices(Ok(devices)));
                        let message = if changed {
                            match action {
                                PairedDeviceAction::Remove => "已从耳机中删除设备",
                                _ => "设备连接状态已确认",
                            }
                        } else {
                            "设备已处于目标连接状态"
                        };
                        let _ = updates.send(UiUpdate::Status(message.to_string()));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::Status(error));
                        paired_devices =
                            read_paired_devices(&mut stream, &mut decoder, Some(updates));
                        let _ = updates.send(UiUpdate::PairedDevices(paired_devices.clone()));
                    }
                }
                let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
            }
            Ok(WorkerCommand::SetGesture {
                side,
                button,
                action,
                function,
            }) => {
                if snapshot.read_only || !snapshot.supports_touch_customization {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放手势自定义，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在修改耳机手势…".to_string()));
                let payload = encode_set_key_function(side, button, action, function);
                match request(&mut stream, &mut decoder, 0x0401, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x0108,
                            0,
                            vec![2, 3, 1],
                            Some(updates),
                        ) {
                            if let Ok(list) = parse_touch_controls_response(&res.payload) {
                                current_key_functions = list.clone();
                                let _ = updates.send(UiUpdate::KeyFunctions(list));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("手势设置已更新".to_string()));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::Status(format!("修改手势失败：{error}")));
                        let _ = updates.send(UiUpdate::KeyFunctions(current_key_functions.clone()));
                    }
                }
            }
            Ok(WorkerCommand::SetNoiseCycle {
                side,
                mask,
                is_shared,
            }) => {
                if snapshot.read_only || snapshot.noise_modes.is_empty() {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放噪声循环控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let _ = updates.send(UiUpdate::Status("正在修改噪声切换循环…".to_string()));
                let payload = encode_set_noise_cycle(side, mask, is_shared);
                match request(&mut stream, &mut decoder, 0x0404, 0, payload, Some(updates)) {
                    Ok(_) => {
                        let cycle_type = if is_shared {
                            1
                        } else if side == EarSide::Left {
                            3
                        } else {
                            4
                        };
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x010c,
                            0,
                            vec![2, cycle_type],
                            Some(updates),
                        ) {
                            if let Ok((_, new_mask)) = parse_noise_cycle_response(&res.payload) {
                                if is_shared {
                                    current_noise_cycle.left_mask = new_mask;
                                    current_noise_cycle.right_mask = new_mask;
                                } else if side == EarSide::Left {
                                    current_noise_cycle.left_mask = new_mask;
                                } else {
                                    current_noise_cycle.right_mask = new_mask;
                                }
                                let _ = updates.send(UiUpdate::NoiseCycle(current_noise_cycle));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("噪声切换循环已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("修改噪声循环失败：{error}")));
                        let _ = updates.send(UiUpdate::NoiseCycle(current_noise_cycle));
                    }
                }
            }
            Ok(WorkerCommand::FindEarbuds(ring)) => {
                if snapshot.read_only || !snapshot.supports_find_device {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放查找耳机功能，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = encode_find_earbuds(ring);
                match request(&mut stream, &mut decoder, 0x0400, 0, payload, Some(updates)) {
                    Ok(_) => {
                        snapshot.is_finding_earbuds = ring;
                        let _ = updates.send(UiUpdate::IsFindingEarbuds(ring));
                        let _ = updates.send(UiUpdate::Status(if ring {
                            "耳机正在播放提示音".to_string()
                        } else {
                            "已停止播放提示音".to_string()
                        }));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("查找耳机命令失败：{error}")));
                        let _ =
                            updates.send(UiUpdate::IsFindingEarbuds(snapshot.is_finding_earbuds));
                    }
                }
            }
            Ok(WorkerCommand::RunFitTest) => {
                if snapshot.read_only || !snapshot.supports_fit_detection {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放耳塞贴合度检测，已阻止发送".to_string(),
                    ));
                    continue;
                }
                if current_wear.left.is_some_and(|p| !p.in_ear)
                    || current_wear.right.is_some_and(|p| !p.in_ear)
                {
                    let _ = updates.send(UiUpdate::Status(
                        "请佩戴好双耳后再进行贴合度测试".to_string(),
                    ));
                    continue;
                }
                let payload = encode_fit_test(true);
                match request(&mut stream, &mut decoder, 0x0405, 0, payload, Some(updates)) {
                    Ok(_) => {
                        let _ = updates.send(UiUpdate::FitTestRunning(true));
                        let _ =
                            updates.send(UiUpdate::Status("正在进行耳塞贴合度检测…".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("启动贴合度检测失败：{error}")));
                        let _ = updates.send(UiUpdate::FitTestRunning(false));
                    }
                }
            }
            Ok(WorkerCommand::SetPromptVolume(volume)) => {
                if snapshot.read_only || !snapshot.supports_prompt_volume {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放提示音音量控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = encode_set_prompt_volume(volume);
                match request(&mut stream, &mut decoder, 0x0427, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0130, 0, vec![], Some(updates))
                        {
                            if let Ok(actual) = parse_prompt_volume_response(&res.payload) {
                                current_prompt_volume = Some(actual);
                                let _ = updates.send(UiUpdate::PromptVolume(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("提示音音量已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("调节提示音音量失败：{error}")));
                        if let Some(v) = current_prompt_volume {
                            let _ = updates.send(UiUpdate::PromptVolume(v));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetHeadMotionType(motion_type)) => {
                if snapshot.read_only || !snapshot.supports_head_motion {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放头部动作控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = encode_set_head_motion_type(motion_type);
                match request(&mut stream, &mut decoder, 0x0431, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0134, 0, vec![], Some(updates))
                        {
                            if let Ok(actual) = parse_head_motion_type_response(&res.payload) {
                                current_head_motion = Some(actual);
                                let _ = updates.send(UiUpdate::HeadMotionType(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("头部控制动作已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("修改头部动作失败：{error}")));
                        if let Some(m) = current_head_motion {
                            let _ = updates.send(UiUpdate::HeadMotionType(m));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetDualDeviceConnection(enabled)) => {
                if snapshot.read_only || !snapshot.supports_paired_devices {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放双设备连接控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = vec![17, if enabled { 1 } else { 0 }];
                match request(&mut stream, &mut decoder, 0x0403, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x010d,
                            0,
                            vec![1, 17],
                            Some(updates),
                        ) {
                            if let Ok(actual) = parse_feature_flags_response(&res.payload, 17) {
                                current_dual_device = Some(actual);
                                let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("双设备连接设置已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("设置双设备连接失败：{error}")));
                        if let Some(d) = current_dual_device {
                            let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(d));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetInEarDetection(enabled)) => {
                if snapshot.read_only || !snapshot.supports_wear_detection {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放佩戴检测控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = vec![0x04, if enabled { 1 } else { 0 }];
                match request(&mut stream, &mut decoder, 0x0403, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x010d,
                            0,
                            vec![1, 0x04],
                            Some(updates),
                        ) {
                            if let Ok(actual) = parse_feature_flags_response(&res.payload, 0x04) {
                                current_in_ear_detection = Some(actual);
                                let _ = updates.send(UiUpdate::InEarDetectionEnabled(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("佩戴检测设置已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("设置佩戴检测失败：{error}")));
                        if let Some(w) = current_in_ear_detection {
                            let _ = updates.send(UiUpdate::InEarDetectionEnabled(w));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetSleepDetection(enabled)) => {
                if snapshot.read_only || !snapshot.supports_sleep_detection {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放入睡检测控制，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = vec![0x3a, if enabled { 1 } else { 0 }];
                match request(&mut stream, &mut decoder, 0x0403, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x010d,
                            0,
                            vec![1, 0x3a],
                            Some(updates),
                        ) {
                            if let Ok(actual) = parse_feature_flags_response(&res.payload, 0x3a) {
                                current_sleep_detection = Some(actual);
                                let _ = updates.send(UiUpdate::SleepDetectionEnabled(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("入睡检测设置已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("设置入睡检测失败：{error}")));
                        if let Some(s) = current_sleep_detection {
                            let _ = updates.send(UiUpdate::SleepDetectionEnabled(s));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetPreferredDevice(addr)) => {
                if snapshot.read_only || !snapshot.supports_paired_devices {
                    let _ = updates.send(UiUpdate::Status(
                        "该型号未开放首选设备设置，已阻止发送".to_string(),
                    ));
                    continue;
                }
                let payload = encode_set_priority_device(addr);
                match request(&mut stream, &mut decoder, 0x0429, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) =
                            request(&mut stream, &mut decoder, 0x0132, 0, vec![2], Some(updates))
                        {
                            if let Ok(actual) = parse_priority_device_response(&res.payload) {
                                current_preferred_device = actual;
                                let _ = updates.send(UiUpdate::PreferredDevice(actual));
                            }
                        }
                        let _ = updates.send(UiUpdate::Status("首选连接设备已更新".to_string()));
                    }
                    Err(error) => {
                        let _ =
                            updates.send(UiUpdate::Status(format!("设置首选设备失败：{error}")));
                        let _ = updates.send(UiUpdate::PreferredDevice(current_preferred_device));
                    }
                }
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        let mut bytes = [0; 1024];
        match stream.read(&mut bytes) {
            Err(error) if is_socket_timeout(&error) => continue,
            Err(error) => return Err(format!("RFCOMM 连接中断：{error}")),
            Ok(0) => return Err("耳机关闭了 RFCOMM 控制连接".to_string()),
            Ok(length) => {
                let packets = decoder
                    .push(&bytes[..length])
                    .map_err(|error| format!("无法解析耳机数据帧：{error}"))?;
                for bytes in packets {
                    if let Ok(packet) = AppPacket::decode(&bytes) {
                        if snapshot.read_only {
                            if snapshot.supports_paired_devices
                                && packet.command == 0x0204
                                && matches!(packet.payload.first(), Some(1 | 2))
                            {
                                publish_unsolicited(&packet, updates);
                            }
                            continue;
                        }
                        if packet.command == 0x0204
                            && packet.payload.first() == Some(&6)
                            && let Ok(devices) = parse_paired_devices_event(&packet.payload)
                        {
                            paired_devices = Ok(devices);
                        }
                        if packet.command == 0x0204
                            && packet.payload.first() == Some(&2)
                            && let Ok(update) = parse_wear_event(&packet.payload)
                        {
                            merge_wear_state(&mut current_wear, update);
                        }
                        publish_unsolicited(&packet, updates);
                    }
                }
            }
        }
    }
}

pub fn initialize_oppo_session<T: Read + Write>(
    stream: &mut T,
    name: &str,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(OppoDeviceSnapshot, FrameDecoder), String> {
    let mut decoder = FrameDecoder::default();

    let handshake = request(stream, &mut decoder, 0x0100, 0, vec![], updates)?;
    if handshake.payload.len() < 2 {
        return Err("耳机握手响应缺少能力信息".to_string());
    }
    let capabilities = handshake.payload[1..].to_vec();

    let product = request(stream, &mut decoder, 0x0103, 0, vec![], updates)?;
    if product.payload.len() < 4 {
        return Err("耳机产品 ID 响应格式不完整".to_string());
    }
    let product_id = u32::from_le_bytes([
        product.payload[1],
        product.payload[2],
        product.payload[3],
        0,
    ]);
    let color_id = match request(stream, &mut decoder, 0x010b, 0, vec![], updates) {
        Ok(packet) => parse_color_response(&packet.payload).ok(),
        Err(_) => None,
    };
    let profile = compatibility::profile_for_product_id(product_id);
    let read_only = profile.is_none_or(|profile| !profile.supports_controls());
    if read_only {
        let model = profile.map_or("未知型号", |profile| profile.model);
        let (battery, firmware, wear, detected_features, probe_errors) = if profile.is_some() {
            probe_read_only_features(stream, &mut decoder, updates)
        } else {
            (
                BatteryState::default(),
                FirmwareVersions::default(),
                WearState::default(),
                Vec::new(),
                Vec::new(),
            )
        };
        let note = if profile.is_none() {
            "未知型号仅验证控制通道握手和产品识别".to_string()
        } else if detected_features.is_empty() {
            format!("{model} 未能确认可用的只读状态查询")
        } else {
            format!("{model} 已检测只读功能：{}", detected_features.join("、"))
        };
        return Ok((
            DeviceSnapshot {
                name: name.to_string(),
                product_id,
                color_id,
                model,
                read_only: true,
                battery,
                firmware,
                wear,
                supports_wear_detection: profile
                    .is_some_and(|profile| profile.supports_wear_detection),
                noise_mode: None,
                noise_mode_indices: Vec::new(),
                noise_mode_support_note: format!("{note}；型号配置未声明可用设置控制"),
                noise_modes: profile.map_or(&[], |profile| profile.noise_modes),
                supports_paired_devices: profile
                    .is_some_and(|profile| profile.supports_dual_device),
                detected_features,
                probe_errors,
                paired_devices: Err(note.clone()),
                paired_device_controls_enabled: Err(note),
                supports_equalizer: profile.is_some_and(|profile| profile.supports_equalizer),
                supports_custom_equalizer: profile
                    .is_some_and(|profile| profile.supports_custom_equalizer),
                custom_equalizers: Vec::new(),
                equalizer_preset: None,
                supports_spatial_audio: profile
                    .is_some_and(|profile| profile.supports_spatial_audio),
                supports_head_tracking: profile
                    .is_some_and(|profile| profile.supports_head_tracking),
                spatial_audio_mode: None,
                supports_game_mode: profile.is_some_and(|profile| profile.supports_game_mode),
                game_mode_enabled: None,
                game_mode_feature_id: 0x06,
                supports_bass_engine: profile.is_some_and(|profile| profile.supports_bass_engine),
                bass_engine_enabled: None,
                bass_engine_level: None,
                supports_hi_res: profile.is_some_and(|profile| profile.supports_hi_res),
                hi_res_enabled: None,
                supports_touch_customization: profile
                    .is_some_and(|profile| profile.supports_touch_customization),
                key_functions: Vec::new(),
                noise_cycle: NoiseCycleState::default(),
                is_finding_earbuds: false,
                supports_prompt_volume: profile
                    .is_some_and(|profile| profile.supports_prompt_volume),
                prompt_volume: None,
                supports_head_motion: profile.is_some_and(|profile| profile.supports_head_motion),
                head_motion_type: None,
                supports_fit_detection: profile
                    .is_some_and(|profile| profile.supports_fit_detection),
                supports_find_device: profile.is_some_and(|profile| profile.supports_find_device),
                supports_sleep_detection: profile
                    .is_some_and(|profile| profile.supports_sleep_detection),
                in_ear_detection_enabled: None,
                sleep_detection_enabled: None,
                dual_device_enabled: None,
                preferred_device: None,
                capabilities,
            },
            decoder,
        ));
    }
    let profile = profile.expect("a non-read-only profile must be present");

    let events = request(stream, &mut decoder, 0x0200, 0, vec![], updates)?;
    let event_ids = events
        .payload
        .get(2..)
        .ok_or_else(|| "耳机事件列表响应格式不完整".to_string())?;
    let event_count = events.payload[1] as usize;
    if event_ids.len() < event_count {
        return Err("耳机事件列表长度不匹配".to_string());
    }
    let supported_events: Vec<u8> = [1, 2, 3, 4, 6]
        .into_iter()
        .filter(|event| {
            event_ids[..event_count].contains(event)
                && (*event != 2 || profile.supports_wear_detection)
                && (*event != 3 || !profile.noise_modes.is_empty())
                && (*event != 4 || profile.supports_fit_detection)
                && (*event != 6 || profile.supports_dual_device)
        })
        .collect();

    if !supported_events.is_empty() && capability_bitmap_allows(&capabilities, 0x0205) {
        let mut subscription = vec![supported_events.len() as u8];
        subscription.extend_from_slice(&supported_events);
        let _ = request(stream, &mut decoder, 0x0205, 0, subscription, updates);
    }

    let battery = request(stream, &mut decoder, 0x0106, 0xf0, vec![], updates)?;
    let battery = parse_battery_response(&battery.payload)
        .map_err(|error| format!("电量响应格式错误：{error}"))?;

    let firmware = request(stream, &mut decoder, 0x0105, 0, vec![], updates)?;
    let firmware = parse_firmware_response(&firmware.payload)
        .map_err(|error| format!("固件版本响应格式错误：{error}"))?;

    let wear = if profile.supports_wear_detection {
        let wear = request(stream, &mut decoder, 0x0109, 0, vec![], updates)?;
        parse_wear_response(&wear.payload)
            .map_err(|error| format!("佩戴状态响应格式错误：{error}"))?
    } else {
        WearState::default()
    };

    let mut noise_mode_indices = if profile.noise_modes.is_empty() {
        Vec::new()
    } else {
        compatibility::fallback_noise_mode_indices_for(profile.noise_modes)
    };
    let noise_mode_support_note = if profile.noise_modes.is_empty() {
        crate::t!("noise-mode-profile-missing")
    } else {
        crate::t!("noise-mode-profile-declares-available")
    };

    let noise_mode = if !profile.noise_modes.is_empty() {
        let noise = request(stream, &mut decoder, 0x010c, 0, vec![1, 1], updates)?;
        parse_noise_mode_response(&noise.payload)
    } else {
        None
    };
    if let Some(index) = noise_mode {
        if !noise_mode_indices.contains(&index) {
            noise_mode_indices.push(index);
        }
    }

    let paired_device_controls_enabled = if !profile.supports_dual_device {
        Err(crate::t!("dual-device-profile-missing"))
    } else {
        match request(stream, &mut decoder, 0x010d, 0, vec![1, 17], updates) {
            Ok(response) => parse_feature_flags_response(&response.payload, 17)
                .map_err(|error| format!("双设备连接状态响应格式错误：{error}")),
            Err(_) => Ok(true),
        }
    };

    let supports_paired_devices = profile.supports_dual_device;
    let paired_devices = if supports_paired_devices {
        read_paired_devices(stream, &mut decoder, updates)
    } else {
        Err(crate::t!("dual-device-profile-missing"))
    };

    let supports_equalizer = profile.supports_equalizer;
    let equalizer_preset = if supports_equalizer {
        match request(stream, &mut decoder, 0x010f, 0, vec![], updates) {
            Ok(response) => parse_equalizer_response(&response.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_custom_equalizer = profile.supports_custom_equalizer;
    let custom_equalizers = if supports_custom_equalizer {
        match request(stream, &mut decoder, 0x0122, 0, vec![], updates) {
            Ok(response) => {
                parse_custom_equalizer_list(&response.payload, true).unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let supports_spatial_audio = profile.supports_spatial_audio;
    let supports_head_tracking = profile.supports_head_tracking;
    let spatial_audio_mode = if supports_spatial_audio {
        match request(stream, &mut decoder, 0x012a, 0, vec![], updates) {
            Ok(response) => parse_spatial_audio_response(&response.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_game_mode = profile.supports_game_mode;
    let game_mode_feature_id = compatibility::game_mode_feature_id_for_product(product_id);
    let game_mode_enabled = if supports_game_mode {
        match request(
            stream,
            &mut decoder,
            0x010d,
            0,
            vec![1, game_mode_feature_id],
            updates,
        ) {
            Ok(response) => {
                parse_feature_flags_response(&response.payload, game_mode_feature_id).ok()
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_bass_engine = profile.supports_bass_engine;
    let (bass_engine_enabled, bass_engine_level) = if supports_bass_engine {
        let enabled = match request(
            stream,
            &mut decoder,
            0x010d,
            0,
            vec![1, FEATURE_BASS_ENGINE],
            updates,
        ) {
            Ok(response) => {
                parse_feature_flags_response(&response.payload, FEATURE_BASS_ENGINE).ok()
            }
            Err(_) => None,
        };
        let level = match request(stream, &mut decoder, 0x0124, 0, vec![], updates) {
            Ok(response) => parse_bass_engine_response(&response.payload).ok(),
            Err(_) => None,
        };
        (enabled, level)
    } else {
        (None, None)
    };

    let supports_hi_res = profile.supports_hi_res;
    let hi_res_enabled = if supports_hi_res {
        match request(
            stream,
            &mut decoder,
            0x010d,
            0,
            vec![1, FEATURE_HI_RES],
            updates,
        ) {
            Ok(response) => parse_feature_flags_response(&response.payload, FEATURE_HI_RES).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_touch_customization = profile.supports_touch_customization;
    let key_functions = if supports_touch_customization {
        match request(stream, &mut decoder, 0x0108, 0, vec![2, 3, 1], updates) {
            Ok(res) => parse_touch_controls_response(&res.payload).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let noise_cycle = if !profile.noise_modes.is_empty() {
        match request(stream, &mut decoder, 0x010c, 0, vec![2, 1], updates) {
            Ok(res) => {
                let (_, mask) = parse_noise_cycle_response(&res.payload).unwrap_or((1, 0));
                NoiseCycleState {
                    left_mask: mask,
                    right_mask: mask,
                }
            }
            Err(_) => NoiseCycleState::default(),
        }
    } else {
        NoiseCycleState::default()
    };

    let supports_prompt_volume = profile.supports_prompt_volume;
    let prompt_volume = if supports_prompt_volume {
        match request(stream, &mut decoder, 0x0130, 0, vec![], updates) {
            Ok(res) => parse_prompt_volume_response(&res.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_head_motion = profile.supports_head_motion;
    let head_motion_type = if supports_head_motion {
        match request(stream, &mut decoder, 0x0134, 0, vec![], updates) {
            Ok(res) => parse_head_motion_type_response(&res.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let preferred_device = if profile.supports_dual_device {
        match request(stream, &mut decoder, 0x0132, 0, vec![2], updates) {
            Ok(res) => parse_priority_device_response(&res.payload).ok().flatten(),
            Err(_) => None,
        }
    } else {
        None
    };

    let in_ear_detection_enabled = if profile.supports_wear_detection {
        match request(stream, &mut decoder, 0x010d, 0, vec![1, 0x04], updates) {
            Ok(res) => parse_feature_flags_response(&res.payload, 0x04).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let sleep_detection_enabled = if profile.supports_sleep_detection {
        match request(stream, &mut decoder, 0x010d, 0, vec![1, 0x3a], updates) {
            Ok(res) => parse_feature_flags_response(&res.payload, 0x3a).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let dual_device_enabled = paired_device_controls_enabled.as_ref().ok().copied();

    Ok((
        DeviceSnapshot {
            name: name.to_string(),
            product_id,
            color_id,
            model: profile.model,
            read_only: false,
            battery,
            firmware,
            wear,
            supports_wear_detection: profile.supports_wear_detection,
            noise_mode,
            noise_mode_indices,
            noise_mode_support_note,
            noise_modes: profile.noise_modes,
            supports_paired_devices,
            detected_features: Vec::new(),
            probe_errors: Vec::new(),
            paired_devices,
            paired_device_controls_enabled,
            supports_equalizer,
            supports_custom_equalizer,
            custom_equalizers,
            equalizer_preset,
            supports_spatial_audio,
            supports_head_tracking,
            spatial_audio_mode,
            supports_game_mode,
            game_mode_enabled,
            game_mode_feature_id,
            supports_bass_engine,
            bass_engine_enabled,
            bass_engine_level,
            supports_hi_res,
            hi_res_enabled,
            supports_touch_customization,
            key_functions,
            noise_cycle,
            is_finding_earbuds: false,
            supports_prompt_volume,
            prompt_volume,
            supports_head_motion,
            head_motion_type,
            supports_fit_detection: profile.supports_fit_detection,
            supports_find_device: profile.supports_find_device,
            supports_sleep_detection: profile.supports_sleep_detection,
            in_ear_detection_enabled,
            sleep_detection_enabled,
            dual_device_enabled,
            preferred_device,
            capabilities,
        },
        decoder,
    ))
}

pub use initialize_oppo_session as initialize_session;

pub fn probe_read_only_features<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> (
    BatteryState,
    FirmwareVersions,
    WearState,
    Vec<String>,
    Vec<String>,
) {
    let mut battery = BatteryState::default();
    let mut firmware = FirmwareVersions::default();
    let mut wear = WearState::default();
    let mut detected_features = Vec::new();
    let mut probe_errors = Vec::new();

    let supported_events = match request(stream, decoder, 0x0200, 0, vec![], updates) {
        Ok(response) => match parse_supported_events(&response.payload) {
            Ok(events) => {
                let event_summary = if events.is_empty() {
                    "未声明事件".to_string()
                } else {
                    events
                        .iter()
                        .map(|event| format!("{event:02X}"))
                        .collect::<Vec<_>>()
                        .join("、")
                };
                detected_features.push(format!("事件列表读取（{event_summary}）"));
                Some(events)
            }
            Err(error) => {
                probe_errors.push(format!("事件能力列表格式错误：{error}"));
                None
            }
        },
        Err(error) => {
            probe_errors.push(format!("事件能力查询失败：{error}"));
            None
        }
    };

    if let Some(events) = supported_events {
        let subscriptions: Vec<u8> = [1, 2]
            .into_iter()
            .filter(|event| events.contains(event))
            .collect();
        if !subscriptions.is_empty() {
            let mut payload = vec![subscriptions.len() as u8];
            payload.extend_from_slice(&subscriptions);
            match request(stream, decoder, 0x0205, 0, payload, updates) {
                Ok(response) if response.payload.first() == Some(&0) => {
                    detected_features.push("电量/佩戴状态变化通知".to_string())
                }
                Ok(response) => probe_errors.push(format!(
                    "状态变化订阅被拒绝，状态码：{}",
                    response
                        .payload
                        .first()
                        .map_or_else(|| "缺失".to_string(), |status| format!("0x{status:02X}"))
                )),
                Err(error) => probe_errors.push(format!("状态变化订阅失败：{error}")),
            }
        }
    }

    match request(stream, decoder, 0x0106, 0xf0, vec![], updates) {
        Ok(response) => match parse_battery_response(&response.payload) {
            Ok(state) => {
                if state.left.is_some() || state.right.is_some() || state.case.is_some() {
                    detected_features.push("电量读取".to_string());
                    battery = state;
                } else {
                    probe_errors.push("电量查询未返回电池数据".to_string());
                }
            }
            Err(error) => probe_errors.push(format!("电量响应格式错误：{error}")),
        },
        Err(error) => probe_errors.push(format!("电量查询失败：{error}")),
    }

    match request(stream, decoder, 0x0105, 0, vec![], updates) {
        Ok(response) => match parse_firmware_response(&response.payload) {
            Ok(state) => {
                detected_features.push("固件版本读取".to_string());
                firmware = state;
            }
            Err(error) => probe_errors.push(format!("固件响应格式错误：{error}")),
        },
        Err(error) => probe_errors.push(format!("固件版本查询失败：{error}")),
    }

    match request(stream, decoder, 0x0109, 0, vec![], updates) {
        Ok(response) => match parse_wear_response(&response.payload) {
            Ok(state) => {
                if state.left.is_some() || state.right.is_some() || state.case_open.is_some() {
                    detected_features.push("佩戴/盒盖状态读取".to_string());
                    wear = state;
                } else {
                    probe_errors.push("佩戴状态查询未返回状态数据".to_string());
                }
            }
            Err(error) => probe_errors.push(format!("佩戴状态响应格式错误：{error}")),
        },
        Err(error) => probe_errors.push(format!("佩戴状态查询失败：{error}")),
    }

    (battery, firmware, wear, detected_features, probe_errors)
}

fn parse_supported_events(payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err("响应缺少成功状态或事件数量".to_string());
    }
    let count = payload[1] as usize;
    let events = payload
        .get(2..)
        .filter(|events| events.len() == count)
        .ok_or_else(|| "事件数量与响应长度不匹配".to_string())?;
    Ok(events.to_vec())
}

pub fn set_noise_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    supported_indices: &[u8],
    modes: &[NoiseMode],
    current_wear: &mut WearState,
    supports_wear_detection: bool,
    protocol_index: u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    if supports_wear_detection && !current_wear.any_earbud_in_ear() {
        return Err("未检测到耳机入耳；请佩戴耳机后再切换降噪模式".to_string());
    }
    let (deferred_tx, deferred_rx) = mpsc::channel();
    let result = (|| {
        if !compatibility::noise_mode_supported_for(protocol_index, supported_indices, modes) {
            return Err(format!(
                "耳机支持模式清单中没有降噪模式索引 {protocol_index}"
            ));
        }
        let payload = noise_mode_write_payload_for(protocol_index, modes)
            .ok_or_else(|| format!("该型号兼容配置未开放降噪模式索引 {protocol_index}"))?;
        request(stream, decoder, 0x0404, 0, payload, Some(&deferred_tx))?;

        let result = request(stream, decoder, 0x010c, 0, vec![1, 1], Some(&deferred_tx))?;
        let actual = parse_noise_mode_response(&result.payload)
            .ok_or_else(|| "降噪设置已确认，但耳机回读的模式格式无法识别".to_string())?;
        if !noise_mode_readback_matches_for(protocol_index, actual, modes) {
            return Err(format!(
                "耳机已确认设置，但回读模式为索引 {actual}，与请求的模式不一致；界面保留原状态"
            ));
        }
        Ok(actual)
    })();
    for update in deferred_rx.try_iter() {
        match update {
            UiUpdate::Wear(ref wear) => merge_wear_state(current_wear, wear.clone()),
            UiUpdate::NoiseMode(_) => continue,
            _ => {}
        }
        let _ = updates.send(update);
    }
    result
}

pub fn set_paired_device<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    controls_enabled: &Result<bool, String>,
    paired_devices: &Result<Vec<PairedDevice>, String>,
    address: PairedDeviceAddress,
    action: PairedDeviceAction,
    updates: &Sender<UiUpdate>,
) -> Result<(Vec<PairedDevice>, bool), String> {
    match controls_enabled {
        Ok(true) => {}
        Ok(false) => return Err("耳机未启用双设备连接，无法管理已配对设备".to_string()),
        Err(error) => return Err(format!("无法确认双设备连接状态，操作已禁用：{error}")),
    }

    let paired_devices = paired_devices
        .as_ref()
        .map_err(|error| format!("无法读取已配对设备列表：{error}"))?;
    let target = paired_devices
        .iter()
        .find(|device| device.address == address)
        .ok_or_else(|| "该设备已不在耳机上报的列表中，请刷新后重试".to_string())?;
    if target.is_current_host {
        let op = match action {
            PairedDeviceAction::Remove => "删除",
            _ => "断开",
        };
        return Err(format!("不能通过此列表{op}本机"));
    }
    match action {
        PairedDeviceAction::Connect | PairedDeviceAction::Disconnect => {
            let expected = action
                .expected_connected()
                .expect("Connect/Disconnect must have expected state");
            if target.connected == expected {
                return Ok((paired_devices.clone(), false));
            }

            let updated = issue_paired_device_action(stream, decoder, address, action, updates)?;
            let updated_target = updated
                .iter()
                .find(|device| device.address == address)
                .ok_or_else(|| "耳机确认了操作，但设备列表回读中未找到目标设备".to_string())?;
            if updated_target.connected != expected {
                return Err("耳机已确认操作，但设备列表回读状态不匹配".to_string());
            }
            Ok((updated, true))
        }
        PairedDeviceAction::Remove => {
            if target.connected {
                return Err("不能删除已连接的设备，请先断开连接后再删除".to_string());
            }

            let updated = issue_paired_device_action(stream, decoder, address, action, updates)?;
            if updated.iter().any(|device| device.address == address) {
                return Err("耳机已确认删除操作，但设备列表回读中仍包含该设备".to_string());
            }
            Ok((updated, true))
        }
    }
}

fn issue_paired_device_action<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    address: PairedDeviceAddress,
    action: PairedDeviceAction,
    updates: &Sender<UiUpdate>,
) -> Result<Vec<PairedDevice>, String> {
    request(
        stream,
        decoder,
        0x0429,
        0,
        action.payload(address),
        Some(updates),
    )?;
    read_paired_devices(stream, decoder, Some(updates))
        .map_err(|error| format!("设备操作后的列表回读失败：{error}"))
}

fn read_paired_devices<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<PairedDevice>, String> {
    let response = request(stream, decoder, 0x0112, 0, vec![], updates)?;
    parse_paired_devices_response(&response.payload)
        .map_err(|error| format!("耳机已配对设备列表格式错误：{error}"))
}

pub fn set_equalizer<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    preset: u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    request(stream, decoder, 0x0406, 0, vec![preset], Some(updates))?;
    let result = request(stream, decoder, 0x010f, 0, vec![], Some(updates))?;
    let actual = parse_equalizer_response(&result.payload)
        .map_err(|error| format!("耳机均衡器回读格式错误：{error}"))?;
    if actual != preset {
        return Err(format!(
            "耳机已确认设置，但回读预设为 {actual}，与请求的预设 {preset} 不一致；界面保留原状态"
        ));
    }
    Ok(actual)
}

pub fn set_spatial_audio<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    mode: u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    request(stream, decoder, 0x0422, 0, vec![mode], Some(updates))?;
    let result = request(stream, decoder, 0x012a, 0, vec![], Some(updates))?;
    let actual = parse_spatial_audio_response(&result.payload)
        .map_err(|error| format!("耳机空间音频回读格式错误：{error}"))?;
    if actual != mode {
        return Err(format!(
            "耳机已确认设置，但回读空间音频模式为 {actual}，与请求的模式 {mode} 不一致；界面保留原状态"
        ));
    }
    Ok(actual)
}

pub fn set_game_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    feature_id: u8,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    set_feature_toggle(stream, decoder, feature_id, enabled, "游戏模式", updates)
}

pub fn set_bass_engine_enabled<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    set_feature_toggle(
        stream,
        decoder,
        FEATURE_BASS_ENGINE,
        enabled,
        "低音增强",
        updates,
    )
}

pub fn set_bass_engine_level<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    min: i8,
    max: i8,
    level: i8,
    updates: &Sender<UiUpdate>,
) -> Result<BassEngineLevel, String> {
    let clamped = level.clamp(min, max);
    request(
        stream,
        decoder,
        0x041b,
        0,
        vec![min as u8, max as u8, clamped as u8],
        Some(updates),
    )?;
    let result = request(stream, decoder, 0x0124, 0, vec![], Some(updates))?;
    let actual = parse_bass_engine_response(&result.payload)
        .map_err(|error| format!("低音增强回读格式错误：{error}"))?;
    if actual.current != clamped {
        return Err(format!(
            "耳机已确认设置，但回读低音增强档位为 {}，与请求的档位 {} 不一致；界面保留原状态",
            actual.current, clamped
        ));
    }
    Ok(actual)
}

pub fn set_hi_res<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    set_feature_toggle(
        stream,
        decoder,
        FEATURE_HI_RES,
        enabled,
        "高解析模式",
        updates,
    )
}

fn set_feature_toggle<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    feature_id: u8,
    enabled: bool,
    feature_name: &str,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    let value = if enabled { 1 } else { 0 };
    request(
        stream,
        decoder,
        0x0403,
        0,
        vec![feature_id, value],
        Some(updates),
    )?;
    let result = request(
        stream,
        decoder,
        0x010d,
        0,
        vec![1, feature_id],
        Some(updates),
    )?;
    let actual = parse_feature_flags_response(&result.payload, feature_id)
        .map_err(|error| format!("{feature_name}回读格式错误：{error}"))?;
    if actual != enabled {
        return Err(format!(
            "耳机已确认设置，但回读{feature_name}状态为 {actual}，与请求的状态 {enabled} 不一致；界面保留原状态"
        ));
    }
    Ok(actual)
}

fn request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    command: u16,
    sequence: u8,
    payload: Vec<u8>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<AppPacket, String> {
    let packet = AppPacket {
        command,
        sequence,
        payload,
    };
    let frame = encode_frame(
        &packet
            .encode()
            .map_err(|error| format!("无法编码 OPPO 命令：{error}"))?,
    )
    .map_err(|error| format!("无法编码 OPPO 帧：{error}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("发送 OPPO 查询失败：{error}"))?;

    let deadline = Instant::now() + REQUEST_TIMEOUT;
    loop {
        let mut bytes = [0; 1024];
        let length = match stream.read(&mut bytes) {
            Ok(0) => return Err("耳机关闭了 RFCOMM 控制连接".to_string()),
            Ok(length) => length,
            Err(error) if is_socket_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(format!("等待耳机查询 0x{command:04X} 响应超时"));
                }
                continue;
            }
            Err(error) => return Err(format!("读取耳机响应失败：{error}")),
        };
        let frames = decoder
            .push(&bytes[..length])
            .map_err(|error| format!("无法解析耳机响应帧：{error}"))?;
        let mut response = None;
        let mut trailing_packets = Vec::new();
        for (index, frame) in frames.iter().enumerate() {
            let Ok(received) = AppPacket::decode(&frame) else {
                continue;
            };
            if received.command == (command | 0x8000) && received.sequence == sequence {
                response = Some(received);
                trailing_packets.extend(frames.iter().skip(index + 1).cloned());
                break;
            }
            if let Some(updates) = updates {
                publish_unsolicited(&received, updates);
            }
        }
        if let Some(response) = response {
            for bytes in trailing_packets {
                if let (Some(updates), Ok(packet)) = (updates, AppPacket::decode(&bytes)) {
                    publish_unsolicited(&packet, updates);
                }
            }
            if response.payload.first() != Some(&0) {
                return Err(format!(
                    "耳机拒绝命令 0x{command:04X}，状态码 {}",
                    response
                        .payload
                        .first()
                        .map(|status| format!("0x{status:02X}"))
                        .unwrap_or_else(|| "缺失".to_string())
                ));
            }
            return Ok(response);
        }
        if Instant::now() >= deadline {
            return Err(format!("等待耳机查询 0x{command:04X} 响应超时"));
        }
    }
}

fn publish_unsolicited(packet: &AppPacket, updates: &Sender<UiUpdate>) {
    if packet.command == 0x0506 {
        if let Ok(list) = parse_custom_equalizer_list(&packet.payload, false) {
            let _ = updates.send(UiUpdate::CustomEqualizers(list));
        }
        return;
    }
    if packet.command == 0x0504 {
        if let Ok(preset) = parse_equalizer_event(&packet.payload) {
            let _ = updates.send(UiUpdate::Equalizer(preset));
        }
        return;
    }
    if packet.command == 0x0508 {
        if let Ok(level) = parse_bass_engine_event(&packet.payload) {
            let _ = updates.send(UiUpdate::BassEngineLevel(level));
        }
        return;
    }
    if packet.command == 0x0503 {
        let pairs: Vec<(u8, u8)> = if packet.payload.len() >= 3
            && packet.payload.len() == 1 + (packet.payload[0] as usize) * 2
        {
            packet.payload[1..]
                .chunks_exact(2)
                .map(|c| (c[0], c[1]))
                .collect()
        } else if packet.payload.len() >= 2 {
            vec![(packet.payload[0], packet.payload[1])]
        } else {
            Vec::new()
        };
        for (feature_id, val) in pairs {
            if feature_id == 0x28 || feature_id == 0x06 {
                let _ = updates.send(UiUpdate::GameMode(val != 0));
            } else if feature_id == FEATURE_BASS_ENGINE {
                let _ = updates.send(UiUpdate::BassEngineEnabled(val != 0));
            } else if feature_id == FEATURE_HI_RES {
                let _ = updates.send(UiUpdate::HiRes(val != 0));
            }
        }
        return;
    }
    if (packet.command == 0x012a || packet.command == 0x812a) && !packet.payload.is_empty() {
        if let Ok(mode) = parse_spatial_audio_response(&packet.payload) {
            let _ = updates.send(UiUpdate::SpatialAudio(mode));
        }
        return;
    }
    if packet.command != 0x0204 {
        return;
    }
    match packet.payload.first() {
        Some(1) => {
            if let Ok(state) = parse_battery_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(state));
            }
        }
        Some(2) => {
            if let Ok(state) = parse_wear_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Wear(state));
            }
        }
        Some(3) => {
            if let Some(mode) = parse_noise_mode_event(&packet.payload) {
                let _ = updates.send(UiUpdate::NoiseMode(mode));
            }
        }
        Some(4) => {
            if let Ok(result) = parse_fit_test_event(&packet.payload) {
                let _ = updates.send(UiUpdate::FitTestRunning(false));
                let _ = updates.send(UiUpdate::FitTestResult(result));
            }
        }
        Some(6) => {
            let result = parse_paired_devices_event(&packet.payload)
                .map_err(|error| format!("耳机已配对设备事件格式错误：{error}"));
            let _ = updates.send(UiUpdate::PairedDevices(result));
        }
        _ => {}
    }
}

fn merge_wear_state(current: &mut WearState, update: WearState) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case_open.is_some() {
        current.case_open = update.case_open;
    }
}
