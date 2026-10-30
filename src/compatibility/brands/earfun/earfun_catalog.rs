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

//! EarFun device catalog and profiles.

use crate::compatibility::{DeviceProfile, NoiseMode, NoiseModeKind, features};
use crate::protocol::brands::earfun::EarFunAudioCodec;

#[allow(unused_imports)]
pub use super::earfun_equalizer::{
    EARFUN_AIR_PRO_4_EQ_PRESETS, EARFUN_AIR_S_EQ_PRESETS, EARFUN_TEN_BAND_EQ_PRESETS,
};

pub static EARFUN_AIR_PRO_4_CODECS: &[EarFunAudioCodec] = &[
    EarFunAudioCodec::StableConnection,
    EarFunAudioCodec::AptX,
    EarFunAudioCodec::AptXAdaptive,
    EarFunAudioCodec::Ldac,
    EarFunAudioCodec::AptXLossless,
];

pub static EARFUN_FREE_PRO_3_CODECS: &[EarFunAudioCodec] = &[
    EarFunAudioCodec::StableConnection,
    EarFunAudioCodec::AptX,
    EarFunAudioCodec::AptXAdaptive,
];

pub fn supported_earfun_codecs(product_id: u32) -> &'static [EarFunAudioCodec] {
    match product_id {
        0x004535 => EARFUN_AIR_PRO_4_CODECS,
        0x004534 => EARFUN_FREE_PRO_3_CODECS,
        _ => &[],
    }
}

/// 3 basic noise modes supported by EarFun Air S (Off, ANC, Transparency)
pub static EARFUN_AIR_S_NOISE_MODES: &[NoiseMode] = &[
    NoiseMode {
        index: 0,
        kind: NoiseModeKind::Off,
        parent: None,
    },
    NoiseMode {
        index: 1,
        kind: NoiseModeKind::NoiseCancellation,
        parent: None,
    },
    NoiseMode {
        index: 2,
        kind: NoiseModeKind::Transparency,
        parent: None,
    },
];

/// Noise modes supported by EarFun Air Pro 4 and Free Pro 3
/// Mapped protocol indices:
/// 0: Off
/// 1: ANC (Top-level)
/// 2: Transparency (Top-level)
/// 3: ANC - AI Environment Adaptive (sub-mode 4)
/// 4: ANC - Strong (sub-mode 2)
/// 5: ANC - Balanced (sub-mode 3)
/// 6: ANC - Wind Noise Cancelling (sub-mode 1)
/// 7: ANC - AI Ear Adaptive (sub-mode 0)
/// 8: Transparency - Default (sub-mode 0)
/// 9: Transparency - Vocal/Natural (sub-mode 1)
pub static EARFUN_AIR_PRO_4_NOISE_MODES: &[NoiseMode] = &[
    NoiseMode {
        index: 0,
        kind: NoiseModeKind::Off,
        parent: None,
    },
    NoiseMode {
        index: 1,
        kind: NoiseModeKind::NoiseCancellation,
        parent: None,
    },
    NoiseMode {
        index: 2,
        kind: NoiseModeKind::Transparency,
        parent: None,
    },
    NoiseMode {
        index: 3,
        kind: NoiseModeKind::Adaptive,
        parent: Some(1),
    },
    NoiseMode {
        index: 4,
        kind: NoiseModeKind::Deep,
        parent: Some(1),
    },
    NoiseMode {
        index: 5,
        kind: NoiseModeKind::Moderate,
        parent: Some(1),
    },
    NoiseMode {
        index: 6,
        kind: NoiseModeKind::Mild,
        parent: Some(1),
    },
    NoiseMode {
        index: 7,
        kind: NoiseModeKind::Smart,
        parent: Some(1),
    },
    NoiseMode {
        index: 8,
        kind: NoiseModeKind::Standard,
        parent: Some(2),
    },
    NoiseMode {
        index: 9,
        kind: NoiseModeKind::VocalEnhancement,
        parent: Some(2),
    },
];

pub static EARFUN_DEVICE_PROFILES: &[DeviceProfile] = &[
    DeviceProfile {
        model: "EarFun Air Pro 4",
        // Product ID 0x4535 ("E5")
        product_id: Some(0x004535),
        uuid: Some("00001107-D102-11E1-9B23-00025B00A5A5"),
        noise_modes: EARFUN_AIR_PRO_4_NOISE_MODES,
        features: &[
            features::WEAR_DETECTION,
            features::DUAL_DEVICE,
            features::GAME_MODE,
            features::EQUALIZER,
            features::CUSTOM_EQUALIZER,
            features::TOUCH_CUSTOMIZATION,
            features::FIND_DEVICE,
            features::PROMPT_VOLUME,
            features::AUDIO_CODEC,
        ],
    },
    DeviceProfile {
        model: "EarFun Free Pro 3",
        // Product ID 0x4534 ("E4")
        product_id: Some(0x004534),
        uuid: Some("00001107-D102-11E1-9B23-00025B00A5A5"),
        noise_modes: EARFUN_AIR_PRO_4_NOISE_MODES,
        features: &[
            features::DUAL_DEVICE,
            features::GAME_MODE,
            features::EQUALIZER,
            features::CUSTOM_EQUALIZER,
            features::TOUCH_CUSTOMIZATION,
            features::FIND_DEVICE,
            features::PROMPT_VOLUME,
            features::AUDIO_CODEC,
        ],
    },
    DeviceProfile {
        model: "EarFun Air S",
        // Product ID 0x4c45 ("LE")
        product_id: Some(0x004c45),
        uuid: Some("00001107-D102-11E1-9B23-00025B00A5A5"),
        noise_modes: EARFUN_AIR_S_NOISE_MODES,
        features: &[
            features::DUAL_DEVICE,
            features::GAME_MODE,
            features::EQUALIZER,
            features::CUSTOM_EQUALIZER,
            features::TOUCH_CUSTOMIZATION,
        ],
    },
];
