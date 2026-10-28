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

pub use brands::oppo::DEVICE_PROFILES;
#[allow(unused_imports)]
pub use brands::oppo::OPPO_DEVICE_PROFILES;
#[allow(unused_imports)]
pub use registry::builtin_profiles;

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
    pub write_enabled: bool,
    pub device_reported_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProfile {
    pub model: &'static str,
    pub product_id: Option<u32>,
    pub uuid: Option<&'static str>,
    pub supports_dual_device: bool,
    pub noise_modes: &'static [NoiseMode],
    pub supports_wear_detection: bool, // 入耳/佩戴检测 (wearDetection)
    pub supports_fit_detection: bool,  // 耳塞贴合度检测 (fitDetection)
    pub supports_equalizer: bool,      // 内置 EQ 预设 (equalizer)
    pub supports_custom_equalizer: bool, // 自定义均衡器 (customEqualizer)
    pub supports_game_mode: bool,      // 游戏模式/低延迟 (gameMode)
    pub supports_spatial_audio: bool,  // 空间音频 (spatialTypes)
    pub supports_head_tracking: bool,  // 头部跟踪 (spatialTypes 包含 2)
    pub supports_bass_engine: bool,    // BassWave 动态低音 (bassEngineSupport)
    pub supports_find_device: bool,    // 查找耳机 (findDevice / heyFindDevice)
    pub supports_touch_customization: bool, // 触控/手势自定义 (control / callControl)
    pub supports_prompt_volume: bool,  // 提示音音量 (promptVolume)
    pub supports_sleep_detection: bool, // 入睡检测 (sleepDetection)
    pub supports_swift_pair: bool,     // Swift Pair (swiftPair)
    pub supports_voice_command: bool,  // 快捷语音指令 (voiceCommand)
    pub supports_head_motion: bool,    // 头部动作接听 (headMotion)
    pub supports_hi_res: bool,         // 高解析模式 (highToneQuality)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualizerPreset {
    pub index: u8,
    pub key: &'static str,
}

pub static DYNAUDIO_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-dynaudio-classic",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-dynaudio-hi-res",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-clear-treble",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-bass-boost",
    },
];

pub static STANDARD_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-classic",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-dynamic-bass",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-clear-vocal",
    },
];

pub static X3_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-ultimate-sound",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-hd-clarity",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-bass-boost",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-dynaudio-tuning",
    },
];

pub fn equalizer_presets_for_product(product_id: u32) -> &'static [EqualizerPreset] {
    if product_id == 0x06ec10 {
        DYNAUDIO_EQ_PRESETS
    } else if product_id == 0x067410 {
        X3_EQ_PRESETS
    } else {
        STANDARD_EQ_PRESETS
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

pub fn game_mode_feature_id_for_product(product_id: u32) -> u8 {
    if product_id == 0x06ec10 { 0x28 } else { 0x06 }
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

impl DeviceProfile {
    pub fn supports_controls(&self) -> bool {
        self.supports_dual_device
            || self.noise_modes.iter().any(|mode| mode.write_enabled)
            || self.supports_equalizer
            || self.supports_spatial_audio
            || self.supports_game_mode
            || self.supports_touch_customization
            || self.supports_wear_detection
            || self.supports_find_device
            || self.supports_fit_detection
            || self.supports_prompt_volume
            || self.supports_head_motion
    }
}

pub fn profile_for_product_id(product_id: u32) -> Option<&'static DeviceProfile> {
    DEVICE_PROFILES
        .iter()
        .find(|profile| profile.product_id == Some(product_id))
}

pub fn noise_mode_for(modes: &[NoiseMode], index: u8) -> Option<&NoiseMode> {
    modes.iter().find(|mode| mode.index == index)
}

pub fn catalog_noise_indices() -> Vec<u8> {
    DEVICE_PROFILES
        .iter()
        .flat_map(|profile| profile.noise_modes.iter().map(|mode| mode.index))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn fallback_noise_mode_indices_for(modes: &[NoiseMode]) -> Vec<u8> {
    modes
        .iter()
        .filter(|mode| !mode.device_reported_only)
        .map(|mode| mode.index)
        .collect()
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
    if mode.device_reported_only && !supported_indices.contains(&index) {
        return false;
    }
    if mode.parent.is_some() {
        return supported_indices.contains(&index);
    }
    modes.iter().any(|child| {
        child.parent == Some(index)
            && (!child.device_reported_only || supported_indices.contains(&child.index))
            && supported_indices.contains(&child.index)
    }) || supported_indices.contains(&index)
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
                .filter(|profile| profile.supports_dual_device)
                .count(),
            51
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_wear_detection)
                .count(),
            49
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_fit_detection)
                .count(),
            31
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_equalizer)
                .count(),
            74
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_custom_equalizer)
                .count(),
            37
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_game_mode)
                .count(),
            69
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_spatial_audio)
                .count(),
            48
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_head_tracking)
                .count(),
            4
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_bass_engine)
                .count(),
            24
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_find_device)
                .count(),
            93
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_touch_customization)
                .count(),
            84
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_prompt_volume)
                .count(),
            27
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_sleep_detection)
                .count(),
            2
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_swift_pair)
                .count(),
            12
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_voice_command)
                .count(),
            5
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_head_motion)
                .count(),
            2
        );
        assert_eq!(
            DEVICE_PROFILES
                .iter()
                .filter(|profile| profile.supports_hi_res)
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
        assert!(profile.supports_dual_device);
        assert!(profile.supports_wear_detection);
        assert!(profile.supports_fit_detection);
        assert!(profile.supports_equalizer);
        assert!(!profile.supports_custom_equalizer);
        assert!(profile.supports_game_mode);
        assert!(profile.supports_spatial_audio);
        assert!(profile.supports_head_tracking);
        assert!(!profile.supports_bass_engine);
        assert!(profile.supports_find_device);
        assert!(profile.supports_touch_customization);
        assert!(!profile.supports_prompt_volume);
        assert!(!profile.supports_sleep_detection);
        assert!(!profile.supports_swift_pair);
        assert!(!profile.supports_voice_command);
        assert!(!profile.supports_head_motion);
        assert!(profile.supports_hi_res);
        assert!(noise_mode_supported_for(1, &[4, 7], profile.noise_modes));
        assert!(!noise_mode_supported_for(2, &[4, 7], profile.noise_modes));
        let voice_mode = noise_mode_for(profile.noise_modes, 9).unwrap();
        assert_eq!(voice_mode.kind, NoiseModeKind::VocalEnhancement);
        assert!(voice_mode.write_enabled);
        assert!(noise_mode_readback_matches_for(9, 9, profile.noise_modes));
        assert!(noise_mode_readback_matches_for(9, 2, profile.noise_modes));
    }

    #[test]
    fn x4_compatibility_profile_enables_all_expected_capabilities() {
        let profile = profile_for_product_id(0x06ec10).unwrap();
        assert_eq!(profile.model, "OPPO Enco X4");
        assert!(profile.supports_controls());
        assert!(profile.supports_dual_device);
        assert!(profile.supports_wear_detection);
        assert!(profile.supports_fit_detection);
        assert!(profile.supports_equalizer);
        assert!(profile.supports_custom_equalizer);
        assert!(profile.supports_game_mode);
        assert!(profile.supports_spatial_audio);
        assert!(profile.supports_head_tracking);
        assert!(profile.supports_bass_engine);
        assert!(profile.supports_find_device);
        assert!(profile.supports_touch_customization);
        assert!(profile.supports_prompt_volume);
        assert!(profile.supports_sleep_detection);
        assert!(profile.supports_swift_pair);
        assert!(profile.supports_voice_command);
        assert!(profile.supports_head_motion);
        assert!(profile.supports_hi_res);
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
    fn device_reported_modes_require_a_matching_runtime_support_index() {
        let profile = DEVICE_PROFILES
            .iter()
            .find(|profile| {
                profile
                    .noise_modes
                    .iter()
                    .any(|mode| mode.device_reported_only)
            })
            .unwrap();
        let mode = profile
            .noise_modes
            .iter()
            .find(|mode| mode.device_reported_only)
            .unwrap();
        let group = mode.parent.unwrap();
        assert!(!noise_mode_supported_for(group, &[], profile.noise_modes));
        assert!(noise_mode_supported_for(
            group,
            &[mode.index],
            profile.noise_modes
        ));
        assert!(!fallback_noise_mode_indices_for(profile.noise_modes).contains(&mode.index));
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
