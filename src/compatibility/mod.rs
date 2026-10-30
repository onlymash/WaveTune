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

pub mod brands;
pub mod registry;

use std::collections::BTreeSet;

#[allow(unused_imports)]
pub use brands::oppo::DEVICE_PROFILES;
#[allow(unused_imports)]
pub use brands::oppo::OPPO_DEVICE_PROFILES;
#[allow(unused_imports)]
pub use registry::builtin_profiles;

pub mod features {
    pub const DUAL_DEVICE: &str = "dual-device";
    pub const WEAR_DETECTION: &str = "wear-detection";
    pub const FIT_DETECTION: &str = "fit-detection";
    pub const EQUALIZER: &str = "equalizer";
    pub const CUSTOM_EQUALIZER: &str = "custom-equalizer";
    pub const GAME_MODE: &str = "game-mode";
    pub const SPATIAL_AUDIO: &str = "spatial-audio";
    pub const HEAD_TRACKING: &str = "head-tracking";
    pub const BASS_ENGINE: &str = "bass-engine";
    pub const FIND_DEVICE: &str = "find-device";
    pub const TOUCH_CUSTOMIZATION: &str = "touch-customization";
    pub const PROMPT_VOLUME: &str = "prompt-volume";
    pub const SLEEP_DETECTION: &str = "sleep-detection";
    pub const SWIFT_PAIR: &str = "swift-pair";
    pub const VOICE_COMMAND: &str = "voice-command";
    pub const HEAD_MOTION: &str = "head-motion";
    pub const HI_RES: &str = "hi-res";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoiseModeKind {
    Off,
    NoiseCancellation,
    Transparency,
    Deep,
    Moderate,
    Mild,
    Smart,
    Adaptive,
    Standard,
    LessNoise,
    MoreNoise,
    VocalEnhancement,
}

impl NoiseModeKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "noise-mode-off",
            Self::NoiseCancellation => "noise-mode-noise-cancellation",
            Self::Transparency => "noise-mode-transparency",
            Self::Deep => "noise-mode-deep",
            Self::Moderate => "noise-mode-moderate",
            Self::Mild => "noise-mode-mild",
            Self::Smart => "noise-mode-smart",
            Self::Adaptive => "noise-mode-adaptive",
            Self::Standard => "noise-mode-standard",
            Self::LessNoise => "noise-mode-less-noise",
            Self::MoreNoise => "noise-mode-more-noise",
            Self::VocalEnhancement => "noise-mode-vocal-enhancement",
        }
    }

    pub const fn key(&self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseMode {
    pub index: u8,
    pub kind: NoiseModeKind,
    pub parent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProfile {
    pub model: &'static str,
    pub product_id: Option<u32>,
    pub uuid: Option<&'static str>,
    pub noise_modes: &'static [NoiseMode],
    pub features: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualizerPreset {
    pub index: u8,
    pub key: &'static str,
}

#[allow(unused_imports)]
pub use brands::earfun::{
    EARFUN_AIR_PRO_4_EQ_PRESETS, EARFUN_AIR_S_EQ_PRESETS, EARFUN_TEN_BAND_EQ_PRESETS,
};
#[allow(unused_imports)]
pub use brands::oppo::{
    DYNAUDIO_EQ_PRESETS, STANDARD_EQ_PRESETS, X3_EQ_PRESETS, game_mode_feature_id_for_product,
};

pub fn equalizer_presets_for_product(product_id: u32) -> &'static [EqualizerPreset] {
    if let Some(presets) = brands::earfun::equalizer_presets_for_product(product_id) {
        presets
    } else if let Some(presets) = brands::oppo::equalizer_presets_for_product(product_id) {
        presets
    } else {
        brands::oppo::STANDARD_EQ_PRESETS
    }
}

pub fn equalizer_preset_key(index: u8, presets: &[EqualizerPreset]) -> Option<&'static str> {
    presets
        .iter()
        .find(|preset| preset.index == index)
        .map(|preset| preset.key)
}

pub fn spatial_audio_modes_for_profile(
    supports_spatial_audio: bool,
    supports_head_tracking: bool,
) -> Vec<(u8, &'static str)> {
    if !supports_spatial_audio {
        return Vec::new();
    }
    let mut modes = vec![(0, "common-off"), (1, "spatial-audio-mode-fixed")];
    if supports_head_tracking {
        modes.push((2, "spatial-audio-mode-head-tracking"));
    }
    modes
}

pub fn spatial_audio_mode_key(mode: u8) -> &'static str {
    match mode {
        1 => "spatial-audio-mode-fixed",
        2 => "spatial-audio-mode-head-tracking",
        _ => "common-off",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GestureRow {
    pub action: crate::protocol::GestureAction,
    pub is_call_control: bool,
    pub button: u8,
    pub both_ears: bool,
    pub edits_noise_cycle: bool,
    pub allowed_functions: &'static [u8],
    pub default_function: Option<u8>,
    pub label_key: &'static str,
}

pub static STANDARD_GESTURE_ROWS: &[GestureRow] = &[
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 1],
        default_function: Some(1),
        label_key: "controls-gesture-single",
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 1, 5, 6, 3, 17],
        default_function: Some(6),
        label_key: "controls-gesture-double",
    },
    GestureRow {
        action: crate::protocol::GestureAction::TripleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 5, 6, 3, 17],
        default_function: Some(5),
        label_key: "controls-gesture-triple",
    },
    GestureRow {
        action: crate::protocol::GestureAction::Slide,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 7, 10],
        default_function: Some(7),
        label_key: "controls-gesture-slide",
    },
    GestureRow {
        action: crate::protocol::GestureAction::SqueezeAndHold,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: true,
        allowed_functions: &[0, 8, 3, 17],
        default_function: Some(8),
        label_key: "controls-gesture-hold",
    },
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: true,
        button: crate::protocol::KeyFunction::CALL_BUTTON,
        both_ears: true,
        edits_noise_cycle: false,
        allowed_functions: &[0, 29],
        default_function: Some(29),
        label_key: "controls-gesture-call-single",
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: true,
        button: crate::protocol::KeyFunction::CALL_BUTTON,
        both_ears: true,
        edits_noise_cycle: false,
        allowed_functions: &[0, 28],
        default_function: Some(28),
        label_key: "controls-gesture-call-double",
    },
];

pub fn gesture_rows_for_product(_product_id: u32) -> &'static [GestureRow] {
    STANDARD_GESTURE_ROWS
}

pub fn gesture_function_label_key(function: u8) -> &'static str {
    match function {
        0 => "controls-function-none",
        1 => "controls-function-play-pause",
        3 => "controls-function-voice-assistant",
        5 => "controls-function-previous",
        6 => "controls-function-next",
        7 => "controls-function-volume-control",
        8 => "controls-function-noise-control",
        10 => "controls-function-switch-track",
        11 => "controls-function-volume-up",
        12 => "controls-function-volume-down",
        13 => "controls-function-switch-devices",
        17 => "controls-function-game-mode",
        28 => "controls-function-decline-end-call",
        29 => "controls-function-answer-end-call",
        _ => "controls-function-unknown",
    }
}

#[allow(dead_code)]
impl DeviceProfile {
    #[inline]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&feature)
    }

    #[inline]
    pub fn supports_noise_control(&self) -> bool {
        !self.noise_modes.is_empty()
    }

    #[inline]
    pub fn supports_dual_device(&self) -> bool {
        self.has_feature(features::DUAL_DEVICE)
    }

    #[inline]
    pub fn supports_wear_detection(&self) -> bool {
        self.has_feature(features::WEAR_DETECTION)
    }

    #[inline]
    pub fn supports_fit_detection(&self) -> bool {
        self.has_feature(features::FIT_DETECTION)
    }

    #[inline]
    pub fn supports_equalizer(&self) -> bool {
        self.has_feature(features::EQUALIZER)
    }

    #[inline]
    pub fn supports_custom_equalizer(&self) -> bool {
        self.has_feature(features::CUSTOM_EQUALIZER)
    }

    #[inline]
    pub fn supports_game_mode(&self) -> bool {
        self.has_feature(features::GAME_MODE)
    }

    #[inline]
    pub fn supports_spatial_audio(&self) -> bool {
        self.has_feature(features::SPATIAL_AUDIO)
    }

    #[inline]
    pub fn supports_head_tracking(&self) -> bool {
        self.has_feature(features::HEAD_TRACKING)
    }

    #[inline]
    pub fn supports_bass_engine(&self) -> bool {
        self.has_feature(features::BASS_ENGINE)
    }

    #[inline]
    pub fn supports_find_device(&self) -> bool {
        self.has_feature(features::FIND_DEVICE)
    }

    #[inline]
    pub fn supports_touch_customization(&self) -> bool {
        self.has_feature(features::TOUCH_CUSTOMIZATION)
    }

    #[inline]
    pub fn supports_prompt_volume(&self) -> bool {
        self.has_feature(features::PROMPT_VOLUME)
    }

    #[inline]
    pub fn supports_sleep_detection(&self) -> bool {
        self.has_feature(features::SLEEP_DETECTION)
    }

    #[inline]
    pub fn supports_swift_pair(&self) -> bool {
        self.has_feature(features::SWIFT_PAIR)
    }

    #[inline]
    pub fn supports_voice_command(&self) -> bool {
        self.has_feature(features::VOICE_COMMAND)
    }

    #[inline]
    pub fn supports_head_motion(&self) -> bool {
        self.has_feature(features::HEAD_MOTION)
    }

    #[inline]
    pub fn supports_hi_res(&self) -> bool {
        self.has_feature(features::HI_RES)
    }

    pub fn supports_controls(&self) -> bool {
        !self.features.is_empty() || !self.noise_modes.is_empty()
    }
}

pub fn profile_for_product_id(product_id: u32) -> Option<&'static DeviceProfile> {
    registry::builtin_profiles().find(|profile| profile.product_id == Some(product_id))
}

pub fn noise_mode_for(modes: &[NoiseMode], index: u8) -> Option<&NoiseMode> {
    modes.iter().find(|mode| mode.index == index)
}

pub fn catalog_noise_indices() -> Vec<u8> {
    registry::builtin_profiles()
        .flat_map(|profile| profile.noise_modes.iter().map(|mode| mode.index))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn fallback_noise_mode_indices_for(modes: &[NoiseMode]) -> Vec<u8> {
    modes.iter().map(|mode| mode.index).collect()
}

pub fn resolve_noise_mode_indices_for_product(
    product_id: u32,
    api_indices: Option<&[u8]>,
) -> (Vec<u8>, bool) {
    let Some(profile) = profile_for_product_id(product_id) else {
        return (Vec::new(), false);
    };
    resolve_noise_mode_indices_for(profile.noise_modes, api_indices)
}

pub fn resolve_noise_mode_indices_for(
    modes: &[NoiseMode],
    api_indices: Option<&[u8]>,
) -> (Vec<u8>, bool) {
    if let Some(api_indices) = api_indices.filter(|indices| !indices.is_empty()) {
        return (
            api_indices
                .iter()
                .copied()
                .filter(|index| noise_mode_for(modes, *index).is_some())
                .collect(),
            true,
        );
    }
    (fallback_noise_mode_indices_for(modes), false)
}

pub fn noise_mode_supported_for_product(
    product_id: u32,
    index: u8,
    supported_indices: &[u8],
) -> bool {
    let Some(profile) = profile_for_product_id(product_id) else {
        return false;
    };
    noise_mode_supported_for(index, supported_indices, profile.noise_modes)
}

pub fn noise_mode_supported_for(index: u8, supported_indices: &[u8], modes: &[NoiseMode]) -> bool {
    let Some(mode) = noise_mode_for(modes, index) else {
        return false;
    };
    if mode.parent.is_some() {
        return supported_indices.contains(&index);
    }
    modes
        .iter()
        .any(|child| child.parent == Some(index) && supported_indices.contains(&child.index))
        || supported_indices.contains(&index)
}

pub fn noise_mode_group_for(modes: &[NoiseMode], index: u8) -> Option<u8> {
    let mode = noise_mode_for(modes, index)?;
    Some(mode.parent.unwrap_or(mode.index))
}

pub fn noise_mode_readback_matches_for(requested: u8, actual: u8, modes: &[NoiseMode]) -> bool {
    requested == actual
        || noise_mode_for(modes, requested).is_some_and(|mode| mode.parent.is_none())
            && noise_mode_for(modes, actual).is_some_and(|mode| mode.parent == Some(requested))
        || noise_mode_for(modes, requested).is_some_and(|mode| mode.parent == Some(actual))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_supported_oppo_oneplus_and_realme_profiles() {
        assert_eq!(DEVICE_PROFILES.len(), 131);
        let product_ids: BTreeSet<_> = DEVICE_PROFILES
            .iter()
            .filter_map(|profile| profile.product_id)
            .collect();
        assert_eq!(product_ids.len(), DEVICE_PROFILES.len());
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| !profile.noise_modes.is_empty())
                .count(),
            43
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_dual_device())
                .count(),
            51
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_wear_detection())
                .count(),
            49
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_fit_detection())
                .count(),
            31
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_equalizer())
                .count(),
            74
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_custom_equalizer())
                .count(),
            37
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_game_mode())
                .count(),
            69
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_spatial_audio())
                .count(),
            48
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_head_tracking())
                .count(),
            4
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_bass_engine())
                .count(),
            24
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_find_device())
                .count(),
            93
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_touch_customization())
                .count(),
            84
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_prompt_volume())
                .count(),
            27
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_sleep_detection())
                .count(),
            2
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_swift_pair())
                .count(),
            12
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_voice_command())
                .count(),
            5
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_head_motion())
                .count(),
            2
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_hi_res())
                .count(),
            28
        );
        for profile in DEVICE_PROFILES {
            let indices: BTreeSet<_> = profile.noise_modes.iter().map(|mode| mode.index).collect();
            assert_eq!(
                indices.len(),
                profile.noise_modes.len(),
                "{}",
                profile.model
            );
            for mode in profile.noise_modes {
                if let Some(parent) = mode.parent {
                    assert!(
                        indices.contains(&parent),
                        "{} mode {}",
                        profile.model,
                        mode.index
                    );
                }
            }
        }
        assert!(
            DEVICE_PROFILES
                .iter()
                .any(|profile| profile.model == "OPPO Enco X3")
        );
        assert!(
            DEVICE_PROFILES
                .iter()
                .any(|profile| profile.model == "OnePlus Buds Pro 2")
        );
        assert!(
            DEVICE_PROFILES
                .iter()
                .any(|profile| profile.model == "realme Buds Air 5 Pro")
        );
    }

    #[test]
    fn x3_compatibility_profile_enables_declared_noise_and_dual_device_controls() {
        let profile = profile_for_product_id(0x067410).unwrap();
        assert_eq!(profile.model, "OPPO Enco X3");
        assert!(profile.supports_controls());
        assert!(profile.supports_dual_device());
        assert!(profile.supports_wear_detection());
        assert!(profile.supports_fit_detection());
        assert!(profile.supports_equalizer());
        assert!(!profile.supports_custom_equalizer());
        assert!(profile.supports_game_mode());
        assert!(profile.supports_spatial_audio());
        assert!(profile.supports_head_tracking());
        assert!(!profile.supports_bass_engine());
        assert!(profile.supports_find_device());
        assert!(profile.supports_touch_customization());
        assert!(!profile.supports_prompt_volume());
        assert!(!profile.supports_sleep_detection());
        assert!(!profile.supports_swift_pair());
        assert!(!profile.supports_voice_command());
        assert!(!profile.supports_head_motion());
        assert!(profile.supports_hi_res());
        assert!(noise_mode_supported_for(1, &[4, 7], profile.noise_modes));
        assert!(!noise_mode_supported_for(2, &[4, 7], profile.noise_modes));
        let voice_mode = noise_mode_for(profile.noise_modes, 9).unwrap();
        assert_eq!(voice_mode.kind, NoiseModeKind::VocalEnhancement);
        assert!(noise_mode_readback_matches_for(9, 9, profile.noise_modes));
        assert!(noise_mode_readback_matches_for(9, 2, profile.noise_modes));
    }

    #[test]
    fn x4_compatibility_profile_enables_all_expected_capabilities() {
        let profile = profile_for_product_id(0x06ec10).unwrap();
        assert_eq!(profile.model, "OPPO Enco X4");
        assert!(profile.supports_controls());
        assert!(profile.supports_dual_device());
        assert!(profile.supports_wear_detection());
        assert!(profile.supports_fit_detection());
        assert!(profile.supports_equalizer());
        assert!(profile.supports_custom_equalizer());
        assert!(profile.supports_game_mode());
        assert!(profile.supports_spatial_audio());
        assert!(profile.supports_head_tracking());
        assert!(profile.supports_bass_engine());
        assert!(profile.supports_find_device());
        assert!(profile.supports_touch_customization());
        assert!(profile.supports_prompt_volume());
        assert!(profile.supports_sleep_detection());
        assert!(profile.supports_swift_pair());
        assert!(profile.supports_voice_command());
        assert!(profile.supports_head_motion());
        assert!(profile.supports_hi_res());
    }

    #[test]
    fn empty_api_support_list_falls_back_to_the_model_tree() {
        let profile = profile_for_product_id(0x06ec10).unwrap();
        let (indices, from_api) = resolve_noise_mode_indices_for(profile.noise_modes, Some(&[]));
        assert!(!from_api);
        assert!(indices.contains(&1));
        assert!(indices.contains(&11));
        assert!(indices.contains(&14));
    }

    #[test]
    fn sound_features_resolve_correctly_for_profiles() {
        let x4_presets = equalizer_presets_for_product(0x06ec10);
        assert_eq!(x4_presets.len(), 5);
        assert_eq!(x4_presets[0].key, "equalizer-preset-dynaudio-classic");
        assert_eq!(
            equalizer_preset_key(7, x4_presets),
            Some("equalizer-preset-dynaudio-hi-res")
        );
        assert_eq!(equalizer_preset_key(99, x4_presets), None);

        let x3_presets = equalizer_presets_for_product(0x067410);
        assert_eq!(x3_presets.len(), 5);
        assert_eq!(x3_presets[0].key, "equalizer-preset-ultimate-sound");
        assert_eq!(
            equalizer_preset_key(1, x3_presets),
            Some("equalizer-preset-hd-clarity")
        );
        assert_eq!(
            equalizer_preset_key(7, x3_presets),
            Some("equalizer-preset-dynaudio-tuning")
        );

        let std_presets = equalizer_presets_for_product(0x064810);
        assert_eq!(std_presets.len(), 4);
        assert_eq!(std_presets[0].key, "equalizer-preset-classic");

        let spatial_with_head = spatial_audio_modes_for_profile(true, true);
        assert_eq!(spatial_with_head.len(), 3);
        assert_eq!(
            spatial_with_head[2],
            (2, "spatial-audio-mode-head-tracking")
        );

        let spatial_no_head = spatial_audio_modes_for_profile(true, false);
        assert_eq!(spatial_no_head.len(), 2);

        assert_eq!(spatial_audio_mode_key(0), "common-off");
        assert_eq!(spatial_audio_mode_key(1), "spatial-audio-mode-fixed");
        assert_eq!(
            spatial_audio_mode_key(2),
            "spatial-audio-mode-head-tracking"
        );

        assert_eq!(game_mode_feature_id_for_product(0x06ec10), 0x28);
        assert_eq!(game_mode_feature_id_for_product(0x067410), 0x06);
    }
}
