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

use std::path::PathBuf;

use crate::{
    compatibility::{EqualizerPreset, GestureRow, NoiseMode},
    protocol::{
        BassEngineLevel, BatteryState, CustomEqualizer, KeyFunction, NoiseCycleState, PairedDevice,
        PairedDeviceAddress, WearState,
    },
};

/// Unified state for a headset control session.
#[derive(Clone, Debug)]
pub struct HeadsetSessionState {
    pub connected: bool,
    pub device_name: String,
    pub product_id: Option<u32>,
    pub artwork_path: Option<PathBuf>,
    pub battery: BatteryState,
    pub case_battery_is_stale: bool,
    pub wear: WearState,
    pub supports_wear_detection: bool,

    pub noise: NoiseState,
    pub sound: SoundState,
    pub controls: ControlsState,
    pub dual_device: DualDeviceState,
}

impl Default for HeadsetSessionState {
    fn default() -> Self {
        Self {
            connected: false,
            device_name: String::new(),
            product_id: None,
            artwork_path: None,
            battery: BatteryState::default(),
            case_battery_is_stale: false,
            wear: WearState::default(),
            supports_wear_detection: true,
            noise: NoiseState::default(),
            sound: SoundState::default(),
            controls: ControlsState::default(),
            dual_device: DualDeviceState::default(),
        }
    }
}

impl HeadsetSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset device features and states when disconnected.
    pub fn reset_on_disconnect(&mut self) {
        self.connected = false;
        self.battery = BatteryState::default();
        self.case_battery_is_stale = false;
        self.supports_wear_detection = true;
        self.product_id = None;
        self.artwork_path = None;

        self.noise.reset_on_disconnect();
        self.sound.reset_on_disconnect();
        self.controls.reset_on_disconnect();
        self.dual_device.reset_on_disconnect();
    }
}

#[derive(Clone, Debug)]
pub struct NoiseState {
    pub enabled: bool,
    pub current_mode: Option<u8>,
    pub last_non_off_mode: Option<u8>,
    pub supported_indices: Vec<u8>,
    pub modes: Vec<NoiseMode>,
    pub support_note: String,
}

impl Default for NoiseState {
    fn default() -> Self {
        Self {
            enabled: false,
            current_mode: None,
            last_non_off_mode: None,
            supported_indices: Vec::new(),
            modes: Vec::new(),
            support_note: crate::t!("noise-mode-read-support-after-connecting"),
        }
    }
}

impl NoiseState {
    pub fn reset_on_disconnect(&mut self) {
        self.enabled = false;
        self.current_mode = None;
    }
}

#[derive(Default, Clone, Debug)]
pub struct SoundState {
    // Equalizer
    pub supports_eq: bool,
    pub supports_custom_eq: bool,
    pub current_eq_preset: Option<u8>,
    pub presets: Vec<EqualizerPreset>,
    pub custom_equalizers: Vec<CustomEqualizer>,

    // Spatial audio
    pub supports_spatial: bool,
    pub supports_head_tracking: bool,
    pub current_spatial_mode: Option<u8>,

    // Game mode
    pub supports_game_mode: bool,
    pub current_game_mode: Option<bool>,

    // Bass engine
    pub supports_bass_engine: bool,
    pub bass_engine_enabled: Option<bool>,
    pub bass_engine_level: Option<BassEngineLevel>,

    // Hi-Res & audio codec
    pub supports_hi_res: bool,
    pub hi_res_enabled: Option<bool>,
    pub supports_audio_codec: bool,
    pub current_audio_codec: Option<u8>,
    pub available_codecs: Vec<u8>,
}

impl SoundState {
    pub fn reset_on_disconnect(&mut self) {
        self.supports_eq = false;
        self.supports_custom_eq = false;
        self.current_eq_preset = None;
        self.presets.clear();
        self.custom_equalizers.clear();

        self.supports_spatial = false;
        self.supports_head_tracking = false;
        self.current_spatial_mode = None;

        self.supports_game_mode = false;
        self.current_game_mode = None;

        self.supports_bass_engine = false;
        self.bass_engine_enabled = None;
        self.bass_engine_level = None;

        self.supports_hi_res = false;
        self.hi_res_enabled = None;

        self.supports_audio_codec = false;
        self.current_audio_codec = None;
        self.available_codecs.clear();
    }
}

#[derive(Default, Clone, Debug)]
pub struct ControlsState {
    pub supports_touch_customization: bool,
    pub supports_sleep_detection: bool,
    pub supports_head_motion: bool,
    pub supports_find_device: bool,
    pub supports_fit_detection: bool,
    pub supports_prompt_volume: bool,

    pub key_functions: Vec<KeyFunction>,
    pub gesture_rows: Vec<GestureRow>,
    pub noise_cycle: Option<NoiseCycleState>,
    pub prompt_volume: Option<u8>,
    pub head_motion: Option<u8>,
    pub in_ear_detection: Option<bool>,
    pub sleep_detection: Option<bool>,
    pub preferred_device: Option<PairedDeviceAddress>,
}

impl ControlsState {
    pub fn reset_on_disconnect(&mut self) {
        self.supports_touch_customization = false;
        self.supports_sleep_detection = false;
        self.supports_head_motion = false;
        self.supports_find_device = false;
        self.supports_fit_detection = false;
        self.supports_prompt_volume = false;

        self.key_functions.clear();
        self.gesture_rows.clear();
        self.noise_cycle = None;
        self.prompt_volume = None;
        self.head_motion = None;
        self.in_ear_detection = None;
        self.sleep_detection = None;
        self.preferred_device = None;
    }
}

#[derive(Clone, Debug)]
pub struct DualDeviceState {
    pub supports_dual_device: bool,
    pub current_dual_device: Option<bool>,
    pub paired_devices: Result<Vec<PairedDevice>, String>,
    pub controls_enabled: Result<bool, String>,
    pub action_pending: bool,
}

impl Default for DualDeviceState {
    fn default() -> Self {
        Self {
            supports_dual_device: false,
            current_dual_device: None,
            paired_devices: Err(crate::t!("dual-device-read-after-connecting")),
            controls_enabled: Err(crate::t!("dual-device-reading-status")),
            action_pending: false,
        }
    }
}

impl DualDeviceState {
    pub fn reset_on_disconnect(&mut self) {
        self.supports_dual_device = false;
        self.current_dual_device = None;
        self.paired_devices = Err(crate::t!("dual-device-read-after-connecting"));
        self.controls_enabled = Err(crate::t!("dual-device-reading-status"));
        self.action_pending = false;
    }
}
