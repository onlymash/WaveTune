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
    bluetooth::DeviceSnapshot,
    compatibility::{EqualizerPreset, GestureRow, NoiseMode},
    protocol::{
        BassEngineLevel, BatteryState, CustomEqualizer, FindEarbudsMode, KeyFunction,
        NoiseCycleState, PairedDevice, PairedDeviceAddress, WearState,
    },
};

/// Unified state for a headset control session.
#[derive(Clone, Debug)]
pub struct HeadsetSessionState {
    pub connected: bool,
    pub device_name: String,
    pub product_id: Option<u32>,
    pub firmware_model_code: Option<String>,
    pub artwork_path: Option<PathBuf>,
    pub battery: BatteryState,
    pub case_battery_is_stale: bool,
    pub wear: WearState,
    pub supports_wear_status: bool,
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
            firmware_model_code: None,
            artwork_path: None,
            battery: BatteryState::default(),
            case_battery_is_stale: false,
            wear: WearState::default(),
            supports_wear_status: false,
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
        self.supports_wear_status = false;
        self.supports_wear_detection = true;
        self.product_id = None;
        self.firmware_model_code = None;
        self.artwork_path = None;

        self.noise.reset_on_disconnect();
        self.sound.reset_on_disconnect();
        self.controls.reset_on_disconnect();
        self.dual_device.reset_on_disconnect();
    }

    /// Atomically apply an initial device snapshot to the session state.
    pub fn apply_initial_snapshot(&mut self, snapshot: &DeviceSnapshot) {
        self.connected = true;
        self.device_name = snapshot.identity.name.clone();
        self.product_id = Some(snapshot.identity.product_id);
        self.firmware_model_code = snapshot.identity.firmware_model_code.clone();
        self.battery = snapshot.battery.clone();
        self.case_battery_is_stale = false;
        self.wear = snapshot.wear.wear.clone();
        self.supports_wear_status = snapshot.wear.supports_wear_status;
        self.supports_wear_detection = snapshot.wear.supports_wear_detection;

        // Noise
        self.noise.enabled = !snapshot.identity.read_only && !snapshot.noise.modes.is_empty();
        self.noise.current_mode = snapshot.noise.current_mode;
        if let Some(mode) = snapshot.noise.current_mode {
            if mode != 0 {
                self.noise.last_non_off_mode = Some(mode);
            }
        }
        self.noise.supported_indices = snapshot.noise.supported_indices.clone();
        self.noise.modes = snapshot.noise.modes.to_vec();
        self.noise.support_note = snapshot.noise.support_note.clone();

        // Sound
        self.sound.supports_eq = snapshot.sound.supports_equalizer;
        self.sound.supports_custom_eq = snapshot.sound.supports_custom_equalizer;
        self.sound.current_eq_preset = snapshot.sound.equalizer_preset;
        self.sound.presets = snapshot.sound.presets.clone();
        self.sound.custom_equalizers = snapshot.sound.custom_equalizers.clone();

        self.sound.supports_spatial = snapshot.sound.supports_spatial_audio;
        self.sound.supports_head_tracking = snapshot.sound.supports_head_tracking;
        self.sound.current_spatial_mode = snapshot.sound.spatial_audio_mode;

        self.sound.supports_game_mode = snapshot.sound.supports_game_mode;
        self.sound.current_game_mode = snapshot.sound.game_mode_enabled;

        self.sound.supports_bass_engine = snapshot.sound.supports_bass_engine;
        self.sound.bass_engine_enabled = snapshot.sound.bass_engine_enabled;
        self.sound.bass_engine_level = snapshot.sound.bass_engine_level;

        self.sound.supports_hi_res = snapshot.sound.supports_hi_res;
        self.sound.hi_res_enabled = snapshot.sound.hi_res_enabled;

        self.sound.supports_audio_codec = snapshot.sound.supports_audio_codec;
        self.sound.current_audio_codec = snapshot.sound.audio_codec;
        self.sound.available_codecs = snapshot.sound.available_codecs.clone();

        // Controls
        self.controls.supports_touch_customization = snapshot.controls.supports_touch_customization;
        self.controls.supports_touch_lock = snapshot.controls.supports_touch_lock;
        self.controls.supports_sleep_detection = snapshot.controls.supports_sleep_detection;
        self.controls.supports_head_motion = snapshot.controls.supports_head_motion;
        self.controls.supports_find_device = snapshot.controls.supports_find_device;
        self.controls.supports_find_device_sides = snapshot.controls.supports_find_device_sides;
        self.controls.supports_fit_detection = snapshot.controls.supports_fit_detection;
        self.controls.supports_prompt_volume = snapshot.controls.supports_prompt_volume;
        self.controls.supports_microphone_mode = snapshot.controls.supports_microphone_mode;

        self.controls.key_functions = snapshot.controls.key_functions.clone();
        self.controls.gesture_rows = snapshot.controls.gesture_rows.clone();
        self.controls.noise_cycle = Some(snapshot.controls.noise_cycle);
        self.controls.prompt_volume = snapshot.controls.prompt_volume;
        self.controls.prompt_volume_range = snapshot.controls.prompt_volume_range;
        self.controls.microphone_mode = snapshot.controls.microphone_mode;
        self.controls.is_finding_earbuds = snapshot.controls.is_finding_earbuds;
        self.controls.find_earbuds_mode = snapshot.controls.find_earbuds_mode;
        self.controls.head_motion = snapshot.controls.head_motion_type;
        self.controls.in_ear_detection = snapshot.controls.in_ear_detection_enabled;
        self.controls.touch_lock_enabled = snapshot.controls.touch_lock_enabled;
        self.controls.sleep_detection = snapshot.controls.sleep_detection_enabled;
        self.controls.preferred_device = snapshot.dual_device.preferred_device;

        // Dual device
        self.dual_device.supports_dual_device = snapshot.dual_device.supports_dual_device;
        self.dual_device.current_dual_device = snapshot.dual_device.dual_device_enabled;
        self.dual_device.paired_devices = snapshot.dual_device.paired_devices.clone();
        self.dual_device.controls_enabled = snapshot.dual_device.controls_enabled.clone();
    }
}

#[derive(Clone, Debug)]
pub struct NoiseState {
    pub enabled: bool,
    pub current_mode: Option<u8>,
    pub pending_mode: Option<u8>,
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
            pending_mode: None,
            last_non_off_mode: None,
            supported_indices: Vec::new(),
            modes: Vec::new(),
            support_note: crate::t!("noise-mode-read-support-after-connecting"),
        }
    }
}

impl NoiseState {
    pub fn observe_mode(&mut self, mode: u8) -> bool {
        if self.pending_mode.is_some() {
            return false;
        }
        self.current_mode = Some(mode);
        true
    }

    pub fn complete_write(&mut self, mode: u8) {
        self.current_mode = Some(mode);
        self.pending_mode = None;
    }

    pub fn reset_on_disconnect(&mut self) {
        self.enabled = false;
        self.current_mode = None;
        self.pending_mode = None;
    }
}

#[cfg(test)]
mod tests {
    use super::NoiseState;

    #[test]
    fn stale_mode_observation_does_not_clear_pending_mode() {
        let mut state = NoiseState {
            current_mode: Some(0),
            pending_mode: Some(1),
            ..NoiseState::default()
        };

        assert!(!state.observe_mode(0));
        assert_eq!(state.current_mode, Some(0));
        assert_eq!(state.pending_mode, Some(1));

        state.complete_write(4);
        assert_eq!(state.current_mode, Some(4));
        assert_eq!(state.pending_mode, None);
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
    pub supports_touch_lock: bool,
    pub supports_sleep_detection: bool,
    pub supports_head_motion: bool,
    pub supports_find_device: bool,
    pub supports_find_device_sides: bool,
    pub supports_fit_detection: bool,
    pub supports_prompt_volume: bool,
    pub supports_microphone_mode: bool,

    pub key_functions: Vec<KeyFunction>,
    pub gesture_rows: Vec<GestureRow>,
    pub noise_cycle: Option<NoiseCycleState>,
    pub prompt_volume: Option<u8>,
    pub prompt_volume_range: Option<(u8, u8)>,
    pub microphone_mode: Option<u8>,
    pub is_finding_earbuds: bool,
    pub find_earbuds_mode: Option<FindEarbudsMode>,
    pub head_motion: Option<u8>,
    pub in_ear_detection: Option<bool>,
    pub touch_lock_enabled: Option<bool>,
    pub sleep_detection: Option<bool>,
    pub preferred_device: Option<PairedDeviceAddress>,
}

impl ControlsState {
    pub fn reset_on_disconnect(&mut self) {
        self.supports_touch_customization = false;
        self.supports_touch_lock = false;
        self.supports_sleep_detection = false;
        self.supports_head_motion = false;
        self.supports_find_device = false;
        self.supports_find_device_sides = false;
        self.supports_fit_detection = false;
        self.supports_prompt_volume = false;
        self.supports_microphone_mode = false;

        self.key_functions.clear();
        self.gesture_rows.clear();
        self.noise_cycle = None;
        self.prompt_volume = None;
        self.prompt_volume_range = None;
        self.microphone_mode = None;
        self.is_finding_earbuds = false;
        self.find_earbuds_mode = None;
        self.head_motion = None;
        self.in_ear_detection = None;
        self.touch_lock_enabled = None;
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
