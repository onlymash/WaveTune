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

use crate::{
    compatibility::{EqualizerPreset, GestureRow, NoiseMode},
    protocol::{
        BassEngineLevel, BatteryState, CustomEqualizer, FirmwareVersions, KeyFunction,
        NoiseCycleState, PairedDevice, PairedDeviceAddress, WearState,
    },
};

/// Unified device snapshot partitioned into domain feature slices.
#[derive(Clone, Debug)]
pub struct DeviceSnapshot {
    pub identity: DeviceIdentitySnapshot,
    pub battery: BatteryState,
    pub wear: WearSnapshot,
    pub noise: NoiseSnapshot,
    pub sound: SoundSnapshot,
    pub controls: ControlsSnapshot,
    pub dual_device: DualDeviceSnapshot,
    pub diagnostics: ProbeDiagnostics,
}

#[derive(Clone, Debug)]
pub struct DeviceIdentitySnapshot {
    pub name: String,
    pub model: String,
    pub product_id: u32,
    pub color_id: Option<u8>,
    pub is_known_model: bool,
    pub read_only: bool,
    pub firmware: FirmwareVersions,
}

#[derive(Clone, Debug, Default)]
pub struct WearSnapshot {
    pub wear: WearState,
    pub supports_wear_detection: bool,
}

#[derive(Clone, Debug)]
pub struct NoiseSnapshot {
    pub current_mode: Option<u8>,
    pub supported_indices: Vec<u8>,
    pub modes: &'static [NoiseMode],
    pub support_note: String,
    pub anc_sub: u8,
    pub transp_sub: u8,
}

impl Default for NoiseSnapshot {
    fn default() -> Self {
        Self {
            current_mode: None,
            supported_indices: Vec::new(),
            modes: &[],
            support_note: String::new(),
            anc_sub: 2,
            transp_sub: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoundSnapshot {
    pub supports_equalizer: bool,
    pub supports_custom_equalizer: bool,
    pub equalizer_preset: Option<u8>,
    pub presets: Vec<EqualizerPreset>,
    pub custom_equalizers: Vec<CustomEqualizer>,

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

    pub supports_audio_codec: bool,
    pub audio_codec: Option<u8>,
    pub available_codecs: Vec<u8>,
}

impl Default for SoundSnapshot {
    fn default() -> Self {
        Self {
            supports_equalizer: false,
            supports_custom_equalizer: false,
            equalizer_preset: None,
            presets: Vec::new(),
            custom_equalizers: Vec::new(),
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
            available_codecs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControlsSnapshot {
    pub supports_touch_customization: bool,
    pub supports_sleep_detection: bool,
    pub supports_head_motion: bool,
    pub supports_find_device: bool,
    pub supports_fit_detection: bool,
    pub supports_prompt_volume: bool,

    pub key_functions: Vec<KeyFunction>,
    pub gesture_rows: Vec<GestureRow>,
    pub noise_cycle: NoiseCycleState,
    pub prompt_volume: Option<u8>,
    pub head_motion_type: Option<u8>,
    pub in_ear_detection_enabled: Option<bool>,
    pub sleep_detection_enabled: Option<bool>,
    pub is_finding_earbuds: bool,
}

impl Default for ControlsSnapshot {
    fn default() -> Self {
        Self {
            supports_touch_customization: false,
            supports_sleep_detection: false,
            supports_head_motion: false,
            supports_find_device: false,
            supports_fit_detection: false,
            supports_prompt_volume: false,
            key_functions: Vec::new(),
            gesture_rows: Vec::new(),
            noise_cycle: NoiseCycleState::default(),
            prompt_volume: None,
            head_motion_type: None,
            in_ear_detection_enabled: None,
            sleep_detection_enabled: None,
            is_finding_earbuds: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DualDeviceSnapshot {
    pub supports_dual_device: bool,
    pub dual_device_enabled: Option<bool>,
    pub preferred_device: Option<PairedDeviceAddress>,
    pub paired_devices: Result<Vec<PairedDevice>, String>,
    pub controls_enabled: Result<bool, String>,
}

impl Default for DualDeviceSnapshot {
    fn default() -> Self {
        Self {
            supports_dual_device: false,
            dual_device_enabled: None,
            preferred_device: None,
            paired_devices: Err(String::new()),
            controls_enabled: Err(String::new()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProbeDiagnostics {
    pub detected_features: Vec<String>,
    pub probe_errors: Vec<String>,
    #[allow(dead_code)]
    pub capabilities: Vec<u8>,
}
