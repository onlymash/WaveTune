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

use crate::{
    app::events::UiUpdate,
    compatibility::{DeviceProfile, NoiseMode},
    protocol::{
        BassEngineLevel, CustomEqualizer, FrameDecoder, KeyFunction, NoiseCycleState, PairedDevice,
        PairedDeviceAddress, WearState,
    },
};

pub mod controls;
pub mod dual_device;
pub mod noise;
pub mod sound;

/// Execution context passed to OPPO feature handlers.
pub struct OppoFeatureContext<'a, T: Read + Write> {
    pub stream: &'a mut T,
    pub decoder: &'a mut FrameDecoder,
    pub updates: &'a Sender<UiUpdate>,
    pub read_only: bool,
    pub profile: Option<&'static DeviceProfile>,
    pub supported_indices: &'a [u8],
    pub supports_wear_detection: bool,
    pub wear: &'a mut WearState,
}

impl<'a, T: Read + Write> OppoFeatureContext<'a, T> {
    #[inline]
    pub fn noise_modes(&self) -> &'static [NoiseMode] {
        self.profile.map_or(&[], |p| p.noise_modes)
    }

    #[inline]
    pub fn supports_touch_customization(&self) -> bool {
        self.profile
            .is_some_and(|p| p.supports_touch_customization())
    }

    #[inline]
    pub fn supports_equalizer(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_equalizer())
    }

    #[inline]
    pub fn supports_custom_equalizer(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_custom_equalizer())
    }

    #[inline]
    pub fn supports_spatial_audio(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_spatial_audio())
    }

    #[inline]
    pub fn supports_game_mode(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_game_mode())
    }

    #[inline]
    pub fn supports_bass_engine(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_bass_engine())
    }

    #[inline]
    pub fn supports_hi_res(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_hi_res())
    }

    #[inline]
    pub fn supports_dual_device(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_dual_device())
    }

    #[inline]
    pub fn supports_find_device(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_find_device())
    }

    #[inline]
    pub fn supports_fit_detection(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_fit_detection())
    }

    #[inline]
    pub fn supports_prompt_volume(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_prompt_volume())
    }

    #[inline]
    pub fn supports_head_motion(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_head_motion())
    }

    #[inline]
    pub fn supports_sleep_detection(&self) -> bool {
        self.profile.is_some_and(|p| p.supports_sleep_detection())
    }
}

/// State managed by OPPO controls & gestures feature handlers.
#[derive(Default)]
pub struct OppoControlsState {
    pub key_functions: Vec<KeyFunction>,
    pub noise_cycle: NoiseCycleState,
    pub is_finding_earbuds: bool,
    pub prompt_volume: Option<u8>,
    pub head_motion: Option<u8>,
    pub in_ear_detection: Option<bool>,
    pub sleep_detection: Option<bool>,
}

/// State managed by OPPO sound, equalizer, and audio feature handlers.
pub struct OppoSoundState {
    pub custom_equalizers: Vec<CustomEqualizer>,
    pub bass_engine_level: Option<BassEngineLevel>,
    pub game_mode_feature_id: u8,
}

impl Default for OppoSoundState {
    fn default() -> Self {
        Self {
            custom_equalizers: Vec::new(),
            bass_engine_level: None,
            game_mode_feature_id: 0x06,
        }
    }
}

/// State managed by OPPO dual-device and multipoint connection feature handlers.
pub struct OppoDualDeviceState {
    pub dual_device_enabled: Option<bool>,
    pub preferred_device: Option<PairedDeviceAddress>,
    pub paired_devices: Result<Vec<PairedDevice>, String>,
    pub controls_enabled: Result<bool, String>,
}

impl Default for OppoDualDeviceState {
    fn default() -> Self {
        Self {
            dual_device_enabled: None,
            preferred_device: None,
            paired_devices: Ok(Vec::new()),
            controls_enabled: Ok(false),
        }
    }
}
