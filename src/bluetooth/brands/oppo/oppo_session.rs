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
    AppPacket, BassEngineLevel, BatteryState, EarSide, FEATURE_BASS_ENGINE, FEATURE_HI_RES,
    FirmwareVersions, FrameDecoder, KeyFunction, NoiseCycleState, PairedDevice, PairedDeviceAction,
    PairedDeviceAddress, WearState, capability_bitmap_allows, encode_custom_equalizer_command,
    encode_find_earbuds, encode_fit_test, encode_frame, encode_set_head_motion_type,
    encode_set_key_function, encode_set_noise_cycle, encode_set_priority_device,
    encode_set_prompt_volume, noise_mode_readback_matches_for, noise_mode_write_payload_for,
    parse_bass_engine_event, parse_bass_engine_response, parse_battery_event,
    parse_battery_response, parse_color_response, parse_custom_equalizer_list,
    parse_equalizer_event, parse_equalizer_response, parse_feature_flags_response,
    parse_firmware_response, parse_fit_test_event, parse_head_motion_type_response,
    parse_noise_cycle_response, parse_noise_mode_event, parse_noise_mode_response,
    parse_paired_devices_event, parse_paired_devices_response, parse_priority_device_response,
    parse_prompt_volume_response, parse_spatial_audio_response, parse_touch_controls_response,
    parse_wear_event, parse_wear_response,
};

use crate::app::events::SettingDomain;
use crate::bluetooth::transport::{RfcommServiceQuery, is_socket_timeout, open_rfcomm};
use crate::bluetooth::{DeviceChoice, UiUpdate, WorkerCommand};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const OPPO_FALLBACK_RFCOMM_CHANNEL: u8 = 15;
const OPPO_PRIMARY_SPP_NAMES: &[&str] = &["elnk", "enco link", "encolink"];
const OPPO_SECONDARY_SPP_NAMES: &[&str] = &["jl_spp"];

pub(crate) fn oppo_candidate_service_uuids(discovered: &[bluer::Uuid]) -> Vec<bluer::Uuid> {
    let mut ordered = discovered.to_vec();
    for candidate in crate::protocol::brands::oppo::OPPO_CONTROL_SERVICE_CANDIDATES {
        if let Ok(uuid) = bluer::Uuid::parse_str(candidate)
            && !ordered.contains(&uuid)
        {
            ordered.push(uuid);
        }
    }
    ordered
}

pub(crate) fn open_oppo_rfcomm(device: &DeviceChoice) -> std::io::Result<std::fs::File> {
    let service_uuids = oppo_candidate_service_uuids(&device.control_service_uuids);
    let query = RfcommServiceQuery {
        service_uuids: &service_uuids,
        primary_spp_names: OPPO_PRIMARY_SPP_NAMES,
        secondary_spp_names: OPPO_SECONDARY_SPP_NAMES,
        fallback_channel: Some(OPPO_FALLBACK_RFCOMM_CHANNEL),
    };
    open_rfcomm(device.address, &query)
}

pub use crate::bluetooth::snapshot::{
    ControlsSnapshot, DeviceIdentitySnapshot, DeviceSnapshot, DualDeviceSnapshot, NoiseSnapshot,
    ProbeDiagnostics, SoundSnapshot, WearSnapshot,
};
pub type OppoDeviceSnapshot = DeviceSnapshot;

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

fn list_separator() -> &'static str {
    if crate::i18n::current_locale().language.as_str() == "zh" {
        "、"
    } else {
        ", "
    }
}

fn clause_separator() -> &'static str {
    if crate::i18n::current_locale().language.as_str() == "zh" {
        "；"
    } else {
        "; "
    }
}

fn monitor_device(
    device: DeviceChoice,
    updates: &Sender<UiUpdate>,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let mut stream = open_oppo_rfcomm(&device)
        .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
    let (mut snapshot, mut decoder) = initialize_session(&mut stream, &device.name, Some(updates))?;

    let _ = updates.send(UiUpdate::Connected {
        device: device.clone(),
    });
    let _ = updates.send(UiUpdate::InitialSession(Box::new(snapshot.clone())));

    let _ = updates.send(UiUpdate::Status(if snapshot.identity.read_only {
        let detected = if snapshot.diagnostics.detected_features.is_empty() {
            crate::t!("probe-no-queries-confirmed")
        } else {
            crate::t!(
                "probe-detected-read-only",
                snapshot
                    .diagnostics
                    .detected_features
                    .join(list_separator())
            )
        };
        if snapshot.diagnostics.probe_errors.is_empty() {
            crate::t!("channel-connected-read-only", detected)
        } else {
            crate::t!(
                "channel-connected-read-only-with-errors",
                detected,
                snapshot.diagnostics.probe_errors.join(clause_separator())
            )
        }
    } else {
        crate::t!("channel-connected-syncing")
    }));
    let mut current_wear = snapshot.wear.wear.clone();
    let mut current_custom_equalizers = snapshot.sound.custom_equalizers.clone();
    let mut current_key_functions = snapshot.controls.key_functions.clone();
    let mut current_noise_cycle = snapshot.controls.noise_cycle;
    let mut current_prompt_volume = snapshot.controls.prompt_volume;
    let mut current_head_motion = snapshot.controls.head_motion_type;
    let mut current_in_ear_detection = snapshot.controls.in_ear_detection_enabled;
    let mut current_sleep_detection = snapshot.controls.sleep_detection_enabled;
    let mut current_dual_device = snapshot.dual_device.dual_device_enabled;
    let mut current_preferred_device = snapshot.dual_device.preferred_device;
    let mut paired_devices = snapshot.dual_device.paired_devices.clone();

    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Disconnect) => {
                let _ = updates.send(UiUpdate::Status(crate::t!("channel-disconnected")));
                return Ok(());
            }
            Ok(WorkerCommand::SetEqualizer(preset)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_equalizer {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-waiting-confirm")));
                match set_equalizer(&mut stream, &mut decoder, preset, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::Equalizer(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-applied")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SelectCustomEqualizer(id)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-switching")));
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
                                .send(UiUpdate::Status(crate::t!("equalizer-custom-applied")));
                        }
                        Err(error) => {
                            let _ = updates.send(UiUpdate::SettingError {
                                domain: SettingDomain::Equalizer,
                                message: crate::t!("equalizer-custom-switch-failed", error),
                            });
                        }
                    }
                }
            }
            Ok(WorkerCommand::AddCustomEqualizer { name, bands }) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-adding")));
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
                        let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-added")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-custom-add-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SaveCustomEqualizer { id, name, bands }) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saving")));
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
                        let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saved")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-custom-save-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::DeleteCustomEqualizer(id)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_custom_equalizer {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-deleting")));
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
                            updates.send(UiUpdate::Status(crate::t!("equalizer-custom-deleted")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-custom-delete-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetSpatialAudio(mode)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_spatial_audio {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::SpatialAudio,
                        message: crate::t!("spatial-audio-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("spatial-audio-waiting-confirm")));
                match set_spatial_audio(&mut stream, &mut decoder, mode, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::SpatialAudio(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!("spatial-audio-applied")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::SpatialAudio,
                            message: crate::t!("spatial-audio-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetGameMode(enabled)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_game_mode {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::GameMode,
                        message: crate::t!("game-mode-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("game-mode-waiting-confirm")));
                match set_game_mode(
                    &mut stream,
                    &mut decoder,
                    snapshot.sound.game_mode_feature_id,
                    enabled,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::GameMode(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            crate::t!("game-mode-state-enabled")
                        } else {
                            crate::t!("game-mode-state-disabled")
                        }));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::GameMode,
                            message: crate::t!("game-mode-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetBassEngineEnabled(enabled)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_bass_engine {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::BassEngine,
                        message: crate::t!("bass-engine-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("bass-engine-waiting-confirm")));
                match set_bass_engine_enabled(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        snapshot.sound.bass_engine_enabled = Some(actual);
                        let _ = updates.send(UiUpdate::BassEngineEnabled(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            crate::t!("bass-engine-state-enabled")
                        } else {
                            crate::t!("bass-engine-state-disabled")
                        }));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::BassEngine,
                            message: crate::t!("bass-engine-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetBassEngineLevel(level)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_bass_engine {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::BassEngine,
                        message: crate::t!("bass-engine-level-blocked"),
                    });
                    continue;
                }
                let (min, max) = snapshot
                    .sound
                    .bass_engine_level
                    .as_ref()
                    .map(|b| (b.min, b.max))
                    .unwrap_or((-5, 5));
                let _ = updates.send(UiUpdate::Status(crate::t!(
                    "bass-engine-level-waiting-confirm"
                )));
                match set_bass_engine_level(&mut stream, &mut decoder, min, max, level, updates) {
                    Ok(actual) => {
                        snapshot.sound.bass_engine_level = Some(actual);
                        let _ = updates.send(UiUpdate::BassEngineLevel(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "bass-engine-level-applied",
                            actual.current
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::BassEngine,
                            message: crate::t!("bass-engine-level-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetHiRes(enabled)) => {
                if snapshot.identity.read_only || !snapshot.sound.supports_hi_res {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::HiRes,
                        message: crate::t!("hi-res-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("hi-res-waiting-confirm")));
                match set_hi_res(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        snapshot.sound.hi_res_enabled = Some(actual);
                        let _ = updates.send(UiUpdate::HiRes(actual));
                        let _ = updates.send(UiUpdate::Status(if actual {
                            crate::t!("hi-res-state-enabled")
                        } else {
                            crate::t!("hi-res-state-disabled")
                        }));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::HiRes,
                            message: crate::t!("hi-res-switch-failed", error),
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetNoiseMode(protocol_index)) => {
                if snapshot.identity.read_only {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::NoiseMode,
                        message: crate::t!("noise-mode-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("noise-mode-waiting-confirm")));
                let result = set_noise_mode(
                    &mut stream,
                    &mut decoder,
                    &snapshot.noise.supported_indices,
                    snapshot.noise.modes,
                    &mut current_wear,
                    snapshot.wear.supports_wear_detection,
                    protocol_index,
                    updates,
                );
                match result {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::NoiseMode(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "noise-mode-confirmed-and-readback"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::NoiseMode,
                            message: error,
                        });
                    }
                }
                let _ = updates.send(UiUpdate::NoiseControlsEnabled(
                    !snapshot.identity.read_only && !snapshot.noise.modes.is_empty(),
                ));
            }
            Ok(WorkerCommand::SetPairedDevice { address, action }) => {
                if snapshot.identity.read_only || !snapshot.dual_device.supports_dual_device {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("dual-device-management-blocked"),
                    });
                    let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    continue;
                }
                let result = set_paired_device(
                    &mut stream,
                    &mut decoder,
                    &snapshot.dual_device.controls_enabled,
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
                                PairedDeviceAction::Remove => crate::t!("dual-device-removed"),
                                _ => crate::t!("dual-device-status-confirmed"),
                            }
                        } else {
                            crate::t!("dual-device-already-in-state")
                        };
                        let _ = updates.send(UiUpdate::Status(message));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: error,
                        });
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
                if snapshot.identity.read_only || !snapshot.controls.supports_touch_customization {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-gesture-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("controls-gesture-updating")));
                let payload = encode_set_key_function(side, button, action, function);
                match request(&mut stream, &mut decoder, 0x0401, 0, payload, Some(updates)) {
                    Ok(_) => {
                        let mut updated = false;
                        if let Ok(res) = request(
                            &mut stream,
                            &mut decoder,
                            0x0108,
                            0,
                            vec![2, 3, 1],
                            Some(updates),
                        ) {
                            if let Ok(list) = parse_touch_controls_response(&res.payload) {
                                if !list.is_empty() {
                                    current_key_functions = list.clone();
                                    let _ = updates.send(UiUpdate::KeyFunctions(list));
                                    updated = true;
                                }
                            }
                        }
                        if !updated {
                            if let Some(existing) = current_key_functions.iter_mut().find(|k| {
                                (k.side == side || k.side == EarSide::Both)
                                    && k.button == button
                                    && k.action == action
                            }) {
                                existing.function = function;
                            } else {
                                current_key_functions.push(KeyFunction {
                                    side,
                                    button,
                                    action,
                                    function,
                                });
                            }
                            let _ =
                                updates.send(UiUpdate::KeyFunctions(current_key_functions.clone()));
                        }
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("controls-gesture-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-gesture-update-failed", error),
                        });
                        let _ = updates.send(UiUpdate::KeyFunctions(current_key_functions.clone()));
                    }
                }
            }
            Ok(WorkerCommand::SetNoiseCycle {
                side,
                mask,
                is_shared,
            }) => {
                if snapshot.identity.read_only || snapshot.noise.modes.is_empty() {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-noise-cycle-blocked"),
                    });
                    continue;
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("controls-noise-cycle-updating")));
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
                        let _ = updates
                            .send(UiUpdate::Status(crate::t!("controls-noise-cycle-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-noise-cycle-update-failed", error),
                        });
                        let _ = updates.send(UiUpdate::NoiseCycle(current_noise_cycle));
                    }
                }
            }
            Ok(WorkerCommand::FindEarbuds(ring)) => {
                if snapshot.identity.read_only || !snapshot.controls.supports_find_device {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-find-blocked"),
                    });
                    continue;
                }
                let payload = encode_find_earbuds(ring);
                match request(&mut stream, &mut decoder, 0x0400, 0, payload, Some(updates)) {
                    Ok(_) => {
                        snapshot.controls.is_finding_earbuds = ring;
                        let _ = updates.send(UiUpdate::IsFindingEarbuds(ring));
                        let _ = updates.send(UiUpdate::Status(if ring {
                            crate::t!("controls-find-ringing")
                        } else {
                            crate::t!("controls-find-stopped")
                        }));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-find-failed", error),
                        });
                        let _ = updates.send(UiUpdate::IsFindingEarbuds(
                            snapshot.controls.is_finding_earbuds,
                        ));
                    }
                }
            }
            Ok(WorkerCommand::RunFitTest) => {
                if snapshot.identity.read_only || !snapshot.controls.supports_fit_detection {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-fit-test-blocked"),
                    });
                    continue;
                }
                if current_wear.left.is_some_and(|p| !p.in_ear)
                    || current_wear.right.is_some_and(|p| !p.in_ear)
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-fit-test-wear-both"),
                    });
                    continue;
                }
                let payload = encode_fit_test(true);
                match request(&mut stream, &mut decoder, 0x0405, 0, payload, Some(updates)) {
                    Ok(_) => {
                        let _ = updates.send(UiUpdate::FitTestRunning(true));
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("controls-fit-test-running")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-fit-test-start-failed", error),
                        });
                        let _ = updates.send(UiUpdate::FitTestRunning(false));
                    }
                }
            }
            Ok(WorkerCommand::SetPromptVolume(volume)) => {
                if snapshot.identity.read_only || !snapshot.controls.supports_prompt_volume {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-prompt-volume-blocked"),
                    });
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
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "controls-prompt-volume-updated"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-prompt-volume-update-failed", error),
                        });
                        if let Some(v) = current_prompt_volume {
                            let _ = updates.send(UiUpdate::PromptVolume(v));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetHeadMotionType(motion_type)) => {
                if snapshot.identity.read_only || !snapshot.controls.supports_head_motion {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-head-motion-blocked"),
                    });
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
                        let _ = updates
                            .send(UiUpdate::Status(crate::t!("controls-head-motion-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-head-motion-update-failed", error),
                        });
                        if let Some(m) = current_head_motion {
                            let _ = updates.send(UiUpdate::HeadMotionType(m));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetDualDeviceConnection(enabled)) => {
                if snapshot.identity.read_only || !snapshot.dual_device.supports_dual_device {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("dual-device-toggle-blocked"),
                    });
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
                                snapshot.dual_device.controls_enabled = Ok(actual);
                                let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(actual));
                                let _ =
                                    updates.send(UiUpdate::PairedDeviceControlsEnabled(Ok(actual)));
                            }
                        }
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("dual-device-toggle-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("dual-device-toggle-failed", error),
                        });
                        if let Some(d) = current_dual_device {
                            let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(d));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetInEarDetection(enabled)) => {
                if snapshot.identity.read_only || !snapshot.wear.supports_wear_detection {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("wear-detection-blocked"),
                    });
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
                        let _ = updates.send(UiUpdate::Status(crate::t!("wear-detection-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("wear-detection-update-failed", error),
                        });
                        if let Some(w) = current_in_ear_detection {
                            let _ = updates.send(UiUpdate::InEarDetectionEnabled(w));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetSleepDetection(enabled)) => {
                if snapshot.identity.read_only || !snapshot.controls.supports_sleep_detection {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-sleep-detection-blocked"),
                    });
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
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "controls-sleep-detection-updated"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-sleep-detection-update-failed", error),
                        });
                        if let Some(s) = current_sleep_detection {
                            let _ = updates.send(UiUpdate::SleepDetectionEnabled(s));
                        }
                    }
                }
            }
            Ok(WorkerCommand::SetPreferredDevice(addr)) => {
                if snapshot.identity.read_only || !snapshot.dual_device.supports_dual_device {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-preferred-device-blocked"),
                    });
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
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "controls-preferred-device-updated"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-preferred-device-update-failed", error),
                        });
                        let _ = updates.send(UiUpdate::PreferredDevice(current_preferred_device));
                    }
                }
            }
            Ok(WorkerCommand::SetAudioCodec(_)) => {}
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        let mut bytes = [0; 1024];
        match stream.read(&mut bytes) {
            Err(error) if is_socket_timeout(&error) => continue,
            Err(error) => return Err(crate::t!("channel-error", error)),
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(length) => {
                let packets = decoder
                    .push(&bytes[..length])
                    .map_err(|error| crate::t!("protocol-frame-decode-failed", error))?;
                for bytes in packets {
                    if let Ok(packet) = AppPacket::decode(&bytes) {
                        if snapshot.identity.read_only {
                            if snapshot.identity.is_known_model
                                && packet.command == 0x0204
                                && matches!(packet.payload.first(), Some(1 | 2))
                            {
                                if packet.payload.first() == Some(&2)
                                    && let Ok(update) = parse_wear_event(&packet.payload)
                                {
                                    merge_wear_state(&mut current_wear, update);
                                }
                                publish_read_only_event(&packet, updates);
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
                        handle_unsolicited(&mut stream, &packet, Some(updates));
                    }
                }
            }
        }
    }
}

fn find_feature_in_records(payload: &[u8], feature_id: u8) -> Option<bool> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return None;
    }
    let count = payload[1] as usize;
    let records = &payload[2..];
    if records.len() < count * 2 {
        return None;
    }
    records[..count * 2]
        .chunks_exact(2)
        .find(|record| record[0] == feature_id)
        .map(|record| record[1] != 0)
}

fn query_feature_flag<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    batch_cache: &Option<Vec<u8>>,
    feature_id: u8,
    updates: Option<&Sender<UiUpdate>>,
) -> Option<bool> {
    if let Some(payload) = batch_cache {
        if let Some(val) = find_feature_in_records(payload, feature_id) {
            return Some(val);
        }
    }
    match request(stream, decoder, 0x010d, 0, vec![1, feature_id], updates) {
        Ok(res) => parse_feature_flags_response(&res.payload, feature_id).ok(),
        Err(_) => None,
    }
}

pub fn initialize_oppo_session<T: Read + Write>(
    stream: &mut T,
    name: &str,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(OppoDeviceSnapshot, FrameDecoder), String> {
    let mut decoder = FrameDecoder::default();

    let mut handshake = None;
    for _ in 0..3 {
        match request(stream, &mut decoder, 0x0100, 0, vec![], updates) {
            Ok(pkt) => {
                handshake = Some(pkt);
                break;
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    let handshake = handshake.ok_or_else(|| crate::t!("device-handshake-missing-caps"))?;
    if handshake.payload.len() < 2 {
        return Err(crate::t!("device-handshake-missing-caps"));
    }
    let capabilities = handshake.payload[1..].to_vec();

    let product = request(stream, &mut decoder, 0x0103, 0, vec![], updates)?;
    if product.payload.len() < 4 {
        return Err(crate::t!("device-product-id-incomplete"));
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
        let is_known_model = profile.is_some();
        let model = profile.map_or_else(
            || crate::t!("device-unknown-model"),
            |p| p.model.to_string(),
        );
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
            crate::t!("device-unknown-model-handshake-only")
        } else if detected_features.is_empty() {
            crate::t!("probe-model-no-queries-confirmed", model.as_str())
        } else {
            crate::t!(
                "probe-model-detected-read-only",
                model.as_str(),
                detected_features.join(list_separator())
            )
        };
        let noise_mode_support_note = crate::t!("probe-read-only-controls-note", note.as_str());
        return Ok((
            DeviceSnapshot {
                identity: DeviceIdentitySnapshot {
                    name: name.to_string(),
                    product_id,
                    color_id,
                    model,
                    is_known_model,
                    read_only: true,
                    firmware,
                },
                battery,
                wear: WearSnapshot {
                    wear,
                    supports_wear_detection: profile
                        .is_some_and(|profile| profile.supports_wear_detection()),
                },
                noise: NoiseSnapshot {
                    current_mode: None,
                    supported_indices: Vec::new(),
                    modes: profile.map_or(&[], |profile| profile.noise_modes),
                    support_note: noise_mode_support_note,
                    anc_sub: 2,
                    transp_sub: 0,
                },
                sound: SoundSnapshot {
                    supports_equalizer: profile.is_some_and(|profile| profile.supports_equalizer()),
                    supports_custom_equalizer: profile
                        .is_some_and(|profile| profile.supports_custom_equalizer()),
                    equalizer_preset: None,
                    presets: Vec::new(),
                    custom_equalizers: Vec::new(),
                    supports_spatial_audio: profile
                        .is_some_and(|profile| profile.supports_spatial_audio()),
                    supports_head_tracking: profile
                        .is_some_and(|profile| profile.supports_head_tracking()),
                    spatial_audio_mode: None,
                    supports_game_mode: profile.is_some_and(|profile| profile.supports_game_mode()),
                    game_mode_enabled: None,
                    game_mode_feature_id: 0x06,
                    supports_bass_engine: profile
                        .is_some_and(|profile| profile.supports_bass_engine()),
                    bass_engine_enabled: None,
                    bass_engine_level: None,
                    supports_hi_res: profile.is_some_and(|profile| profile.supports_hi_res()),
                    hi_res_enabled: None,
                    supports_audio_codec: false,
                    audio_codec: None,
                    available_codecs: Vec::new(),
                },
                controls: ControlsSnapshot {
                    supports_touch_customization: profile
                        .is_some_and(|profile| profile.supports_touch_customization()),
                    supports_sleep_detection: profile
                        .is_some_and(|profile| profile.supports_sleep_detection()),
                    supports_head_motion: profile
                        .is_some_and(|profile| profile.supports_head_motion()),
                    supports_find_device: profile
                        .is_some_and(|profile| profile.supports_find_device()),
                    supports_fit_detection: profile
                        .is_some_and(|profile| profile.supports_fit_detection()),
                    supports_prompt_volume: profile
                        .is_some_and(|profile| profile.supports_prompt_volume()),
                    key_functions: Vec::new(),
                    gesture_rows: Vec::new(),
                    noise_cycle: NoiseCycleState::default(),
                    prompt_volume: None,
                    head_motion_type: None,
                    in_ear_detection_enabled: None,
                    sleep_detection_enabled: None,
                    is_finding_earbuds: false,
                },
                dual_device: DualDeviceSnapshot {
                    supports_dual_device: profile
                        .is_some_and(|profile| profile.supports_dual_device()),
                    dual_device_enabled: None,
                    preferred_device: None,
                    paired_devices: Err(note.clone()),
                    controls_enabled: Err(note),
                },
                diagnostics: ProbeDiagnostics {
                    detected_features,
                    probe_errors,
                    capabilities,
                },
            },
            decoder,
        ));
    }
    let profile = profile.expect("a non-read-only profile must be present");

    let events = request(stream, &mut decoder, 0x0200, 0, vec![], updates)?;
    let event_ids = events
        .payload
        .get(2..)
        .ok_or_else(|| crate::t!("device-event-list-incomplete"))?;
    let event_count = events.payload[1] as usize;
    if event_ids.len() < event_count {
        return Err(crate::t!("device-event-list-mismatch"));
    }
    let supported_events: Vec<u8> = [1, 2, 3, 4, 6]
        .into_iter()
        .filter(|event| {
            event_ids[..event_count].contains(event)
                && (*event != 2 || profile.supports_wear_detection())
                && (*event != 3 || !profile.noise_modes.is_empty())
                && (*event != 4 || profile.supports_fit_detection())
                && (*event != 6 || profile.supports_dual_device())
        })
        .collect();

    if !supported_events.is_empty() && capability_bitmap_allows(&capabilities, 0x0205) {
        let mut subscription = vec![supported_events.len() as u8];
        subscription.extend_from_slice(&supported_events);
        let _ = request(stream, &mut decoder, 0x0205, 0, subscription, updates);
    }

    let battery = request(stream, &mut decoder, 0x0106, 0xf0, vec![], updates)?;
    let battery = parse_battery_response(&battery.payload)
        .map_err(|error| crate::t!("battery-response-invalid", error))?;

    let firmware = request(stream, &mut decoder, 0x0105, 0, vec![], updates)?;
    let firmware = parse_firmware_response(&firmware.payload)
        .map_err(|error| crate::t!("firmware-response-invalid", error))?;

    let wear = if profile.supports_wear_detection() {
        let wear = request(stream, &mut decoder, 0x0109, 0, vec![], updates)?;
        parse_wear_response(&wear.payload)
            .map_err(|error| crate::t!("wear-response-invalid", error))?
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

    let supports_game_mode = profile.supports_game_mode();
    let game_mode_feature_id = if capability_bitmap_allows(&capabilities, 0x0423) {
        0x28
    } else {
        compatibility::game_mode_feature_id_for_product(product_id)
    };

    let mut feature_ids_to_query = Vec::new();
    if profile.supports_dual_device() {
        feature_ids_to_query.push(17);
    }
    if supports_game_mode {
        feature_ids_to_query.push(game_mode_feature_id);
    }
    if profile.supports_bass_engine() {
        feature_ids_to_query.push(FEATURE_BASS_ENGINE);
    }
    if profile.supports_hi_res() {
        feature_ids_to_query.push(FEATURE_HI_RES);
    }
    if profile.supports_wear_detection() {
        feature_ids_to_query.push(0x04);
    }
    if profile.supports_sleep_detection() {
        feature_ids_to_query.push(0x3a);
    }

    let batch_features_payload = if !feature_ids_to_query.is_empty() {
        let mut req = vec![feature_ids_to_query.len() as u8];
        req.extend_from_slice(&feature_ids_to_query);
        match request(stream, &mut decoder, 0x010d, 0, req, updates) {
            Ok(res) => Some(res.payload),
            Err(_) => None,
        }
    } else {
        None
    };

    let paired_device_controls_enabled = if !profile.supports_dual_device() {
        Err(crate::t!("dual-device-profile-missing"))
    } else {
        match query_feature_flag(stream, &mut decoder, &batch_features_payload, 17, updates) {
            Some(enabled) => Ok(enabled),
            None => Ok(true),
        }
    };

    let supports_paired_devices = profile.supports_dual_device();
    let paired_devices = if supports_paired_devices {
        read_paired_devices(stream, &mut decoder, updates)
    } else {
        Err(crate::t!("dual-device-profile-missing"))
    };

    let supports_equalizer = profile.supports_equalizer();
    let equalizer_preset = if supports_equalizer {
        match request(stream, &mut decoder, 0x010f, 0, vec![], updates) {
            Ok(response) => parse_equalizer_response(&response.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_custom_equalizer =
        profile.supports_custom_equalizer() && capability_bitmap_allows(&capabilities, 0x0418);
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

    let supports_spatial_audio = profile.supports_spatial_audio();
    let supports_head_tracking = profile.supports_head_tracking();
    let spatial_audio_mode = if supports_spatial_audio {
        match request(stream, &mut decoder, 0x012a, 0, vec![], updates) {
            Ok(response) => parse_spatial_audio_response(&response.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let game_mode_enabled = if supports_game_mode {
        query_feature_flag(
            stream,
            &mut decoder,
            &batch_features_payload,
            game_mode_feature_id,
            updates,
        )
    } else {
        None
    };

    let supports_bass_engine = profile.supports_bass_engine();
    let (bass_engine_enabled, bass_engine_level) = if supports_bass_engine {
        let enabled = query_feature_flag(
            stream,
            &mut decoder,
            &batch_features_payload,
            FEATURE_BASS_ENGINE,
            updates,
        );
        let level = match request(stream, &mut decoder, 0x0124, 0, vec![], updates) {
            Ok(response) => parse_bass_engine_response(&response.payload).ok(),
            Err(_) => None,
        };
        (enabled, level)
    } else {
        (None, None)
    };

    let supports_hi_res = profile.supports_hi_res();
    let hi_res_enabled = if supports_hi_res {
        query_feature_flag(
            stream,
            &mut decoder,
            &batch_features_payload,
            FEATURE_HI_RES,
            updates,
        )
    } else {
        None
    };

    let supports_touch_customization = profile.supports_touch_customization();
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

    let supports_prompt_volume = profile.supports_prompt_volume();
    let prompt_volume = if supports_prompt_volume {
        match request(stream, &mut decoder, 0x0130, 0, vec![], updates) {
            Ok(res) => parse_prompt_volume_response(&res.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let supports_head_motion = profile.supports_head_motion();
    let head_motion_type = if supports_head_motion {
        match request(stream, &mut decoder, 0x0134, 0, vec![], updates) {
            Ok(res) => parse_head_motion_type_response(&res.payload).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let preferred_device = if profile.supports_dual_device() {
        match request(stream, &mut decoder, 0x0132, 0, vec![2], updates) {
            Ok(res) => parse_priority_device_response(&res.payload).ok().flatten(),
            Err(_) => None,
        }
    } else {
        None
    };

    let in_ear_detection_enabled = if profile.supports_wear_detection() {
        query_feature_flag(stream, &mut decoder, &batch_features_payload, 0x04, updates)
    } else {
        None
    };

    let sleep_detection_enabled = if profile.supports_sleep_detection() {
        query_feature_flag(stream, &mut decoder, &batch_features_payload, 0x3a, updates)
    } else {
        None
    };

    let dual_device_enabled = paired_device_controls_enabled.as_ref().ok().copied();

    if capability_bitmap_allows(&capabilities, 0x0419) {
        let _ = request(stream, &mut decoder, 0x0419, 0xf0, vec![0x00], updates);
    }

    let presets = compatibility::equalizer_presets_for_product(product_id).to_vec();
    let gesture_rows = compatibility::gesture_rows_for_product(product_id).to_vec();

    Ok((
        DeviceSnapshot {
            identity: DeviceIdentitySnapshot {
                name: name.to_string(),
                product_id,
                color_id,
                model: profile.model.to_string(),
                is_known_model: true,
                read_only: false,
                firmware,
            },
            battery,
            wear: WearSnapshot {
                wear,
                supports_wear_detection: profile.supports_wear_detection(),
            },
            noise: NoiseSnapshot {
                current_mode: noise_mode,
                supported_indices: noise_mode_indices,
                modes: profile.noise_modes,
                support_note: noise_mode_support_note,
                anc_sub: 2,
                transp_sub: 0,
            },
            sound: SoundSnapshot {
                supports_equalizer,
                supports_custom_equalizer,
                equalizer_preset,
                presets,
                custom_equalizers,
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
                supports_audio_codec: false,
                audio_codec: None,
                available_codecs: Vec::new(),
            },
            controls: ControlsSnapshot {
                supports_touch_customization,
                supports_sleep_detection: profile.supports_sleep_detection(),
                supports_head_motion,
                supports_find_device: profile.supports_find_device(),
                supports_fit_detection: profile.supports_fit_detection(),
                supports_prompt_volume,
                key_functions,
                gesture_rows,
                noise_cycle,
                prompt_volume,
                head_motion_type,
                in_ear_detection_enabled,
                sleep_detection_enabled,
                is_finding_earbuds: false,
            },
            dual_device: DualDeviceSnapshot {
                supports_dual_device: supports_paired_devices,
                dual_device_enabled,
                preferred_device,
                paired_devices,
                controls_enabled: paired_device_controls_enabled,
            },
            diagnostics: ProbeDiagnostics {
                detected_features: Vec::new(),
                probe_errors: Vec::new(),
                capabilities,
            },
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
                    crate::t!("device-no-declared-events")
                } else {
                    events
                        .iter()
                        .map(|event| format!("{event:02X}"))
                        .collect::<Vec<_>>()
                        .join(list_separator())
                };
                detected_features.push(crate::t!("event-list-read", event_summary));
                Some(events)
            }
            Err(error) => {
                probe_errors.push(crate::t!("event-list-invalid", error));
                None
            }
        },
        Err(error) => {
            probe_errors.push(crate::t!("event-list-query-failed", error));
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
                    detected_features.push(crate::t!("battery-wear-notifications"))
                }
                Ok(response) => {
                    let missing = crate::t!("common-missing");
                    let status_str = response
                        .payload
                        .first()
                        .map_or(missing, |status| format!("0x{status:02X}"));
                    probe_errors.push(crate::t!("event-subscribe-rejected", status_str));
                }
                Err(error) => probe_errors.push(crate::t!("event-subscribe-failed", error)),
            }
        }
    }

    match request(stream, decoder, 0x0106, 0xf0, vec![], updates) {
        Ok(response) => match parse_battery_response(&response.payload) {
            Ok(state) => {
                if state.left.is_some() || state.right.is_some() || state.case.is_some() {
                    detected_features.push(crate::t!("battery-read"));
                    battery = state;
                } else {
                    probe_errors.push(crate::t!("battery-query-no-data"));
                }
            }
            Err(error) => probe_errors.push(crate::t!("battery-response-invalid", error)),
        },
        Err(error) => probe_errors.push(crate::t!("battery-query-failed", error)),
    }

    match request(stream, decoder, 0x0105, 0, vec![], updates) {
        Ok(response) => match parse_firmware_response(&response.payload) {
            Ok(state) => {
                detected_features.push(crate::t!("firmware-version-read"));
                firmware = state;
            }
            Err(error) => probe_errors.push(crate::t!("firmware-response-invalid", error)),
        },
        Err(error) => probe_errors.push(crate::t!("firmware-query-failed", error)),
    }

    match request(stream, decoder, 0x0109, 0, vec![], updates) {
        Ok(response) => match parse_wear_response(&response.payload) {
            Ok(state) => {
                if state.left.is_some() || state.right.is_some() || state.case_open.is_some() {
                    detected_features.push(crate::t!("wear-case-lid-read"));
                    wear = state;
                } else {
                    probe_errors.push(crate::t!("wear-status-no-data"));
                }
            }
            Err(error) => probe_errors.push(crate::t!("wear-response-invalid", error)),
        },
        Err(error) => probe_errors.push(crate::t!("wear-query-failed", error)),
    }

    (battery, firmware, wear, detected_features, probe_errors)
}

fn parse_supported_events(payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(crate::t!("device-response-missing-status"));
    }
    let count = payload[1] as usize;
    let events = payload
        .get(2..)
        .filter(|events| events.len() == count)
        .ok_or_else(|| crate::t!("device-event-count-mismatch"))?;
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
        return Err(crate::t!("wear-earbuds-not-in-ear"));
    }
    let (deferred_tx, deferred_rx) = mpsc::channel();
    let result = (|| {
        if !compatibility::noise_mode_supported_for(protocol_index, supported_indices, modes) {
            return Err(crate::t!(
                "noise-mode-not-in-supported-list",
                protocol_index
            ));
        }
        let payload = noise_mode_write_payload_for(protocol_index, modes)
            .ok_or_else(|| crate::t!("noise-mode-not-enabled-in-profile", protocol_index))?;
        request(stream, decoder, 0x0404, 0, payload, Some(&deferred_tx))?;

        let result = request(stream, decoder, 0x010c, 0, vec![1, 1], Some(&deferred_tx))?;
        let mut actual = parse_noise_mode_response(&result.payload)
            .ok_or_else(|| crate::t!("noise-mode-readback-unrecognized"))?;
        if !noise_mode_readback_matches_for(protocol_index, actual, modes) {
            for delay_ms in [250, 500] {
                thread::sleep(Duration::from_millis(delay_ms));
                if let Ok(retry) =
                    request(stream, decoder, 0x010c, 0, vec![1, 1], Some(&deferred_tx))
                {
                    if let Some(retry_val) = parse_noise_mode_response(&retry.payload) {
                        if noise_mode_readback_matches_for(protocol_index, retry_val, modes) {
                            actual = retry_val;
                            break;
                        }
                    }
                }
            }
        }
        if !noise_mode_readback_matches_for(protocol_index, actual, modes) {
            return Err(crate::t!(
                "noise-mode-readback-mismatch",
                actual,
                protocol_index
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
        Ok(false) => return Err(crate::t!("dual-device-not-enabled")),
        Err(error) => return Err(crate::t!("dual-device-confirm-status-failed", error)),
    }

    let paired_devices = paired_devices
        .as_ref()
        .map_err(|error| crate::t!("dual-device-read-list-failed", error))?;
    let target = paired_devices
        .iter()
        .find(|device| device.address == address)
        .ok_or_else(|| crate::t!("dual-device-not-in-list"))?;
    if target.is_current_host {
        return Err(match action {
            PairedDeviceAction::Remove => crate::t!("dual-device-this-host-cannot-remove"),
            _ => crate::t!("dual-device-this-host-cannot-disconnect"),
        });
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
                .ok_or_else(|| crate::t!("dual-device-target-missing"))?;
            if updated_target.connected != expected {
                return Err(crate::t!("dual-device-readback-mismatch"));
            }
            Ok((updated, true))
        }
        PairedDeviceAction::Remove => {
            if target.connected {
                return Err(crate::t!("dual-device-cannot-delete-connected"));
            }

            let updated = issue_paired_device_action(stream, decoder, address, action, updates)?;
            if updated.iter().any(|device| device.address == address) {
                return Err(crate::t!("dual-device-deletion-unconfirmed"));
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
        .map_err(|error| crate::t!("dual-device-readback-list-failed", error))
}

fn read_paired_devices<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<PairedDevice>, String> {
    let response = request(stream, decoder, 0x0112, 0, vec![], updates)?;
    parse_paired_devices_response(&response.payload)
        .map_err(|error| crate::t!("dual-device-list-invalid", error))
}

pub fn set_equalizer<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    preset: u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    request(stream, decoder, 0x0406, 0, vec![preset], Some(updates))?;
    let result = request(stream, decoder, 0x010f, 0, vec![], Some(updates))?;
    let mut actual = parse_equalizer_response(&result.payload)
        .map_err(|error| crate::t!("equalizer-readback-invalid", error))?;
    if actual != preset {
        for delay_ms in [250, 500] {
            thread::sleep(Duration::from_millis(delay_ms));
            if let Ok(retry) = request(stream, decoder, 0x010f, 0, vec![], Some(updates)) {
                if let Ok(retry_val) = parse_equalizer_response(&retry.payload) {
                    if retry_val == preset {
                        actual = retry_val;
                        break;
                    }
                }
            }
        }
    }
    if actual != preset {
        return Err(crate::t!("equalizer-readback-mismatch", actual, preset));
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
    let mut actual = parse_spatial_audio_response(&result.payload)
        .map_err(|error| crate::t!("spatial-audio-readback-invalid", error))?;
    if actual != mode {
        for delay_ms in [250, 500] {
            thread::sleep(Duration::from_millis(delay_ms));
            if let Ok(retry) = request(stream, decoder, 0x012a, 0, vec![], Some(updates)) {
                if let Ok(retry_val) = parse_spatial_audio_response(&retry.payload) {
                    if retry_val == mode {
                        actual = retry_val;
                        break;
                    }
                }
            }
        }
    }
    if actual != mode {
        return Err(crate::t!("spatial-audio-readback-mismatch", actual, mode));
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
    set_feature_toggle(
        stream,
        decoder,
        feature_id,
        enabled,
        &crate::t!("game-mode-title"),
        updates,
    )
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
        &crate::t!("bass-engine-title"),
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
        .map_err(|error| crate::t!("bass-engine-readback-invalid", error))?;
    if actual.current != clamped {
        return Err(crate::t!(
            "bass-engine-level-readback-mismatch",
            actual.current,
            clamped
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
        &crate::t!("hi-res-title"),
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
    let mut actual = parse_feature_flags_response(&result.payload, feature_id)
        .map_err(|error| crate::t!("feature-readback-invalid", feature_name, error))?;
    if actual != enabled {
        for delay_ms in [250, 500] {
            thread::sleep(Duration::from_millis(delay_ms));
            if let Ok(retry) = request(
                stream,
                decoder,
                0x010d,
                0,
                vec![1, feature_id],
                Some(updates),
            ) {
                if let Ok(retry_val) = parse_feature_flags_response(&retry.payload, feature_id) {
                    if retry_val == enabled {
                        actual = retry_val;
                        break;
                    }
                }
            }
        }
    }
    if actual != enabled {
        return Err(crate::t!("feature-readback-mismatch", feature_name));
    }
    Ok(actual)
}

fn reply_packet<T: Write>(
    stream: &mut T,
    command: u16,
    sequence: u8,
    payload: Vec<u8>,
) -> Result<(), String> {
    let packet = AppPacket {
        command: command | 0x8000,
        sequence,
        payload,
    };
    let packet_bytes = packet
        .encode()
        .map_err(|error| crate::t!("protocol-command-encode-failed", error))?;
    let frame = encode_frame(&packet_bytes)
        .map_err(|error| crate::t!("protocol-frame-encode-failed", error))?;
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;
    let _ = stream.flush();
    Ok(())
}

pub(crate) fn request<T: Read + Write>(
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
            .map_err(|error| crate::t!("protocol-command-encode-failed", error))?,
    )
    .map_err(|error| crate::t!("protocol-frame-encode-failed", error))?;
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    let deadline = Instant::now() + REQUEST_TIMEOUT;
    loop {
        let mut bytes = [0; 1024];
        let length = match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(length) => length,
            Err(error) if is_socket_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(crate::t!(
                        "protocol-query-timeout",
                        format!("{command:04X}")
                    ));
                }
                continue;
            }
            Err(error) => return Err(crate::t!("protocol-response-read-failed", error)),
        };
        let frames = decoder
            .push(&bytes[..length])
            .map_err(|error| crate::t!("protocol-frame-decode-failed", error))?;
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
            handle_unsolicited(stream, &received, updates);
        }
        if let Some(response) = response {
            for bytes in trailing_packets {
                if let Ok(packet) = AppPacket::decode(&bytes) {
                    handle_unsolicited(stream, &packet, updates);
                }
            }
            if response.payload.first() != Some(&0) {
                let missing = crate::t!("common-missing");
                let status_str = response
                    .payload
                    .first()
                    .map_or(missing, |status| format!("0x{status:02X}"));
                return Err(crate::t!(
                    "protocol-command-rejected",
                    format!("{command:04X}"),
                    status_str
                ));
            }
            return Ok(response);
        }
        if Instant::now() >= deadline {
            return Err(crate::t!(
                "protocol-query-timeout",
                format!("{command:04X}")
            ));
        }
    }
}

fn handle_unsolicited<T: Write>(
    stream: &mut T,
    packet: &AppPacket,
    updates: Option<&Sender<UiUpdate>>,
) {
    match packet.command {
        0x0500 => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            let mut payload = vec![0x00];
            payload.extend_from_slice(&now.to_le_bytes());
            let _ = reply_packet(stream, 0x0500, packet.sequence, payload);
            return;
        }
        0x051C => {
            let _ = reply_packet(stream, 0x051C, packet.sequence, vec![0x00, 0x00, 0x00]);
            return;
        }
        0x051D => {
            let _ = reply_packet(stream, 0x051D, packet.sequence, vec![0x01]);
            return;
        }
        0x050C => {
            let _ = reply_packet(stream, 0x050C, packet.sequence, vec![0x00, 0x01, 0x00]);
            return;
        }
        0x0402 => {
            let _ = reply_packet(stream, 0x0402, packet.sequence, vec![0x00]);
            return;
        }
        _ => {}
    }

    if packet.command == 0x0204 {
        match packet.payload.first() {
            Some(&0xF2) => {
                let _ = reply_packet(stream, 0x0204, packet.sequence, vec![0x00, 0xF2]);
                return;
            }
            Some(&0xF4) => {
                let mut cmd = String::new();
                if packet.payload.len() > 1 {
                    if let Ok(value) =
                        serde_json::from_slice::<serde_json::Value>(&packet.payload[1..])
                    {
                        if let Some(c) = value.get("cmd").and_then(|v| v.as_str()) {
                            cmd = c.to_string();
                        }
                    }
                }
                let reply_json = serde_json::json!({ "cmd": cmd }).to_string();
                let mut reply_payload = vec![0x00, 0xF4];
                reply_payload.extend_from_slice(reply_json.as_bytes());
                let _ = reply_packet(stream, 0x0204, packet.sequence, reply_payload);
                return;
            }
            _ => {}
        }
    }

    let Some(updates) = updates else {
        return;
    };

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
            } else if feature_id == 0x04 {
                let _ = updates.send(UiUpdate::InEarDetectionEnabled(val != 0));
            } else if feature_id == 17 {
                let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(val != 0));
                let _ = updates.send(UiUpdate::PairedDeviceControlsEnabled(Ok(val != 0)));
            } else if feature_id == 0x3a {
                let _ = updates.send(UiUpdate::SleepDetectionEnabled(val != 0));
            } else if feature_id == 27 {
                let mode = if val != 0 { 1 } else { 0 };
                let _ = updates.send(UiUpdate::SpatialAudio(mode));
            }
        }
        return;
    }
    if (packet.command == 0x012a || packet.command == 0x812a || packet.command == 0x0510)
        && !packet.payload.is_empty()
    {
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
                .map_err(|error| crate::t!("dual-device-event-invalid", error));
            let _ = updates.send(UiUpdate::PairedDevices(result));
        }
        _ => {}
    }
}

fn publish_read_only_event(packet: &AppPacket, updates: &Sender<UiUpdate>) {
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
