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
use crate::bluetooth::brands::oppo::features::{OppoFeatureContext, OppoSoundState};
use crate::bluetooth::brands::oppo::transport::request;
use crate::protocol::{
    BassEngineLevel, CustomEqualizerAction, CustomEqualizerParams, FEATURE_BASS_ENGINE,
    FEATURE_HI_RES, FrameDecoder, encode_custom_equalizer_command, parse_bass_engine_response,
    parse_custom_equalizer_list, parse_equalizer_response, parse_feature_flags_response,
    parse_spatial_audio_response,
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

pub fn handle_sound_command<T: Read + Write>(
    ctx: &mut OppoFeatureContext<'_, T>,
    command: &WorkerCommand,
    state: &mut OppoSoundState,
) -> bool {
    match command {
        WorkerCommand::SetEqualizer(preset) => {
            if ctx.read_only || !ctx.supports_equalizer() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("equalizer-waiting-confirm")));
            match set_equalizer(ctx.stream, ctx.decoder, *preset, ctx.updates) {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::Equalizer(actual));
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("equalizer-applied")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-switch-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SelectCustomEqualizer(id) => {
            if ctx.read_only || !ctx.supports_custom_equalizer() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("equalizer-custom-switching")));
            let preset = state
                .custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .cloned();
            if let Some(p) = preset {
                let payload =
                    encode_custom_equalizer_command(CustomEqualizerAction::Update, &(&p).into());
                match request(
                    ctx.stream,
                    ctx.decoder,
                    0x0418,
                    0,
                    payload,
                    Some(ctx.updates),
                ) {
                    Ok(_) => {
                        if let Ok(res) = request(
                            ctx.stream,
                            ctx.decoder,
                            0x0122,
                            0,
                            vec![],
                            Some(ctx.updates),
                        ) {
                            if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                                state.custom_equalizers = list.clone();
                                let _ = ctx.updates.send(UiUpdate::CustomEqualizers(list));
                            }
                        }
                        let _ = ctx.updates.send(UiUpdate::Equalizer(*id));
                        let _ = ctx
                            .updates
                            .send(UiUpdate::Status(crate::t!("equalizer-custom-applied")));
                    }
                    Err(error) => {
                        let _ = ctx.updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-custom-switch-failed", error),
                        });
                    }
                }
            }
            true
        }
        WorkerCommand::AddCustomEqualizer { name, bands } => {
            if ctx.read_only || !ctx.supports_custom_equalizer() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("equalizer-custom-adding")));
            let params = CustomEqualizerParams {
                id: 0,
                name,
                min_gain: -6,
                max_gain: 6,
                bands,
            };
            let payload = encode_custom_equalizer_command(CustomEqualizerAction::Create, &params);
            match request(
                ctx.stream,
                ctx.decoder,
                0x0418,
                0,
                payload,
                Some(ctx.updates),
            ) {
                Ok(_) => {
                    if let Ok(res) = request(
                        ctx.stream,
                        ctx.decoder,
                        0x0122,
                        0,
                        vec![],
                        Some(ctx.updates),
                    ) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            state.custom_equalizers = list.clone();
                            let _ = ctx.updates.send(UiUpdate::CustomEqualizers(list));
                        }
                    }
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("equalizer-custom-added")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-add-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SaveCustomEqualizer { id, name, bands } => {
            if ctx.read_only || !ctx.supports_custom_equalizer() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("equalizer-custom-saving")));
            let (min_gain, max_gain) = state
                .custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| (p.min_gain, p.max_gain))
                .unwrap_or((-6, 6));
            let params = CustomEqualizerParams {
                id: *id,
                name,
                min_gain,
                max_gain,
                bands,
            };
            let payload = encode_custom_equalizer_command(CustomEqualizerAction::Update, &params);
            match request(
                ctx.stream,
                ctx.decoder,
                0x0418,
                0,
                payload,
                Some(ctx.updates),
            ) {
                Ok(_) => {
                    if let Ok(res) = request(
                        ctx.stream,
                        ctx.decoder,
                        0x0122,
                        0,
                        vec![],
                        Some(ctx.updates),
                    ) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            state.custom_equalizers = list.clone();
                            let _ = ctx.updates.send(UiUpdate::CustomEqualizers(list));
                        }
                    }
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("equalizer-custom-saved")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-save-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::DeleteCustomEqualizer(id) => {
            if ctx.read_only || !ctx.supports_custom_equalizer() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Equalizer,
                    message: crate::t!("equalizer-custom-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("equalizer-custom-deleting")));
            let (name, min_gain, max_gain, bands) = state
                .custom_equalizers
                .iter()
                .find(|p| p.id == *id)
                .map(|p| (p.name.clone(), p.min_gain, p.max_gain, p.bands.clone()))
                .unwrap_or_else(|| (String::new(), -6, 6, Vec::new()));
            let params = CustomEqualizerParams {
                id: *id,
                name: &name,
                min_gain,
                max_gain,
                bands: &bands,
            };
            let payload = encode_custom_equalizer_command(CustomEqualizerAction::Delete, &params);
            match request(
                ctx.stream,
                ctx.decoder,
                0x0418,
                0,
                payload,
                Some(ctx.updates),
            ) {
                Ok(_) => {
                    if let Ok(res) = request(
                        ctx.stream,
                        ctx.decoder,
                        0x0122,
                        0,
                        vec![],
                        Some(ctx.updates),
                    ) {
                        if let Ok(list) = parse_custom_equalizer_list(&res.payload, true) {
                            state.custom_equalizers = list.clone();
                            let _ = ctx.updates.send(UiUpdate::CustomEqualizers(list));
                        }
                    }
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("equalizer-custom-deleted")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-delete-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetSpatialAudio(mode) => {
            if ctx.read_only || !ctx.supports_spatial_audio() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::SpatialAudio,
                    message: crate::t!("spatial-audio-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("spatial-audio-waiting-confirm")));
            match set_spatial_audio(ctx.stream, ctx.decoder, *mode, ctx.updates) {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::SpatialAudio(actual));
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("spatial-audio-applied")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::SpatialAudio,
                        message: crate::t!("spatial-audio-switch-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetGameMode(enabled) => {
            if ctx.read_only || !ctx.supports_game_mode() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::GameMode,
                    message: crate::t!("game-mode-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("game-mode-waiting-confirm")));
            match set_game_mode(
                ctx.stream,
                ctx.decoder,
                state.game_mode_feature_id,
                *enabled,
                ctx.updates,
            ) {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::GameMode(actual));
                    let _ = ctx.updates.send(UiUpdate::Status(if actual {
                        crate::t!("game-mode-state-enabled")
                    } else {
                        crate::t!("game-mode-state-disabled")
                    }));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::GameMode,
                        message: crate::t!("game-mode-switch-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetBassEngineEnabled(enabled) => {
            if ctx.read_only || !ctx.supports_bass_engine() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::BassEngine,
                    message: crate::t!("bass-engine-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("bass-engine-waiting-confirm")));
            match set_bass_engine_enabled(ctx.stream, ctx.decoder, *enabled, ctx.updates) {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::BassEngineEnabled(actual));
                    let _ = ctx.updates.send(UiUpdate::Status(if actual {
                        crate::t!("bass-engine-state-enabled")
                    } else {
                        crate::t!("bass-engine-state-disabled")
                    }));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::BassEngine,
                        message: crate::t!("bass-engine-switch-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetBassEngineLevel(level) => {
            if ctx.read_only || !ctx.supports_bass_engine() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::BassEngine,
                    message: crate::t!("bass-engine-level-blocked"),
                });
                return true;
            }
            let (min, max) = state
                .bass_engine_level
                .as_ref()
                .map(|b| (b.min, b.max))
                .unwrap_or((-5, 5));
            let _ = ctx.updates.send(UiUpdate::Status(crate::t!(
                "bass-engine-level-waiting-confirm"
            )));
            match set_bass_engine_level(ctx.stream, ctx.decoder, min, max, *level, ctx.updates) {
                Ok(actual) => {
                    state.bass_engine_level = Some(actual.clone());
                    let _ = ctx.updates.send(UiUpdate::BassEngineLevel(actual.clone()));
                    let _ = ctx.updates.send(UiUpdate::Status(crate::t!(
                        "bass-engine-level-applied",
                        actual.current
                    )));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::BassEngine,
                        message: crate::t!("bass-engine-level-switch-failed", error),
                    });
                }
            }
            true
        }
        WorkerCommand::SetHiRes(enabled) => {
            if ctx.read_only || !ctx.supports_hi_res() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::HiRes,
                    message: crate::t!("hi-res-blocked"),
                });
                return true;
            }
            let _ = ctx
                .updates
                .send(UiUpdate::Status(crate::t!("hi-res-waiting-confirm")));
            match set_hi_res(ctx.stream, ctx.decoder, *enabled, ctx.updates) {
                Ok(actual) => {
                    let _ = ctx.updates.send(UiUpdate::HiRes(actual));
                    let _ = ctx.updates.send(UiUpdate::Status(if actual {
                        crate::t!("hi-res-state-enabled")
                    } else {
                        crate::t!("hi-res-state-disabled")
                    }));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
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
