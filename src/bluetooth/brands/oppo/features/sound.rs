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

use crate::app::events::{SettingDomain, UiUpdate, WorkerCommand};
use crate::bluetooth::brands::oppo::transport::request;
use crate::protocol::{
    BassEngineLevel, CustomEqualizer, FEATURE_BASS_ENGINE, FEATURE_HI_RES, FrameDecoder,
    encode_custom_equalizer_command, parse_bass_engine_response, parse_custom_equalizer_list,
    parse_equalizer_response, parse_feature_flags_response, parse_spatial_audio_response,
};

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

pub fn set_feature_toggle<T: Read + Write>(
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

#[allow(clippy::too_many_arguments)]
pub fn handle_sound_command<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    command: &WorkerCommand,
    read_only: bool,
    supports_equalizer: bool,
    supports_custom_equalizer: bool,
    current_custom_equalizers: &mut Vec<CustomEqualizer>,
    supports_spatial_audio: bool,
    supports_game_mode: bool,
    game_mode_feature_id: u8,
    supports_bass_engine: bool,
    bass_engine_level: &mut Option<BassEngineLevel>,
    supports_hi_res: bool,
    updates: &Sender<UiUpdate>,
) -> bool {
    match command {
        WorkerCommand::SetEqualizer(preset) => {
            if read_only || !supports_equalizer {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-waiting-confirm")));
            match set_equalizer(stream, decoder, *preset, updates) {
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
            true
        }
        WorkerCommand::SelectCustomEqualizer(id) => {
            if read_only || !supports_custom_equalizer {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-switching")));
            let preset = current_custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .cloned();
            if let Some(p) = preset {
                let payload = encode_custom_equalizer_command(
                    0x02, p.id, &p.name, p.min_gain, p.max_gain, &p.bands,
                );
                match request(stream, decoder, 0x0418, 0, payload, Some(updates)) {
                    Ok(_) => {
                        if let Ok(res) = request(stream, decoder, 0x0122, 0, vec![], Some(updates))
                        {
                            if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                *current_custom_equalizers = list.clone();
                                let _ = updates.send(UiUpdate::CustomEqualizers(list));
                            }
                        }
                        let _ = updates.send(UiUpdate::Equalizer(*id));
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("equalizer-custom-applied")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-custom-switch-failed", error),
                        });
                    }
                }
            }
            true
        }
        WorkerCommand::AddCustomEqualizer { name, bands } => {
            if read_only || !supports_custom_equalizer {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-adding")));
            let payload = encode_custom_equalizer_command(0x01, 0, name, -6, 6, bands);
            match request(stream, decoder, 0x0418, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) = request(stream, decoder, 0x0122, 0, vec![], Some(updates)) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            *current_custom_equalizers = list.clone();
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
            true
        }
        WorkerCommand::SaveCustomEqualizer { id, name, bands } => {
            if read_only || !supports_custom_equalizer {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saving")));
            let (min_gain, max_gain) = current_custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| (p.min_gain, p.max_gain))
                .unwrap_or((-6, 6));
            let payload =
                encode_custom_equalizer_command(0x02, *id, name, min_gain, max_gain, bands);
            match request(stream, decoder, 0x0418, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) = request(stream, decoder, 0x0122, 0, vec![], Some(updates)) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            *current_custom_equalizers = list.clone();
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
            true
        }
        WorkerCommand::DeleteCustomEqualizer(id) => {
            if read_only || !supports_custom_equalizer {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-deleting")));
            let (name, min_gain, max_gain, bands) = current_custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| (p.name.clone(), p.min_gain, p.max_gain, p.bands.clone()))
                .unwrap_or_else(|| (String::new(), -6, 6, Vec::new()));
            let payload =
                encode_custom_equalizer_command(0x03, *id, &name, min_gain, max_gain, &bands);
            match request(stream, decoder, 0x0418, 0, payload, Some(updates)) {
                Ok(_) => {
                    if let Ok(res) = request(stream, decoder, 0x0122, 0, vec![], Some(updates)) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            *current_custom_equalizers = list.clone();
                            let _ = updates.send(UiUpdate::CustomEqualizers(list));
                        }
                    }
                    let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-deleted")));
                }
                Err(error) => {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-delete-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetSpatialAudio(mode) => {
            if read_only || !supports_spatial_audio {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::SpatialAudio,
                    message: crate::t!("spatial-audio-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("spatial-audio-waiting-confirm")));
            match set_spatial_audio(stream, decoder, *mode, updates) {
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
            true
        }
        WorkerCommand::SetGameMode(enabled) => {
            if read_only || !supports_game_mode {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::GameMode,
                    message: crate::t!("game-mode-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("game-mode-waiting-confirm")));
            match set_game_mode(stream, decoder, game_mode_feature_id, *enabled, updates) {
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
            true
        }
        WorkerCommand::SetBassEngineEnabled(enabled) => {
            if read_only || !supports_bass_engine {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::BassEngine,
                    message: crate::t!("bass-engine-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("bass-engine-waiting-confirm")));
            match set_bass_engine_enabled(stream, decoder, *enabled, updates) {
                Ok(actual) => {
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
            true
        }
        WorkerCommand::SetBassEngineLevel(level) => {
            if read_only || !supports_bass_engine {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::BassEngine,
                    message: crate::t!("bass-engine-level-blocked"),
                });
                return true;
            }
            let (min, max) = bass_engine_level
                .as_ref()
                .map(|b| (b.min, b.max))
                .unwrap_or((-5, 5));
            let _ = updates.send(UiUpdate::Status(crate::t!(
                "bass-engine-level-waiting-confirm"
            )));
            match set_bass_engine_level(stream, decoder, min, max, *level, updates) {
                Ok(actual) => {
                    *bass_engine_level = Some(actual);
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
            true
        }
        WorkerCommand::SetHiRes(enabled) => {
            if read_only || !supports_hi_res {
                let _ = updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::HiRes,
                    message: crate::t!("hi-res-blocked"),
                });
                return true;
            }
            let _ = updates.send(UiUpdate::Status(crate::t!("hi-res-waiting-confirm")));
            match set_hi_res(stream, decoder, *enabled, updates) {
                Ok(actual) => {
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
            true
        }
        _ => false,
    }
}
