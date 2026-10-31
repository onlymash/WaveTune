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
    sync::mpsc::Sender,
};

use crate::app::events::UiUpdate;
use crate::bluetooth::driver::HeadsetDriver;
use crate::bluetooth::{
    ControlsSnapshot, DeviceChoice, DeviceIdentitySnapshot, DeviceSnapshot, DualDeviceSnapshot,
    NoiseSnapshot, ProbeDiagnostics, SoundSnapshot, WearSnapshot,
};
use crate::compatibility;
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunFrameDecoder, parse_earfun_ambient_mode, parse_earfun_anc_sub_mode,
    parse_earfun_audio_codec, parse_earfun_battery_component, parse_earfun_dual_device,
    parse_earfun_firmware_version, parse_earfun_game_mode, parse_earfun_in_ear_detection,
    parse_earfun_transparency_sub_mode,
};
use crate::protocol::{BatteryState, FirmwareVersions, WearPart, WearState};

use super::driver::EarFunDriver;
use super::features::{
    dual_device::read_earfun_paired_devices, noise::earfun_mode_to_protocol_index,
};
use super::transport::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, EARFUN_FREE_PRO_3_PRODUCT_ID, request,
};

pub type EarFunDeviceSnapshot = DeviceSnapshot;

/// Extracts the EarFun Product ID from hardware firmware version string (e.g. "TW600-3091_...").
///
/// In EarFun's architecture:
/// - TW600: EarFun Air Pro 4 (PID 0x004535)
/// - TW400 / TW400B: EarFun Free Pro 3 (PID 0x004534)
/// - TW201: EarFun Air S (PID 0x004C45)
pub fn parse_earfun_product_id_from_firmware(fw: &str) -> Option<u32> {
    if fw.starts_with("TW600") {
        Some(EARFUN_AIR_PRO_4_PRODUCT_ID)
    } else if fw.starts_with("TW400") {
        Some(EARFUN_FREE_PRO_3_PRODUCT_ID)
    } else if fw.starts_with("TW201") {
        Some(EARFUN_AIR_S_PRODUCT_ID)
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn initialize_earfun_session<T: Read + Write>(
    stream: &mut T,
    device_name: &str,
    device_address: Option<&str>,
    discovered_product_id: Option<u32>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(EarFunDeviceSnapshot, EarFunFrameDecoder), String> {
    initialize_earfun_session_with_lap(
        stream,
        device_name,
        device_address,
        discovered_product_id,
        None,
        updates,
    )
}

pub fn initialize_earfun_session_with_lap<T: Read + Write>(
    stream: &mut T,
    device_name: &str,
    device_address: Option<&str>,
    discovered_product_id: Option<u32>,
    local_lap: Option<[u8; 3]>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(EarFunDeviceSnapshot, EarFunFrameDecoder), String> {
    let mut decoder = EarFunFrameDecoder::default();
    let mut detected_features = Vec::new();
    let mut probe_errors = Vec::new();

    // 1. Firmware version
    let mut raw_firmware_string: Option<String> = None;
    let firmware = match request(
        stream,
        &mut decoder,
        EarFunCommands::FIRMWARE_VERSION,
        vec![],
        updates,
    ) {
        Ok(pkt) => {
            if pkt.payload.len() > 1 && pkt.payload[0] == 0x00 {
                if let Ok(s) = std::str::from_utf8(&pkt.payload[1..]) {
                    raw_firmware_string = Some(s.to_string());
                }
            }
            match parse_earfun_firmware_version(&pkt.payload) {
                Ok(fw) => {
                    detected_features.push("firmware-version".to_string());
                    fw
                }
                Err(e) => {
                    probe_errors.push(format!("Firmware version parse error: {e}"));
                    FirmwareVersions::default()
                }
            }
        }
        Err(e) => {
            probe_errors.push(format!("Firmware query failed: {e}"));
            FirmwareVersions::default()
        }
    };

    let product_id = discovered_product_id.or_else(|| {
        raw_firmware_string
            .as_deref()
            .and_then(parse_earfun_product_id_from_firmware)
    });
    let profile = product_id.and_then(compatibility::profile_for_product_id);
    let is_known_model = profile.is_some();
    let read_only = profile.is_none_or(|profile| !profile.supports_controls());
    let model = profile.map_or_else(
        || crate::t!("device-unknown-model"),
        |p| p.model.to_string(),
    );

    if read_only {
        let left_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_LEFT,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
        let right_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_RIGHT,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
        let case_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_CASE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());

        let snapshot = EarFunDeviceSnapshot {
            identity: DeviceIdentitySnapshot {
                name: device_name.to_string(),
                product_id: product_id.unwrap_or(0),
                color_id: None,
                model,
                is_known_model,
                read_only: true,
                firmware,
            },
            battery: BatteryState {
                left: left_bat,
                right: right_bat,
                case: case_bat,
            },
            wear: WearSnapshot {
                wear: WearState::default(),
                supports_wear_detection: false,
            },
            noise: NoiseSnapshot {
                current_mode: None,
                supported_indices: vec![],
                modes: &[],
                support_note: String::new(),
                anc_sub: 2,
                transp_sub: 0,
            },
            sound: SoundSnapshot {
                supports_equalizer: false,
                supports_custom_equalizer: false,
                equalizer_preset: None,
                presets: vec![],
                custom_equalizers: vec![],
                supports_spatial_audio: false,
                supports_head_tracking: false,
                spatial_audio_mode: None,
                supports_game_mode: false,
                game_mode_enabled: None,
                game_mode_feature_id: 0x06,
                supports_bass_engine: false,
                bass_engine_enabled: None,
                bass_engine_level: None,
                supports_hi_res: false,
                hi_res_enabled: None,
                supports_audio_codec: false,
                audio_codec: None,
                available_codecs: vec![],
            },
            controls: ControlsSnapshot::default(),
            dual_device: DualDeviceSnapshot {
                supports_dual_device: false,
                dual_device_enabled: None,
                preferred_device: None,
                paired_devices: Err(crate::t!("dual-device-not-supported")),
                controls_enabled: Err(crate::t!("dual-device-not-supported")),
            },
            diagnostics: ProbeDiagnostics {
                detected_features,
                probe_errors,
                capabilities: vec![],
            },
        };
        return Ok((snapshot, decoder));
    }

    let profile = profile.unwrap();
    let pid = product_id.unwrap();
    let noise_modes = profile.noise_modes;
    let has_submodes = noise_modes.iter().any(|m| m.parent.is_some());
    let noise_mode_indices: Vec<u8> = noise_modes.iter().map(|m| m.index).collect();
    let supports_wear_detection = profile.supports_wear_detection();
    let supports_dual_device = profile.supports_dual_device();
    let supports_game_mode = profile.supports_game_mode();
    let supports_equalizer = profile.supports_equalizer();
    let supports_audio_codec = profile.supports_audio_codec();
    let available_codecs: Vec<u8> = compatibility::supported_earfun_codecs(pid)
        .iter()
        .map(|c| c.as_u8())
        .collect();

    // 2. Battery levels
    let left_bat = request(
        stream,
        &mut decoder,
        EarFunCommands::BATTERY_LEFT,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
    let right_bat = request(
        stream,
        &mut decoder,
        EarFunCommands::BATTERY_RIGHT,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
    let case_bat = if pid != EARFUN_AIR_S_PRODUCT_ID {
        request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_CASE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten())
    } else {
        None
    };

    if left_bat.is_some() || right_bat.is_some() || case_bat.is_some() {
        detected_features.push("battery-status".to_string());
    }

    let battery = BatteryState {
        left: left_bat,
        right: right_bat,
        case: case_bat,
    };

    // 3. Audio & Noise Modes
    let ambient_mode = request(
        stream,
        &mut decoder,
        EarFunCommands::GET_AMBIENT_SOUND,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_ambient_mode(&pkt.payload).ok())
    .unwrap_or(0);

    let anc_sub = if ambient_mode == 1 && has_submodes {
        request(
            stream,
            &mut decoder,
            EarFunCommands::GET_ANC_SUB_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_anc_sub_mode(&pkt.payload).ok())
        .unwrap_or(2)
    } else {
        2 // Default to Strong ANC
    };

    let transp_sub = if ambient_mode == 2 && has_submodes {
        request(
            stream,
            &mut decoder,
            EarFunCommands::GET_TRANSPARENCY_SUB_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_transparency_sub_mode(&pkt.payload).ok())
        .unwrap_or(0)
    } else {
        0 // Default
    };

    let current_protocol_index = if !has_submodes {
        ambient_mode
    } else {
        earfun_mode_to_protocol_index(ambient_mode, anc_sub, transp_sub)
    };

    // 4. Game mode
    let game_mode_enabled = if supports_game_mode {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_GAME_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_game_mode(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("game-mode".to_string());
        }
        enabled
    } else {
        None
    };

    // 5. In-ear detection setting
    let in_ear_detection_enabled = if supports_wear_detection {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_IN_EAR_DETECTION,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_in_ear_detection(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("wear-detection".to_string());
        }
        enabled
    } else {
        None
    };

    // 6. Dual device (multipoint)
    let (dual_device_enabled, paired_devices) = if supports_dual_device {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_CONNECT_TWO_DEVICES,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_dual_device(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("dual-device".to_string());
        }

        let devs =
            read_earfun_paired_devices(stream, &mut decoder, local_lap, updates).map_err(|e| {
                probe_errors.push(e.clone());
                e
            });

        (enabled, devs)
    } else {
        (None, Err(crate::t!("dual-device-not-supported")))
    };

    let paired_device_controls_enabled = if !supports_dual_device {
        Err(crate::t!("dual-device-not-supported"))
    } else if dual_device_enabled == Some(false) {
        Err(crate::t!("dual-device-not-enabled"))
    } else {
        Ok(true)
    };

    // 7. Active audio codec (0x032F)
    let audio_codec = if supports_audio_codec {
        let codec = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_AUDIO_CODEC,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_audio_codec(&pkt.payload).ok());
        if codec.is_some() {
            detected_features.push("audio-codec".to_string());
        }
        codec
    } else {
        None
    };

    // When connected via RFCOMM, earbuds are active and in-ear
    let wear = if supports_wear_detection {
        WearState {
            left: Some(WearPart {
                out_of_case: true,
                in_ear: true,
                raw_flags: 0,
            }),
            right: Some(WearPart {
                out_of_case: true,
                in_ear: true,
                raw_flags: 0,
            }),
            case_open: Some(true),
        }
    } else {
        WearState::default()
    };

    let presets = compatibility::equalizer_presets_for_product(pid).to_vec();
    let snapshot = EarFunDeviceSnapshot {
        identity: DeviceIdentitySnapshot {
            name: device_name.to_string(),
            product_id: pid,
            color_id: None,
            model,
            is_known_model: true,
            read_only: false,
            firmware,
        },
        battery,
        wear: WearSnapshot {
            wear,
            supports_wear_detection,
        },
        noise: NoiseSnapshot {
            current_mode: Some(current_protocol_index),
            supported_indices: noise_mode_indices,
            modes: noise_modes,
            support_note: "EarFun GAIA V3 Noise Architecture".to_string(),
            anc_sub,
            transp_sub,
        },
        sound: SoundSnapshot {
            supports_equalizer,
            supports_custom_equalizer: false,
            equalizer_preset: if supports_equalizer {
                Some(crate::resources::get_saved_equalizer_preset(pid, device_address).unwrap_or(0))
            } else {
                None
            },
            presets,
            custom_equalizers: vec![],
            supports_spatial_audio: false,
            supports_head_tracking: false,
            spatial_audio_mode: None,
            supports_game_mode,
            game_mode_enabled,
            game_mode_feature_id: 0x06,
            supports_bass_engine: false,
            bass_engine_enabled: None,
            bass_engine_level: None,
            supports_hi_res: false,
            hi_res_enabled: None,
            supports_audio_codec,
            audio_codec,
            available_codecs,
        },
        controls: ControlsSnapshot {
            in_ear_detection_enabled,
            ..ControlsSnapshot::default()
        },
        dual_device: DualDeviceSnapshot {
            supports_dual_device,
            dual_device_enabled,
            preferred_device: None,
            paired_devices,
            controls_enabled: paired_device_controls_enabled,
        },
        diagnostics: ProbeDiagnostics {
            detected_features,
            probe_errors,
            capabilities: vec![],
        },
    };

    Ok((snapshot, decoder))
}

#[allow(dead_code)]
pub fn probe_earfun(device: &DeviceChoice) -> Result<(), String> {
    let mut driver = EarFunDriver::new();
    let mut stream = driver
        .open_rfcomm(device)
        .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
    let snapshot = driver.initialize(&mut stream, device, None)?;
    crate::bluetooth::probe::print_snapshot(&snapshot);
    Ok(())
}
