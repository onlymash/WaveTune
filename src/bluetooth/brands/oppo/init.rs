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
    thread,
    time::Duration,
};

use crate::bluetooth::{
    ControlsSnapshot, DeviceIdentitySnapshot, DeviceSnapshot, DualDeviceSnapshot, NoiseSnapshot,
    ProbeDiagnostics, SoundSnapshot, UiUpdate, WearSnapshot,
};
use crate::compatibility::{self};
use crate::protocol::{
    BatteryState, FEATURE_BASS_ENGINE, FEATURE_HI_RES, FirmwareVersions, FrameDecoder,
    NoiseCycleState, WearState, capability_bitmap_allows, parse_bass_engine_response,
    parse_battery_response, parse_color_response, parse_custom_equalizer_list,
    parse_equalizer_response, parse_feature_flags_response, parse_firmware_response,
    parse_head_motion_type_response, parse_noise_cycle_response, parse_noise_mode_response,
    parse_priority_device_response, parse_prompt_volume_response, parse_spatial_audio_response,
    parse_touch_controls_response, parse_wear_response,
};

use super::features::dual_device::read_paired_devices;
use super::transport::request;

#[allow(dead_code)]
pub type OppoDeviceSnapshot = DeviceSnapshot;

fn list_separator() -> &'static str {
    match crate::t!("common-list-separator").as_str() {
        " " => " ",
        "、" => "、",
        _ => ", ",
    }
}

pub fn find_feature_in_records(payload: &[u8], feature_id: u8) -> Option<bool> {
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

pub fn query_feature_flag<T: Read + Write>(
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
) -> Result<(DeviceSnapshot, FrameDecoder), String> {
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
