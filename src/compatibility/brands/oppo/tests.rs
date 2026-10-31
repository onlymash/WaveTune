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

use std::collections::BTreeSet;

use super::*;
use crate::compatibility::{
    NoiseModeKind, equalizer_preset_key,
    equalizer_presets_for_product as global_equalizer_presets_for_product, noise_mode_for,
    noise_mode_readback_matches_for, noise_mode_supported_for, profile_for_product_id,
};

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
    assert!(noise_mode_readback_matches_for(0, 3, profile.noise_modes));
    assert!(noise_mode_readback_matches_for(1, 7, profile.noise_modes));
    assert!(noise_mode_readback_matches_for(2, 8, profile.noise_modes));
    assert!(noise_mode_readback_matches_for(4, 4, profile.noise_modes));
    assert!(!noise_mode_readback_matches_for(4, 6, profile.noise_modes));
    assert!(noise_mode_readback_matches_for(11, 13, profile.noise_modes));
}

#[test]
fn oppo_sound_features_resolve_correctly_for_profiles() {
    let x4_presets = global_equalizer_presets_for_product(0x06ec10);
    assert_eq!(x4_presets.len(), 5);
    assert_eq!(x4_presets[0].key, "equalizer-preset-dynaudio-classic");
    assert_eq!(
        equalizer_preset_key(7, x4_presets),
        Some("equalizer-preset-dynaudio-hi-res")
    );
    assert_eq!(equalizer_preset_key(99, x4_presets), None);

    let x3_presets = global_equalizer_presets_for_product(0x067410);
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

    let std_presets = global_equalizer_presets_for_product(0x064810);
    assert_eq!(std_presets.len(), 4);
    assert_eq!(std_presets[0].key, "equalizer-preset-classic");

    assert_eq!(game_mode_feature_id_for_product(0x06ec10), 0x28);
    assert_eq!(game_mode_feature_id_for_product(0x067410), 0x06);
}
