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

use super::{earfun_equalizer::*, earfun_gestures::*};
use crate::compatibility::{
    gesture_function_label_key_for_product, gesture_rows_for_product as global_gesture_rows,
    profile_for_identity, profile_for_product_id,
};
use crate::protocol::GestureAction;

#[test]
fn test_earfun_gesture_rows_per_product() {
    // Air Pro 4
    let pro4_rows = gesture_rows_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID).unwrap();
    assert_eq!(pro4_rows.len(), 4);
    assert_eq!(pro4_rows[0].action, GestureAction::SingleSqueeze);
    assert_eq!(pro4_rows[1].action, GestureAction::DoubleSqueeze);
    assert_eq!(pro4_rows[2].action, GestureAction::TripleSqueeze);
    assert_eq!(pro4_rows[3].action, GestureAction::SqueezeAndHold);

    for row in pro4_rows {
        assert!(!row.is_call_control);
        assert!(!row.edits_noise_cycle);
        assert_eq!(row.allowed_functions.len(), 13);
        assert!(row.allowed_functions.contains(&13)); // Redial
        assert!(row.allowed_functions.contains(&16)); // None
    }

    // Free Pro 3
    let free3_rows = gesture_rows_for_product(EARFUN_FREE_PRO_3_PRODUCT_ID).unwrap();
    assert_eq!(free3_rows.len(), 4);
    assert_eq!(free3_rows[0].allowed_functions.len(), 13);

    // Air S (9 functions, no redial)
    let airs_rows = gesture_rows_for_product(EARFUN_AIR_S_PRODUCT_ID).unwrap();
    assert_eq!(airs_rows.len(), 4);
    for row in airs_rows {
        assert!(!row.is_call_control);
        assert_eq!(row.allowed_functions.len(), 9);
        assert!(!row.allowed_functions.contains(&13)); // No Redial
        assert!(row.allowed_functions.contains(&16)); // None
    }

    // Unknown product
    assert!(gesture_rows_for_product(0x999999).is_none());
}

#[test]
fn test_earfun_gesture_function_labels() {
    assert_eq!(
        earfun_gesture_function_label_key(16),
        "controls-function-none"
    );
    assert_eq!(
        earfun_gesture_function_label_key(1),
        "controls-function-play-pause"
    );
    assert_eq!(
        earfun_gesture_function_label_key(2),
        "controls-function-volume-up"
    );
    assert_eq!(
        earfun_gesture_function_label_key(3),
        "controls-function-volume-down"
    );
    assert_eq!(
        earfun_gesture_function_label_key(8),
        "controls-function-noise-control-all"
    );
    assert_eq!(
        earfun_gesture_function_label_key(9),
        "controls-function-noise-normal-wind"
    );
    assert_eq!(
        earfun_gesture_function_label_key(10),
        "controls-function-noise-normal-ambient"
    );
    assert_eq!(
        earfun_gesture_function_label_key(11),
        "controls-function-noise-wind-ambient"
    );
    assert_eq!(
        earfun_gesture_function_label_key(13),
        "controls-function-redial"
    );
    assert_eq!(
        earfun_gesture_function_label_key(99),
        "controls-function-unknown"
    );
}

#[test]
fn test_earfun_touch_lock_feature_declaration() {
    let air_pro_4 = profile_for_product_id(EARFUN_AIR_PRO_4_PRODUCT_ID).unwrap();
    assert!(air_pro_4.supports_touch_customization());
    assert!(air_pro_4.supports_touch_lock());
    assert!(air_pro_4.supports_find_device());
    assert!(air_pro_4.supports_find_device_sides());
    assert!(air_pro_4.supports_prompt_volume());
    assert!(air_pro_4.supports_microphone_mode());

    let free_pro_3 = profile_for_product_id(EARFUN_FREE_PRO_3_PRODUCT_ID).unwrap();
    assert!(free_pro_3.supports_touch_customization());
    assert!(free_pro_3.supports_touch_lock());
    assert!(free_pro_3.supports_find_device());
    assert!(free_pro_3.supports_find_device_sides());
    assert!(!free_pro_3.supports_prompt_volume());
    assert!(!free_pro_3.supports_microphone_mode());

    let air_s = profile_for_product_id(EARFUN_AIR_S_PRODUCT_ID).unwrap();
    assert!(air_s.supports_touch_customization());
    assert!(!air_s.supports_touch_lock());
    assert!(!air_s.supports_find_device());
    assert!(!air_s.supports_find_device_sides());
    assert!(!air_s.supports_prompt_volume());
    assert!(!air_s.supports_microphone_mode());
}

#[test]
fn firmware_model_profiles_are_resolved_without_fabricated_product_ids() {
    let models = [
        ("TW700", "EarFun Air Pro 4+", true, true),
        ("TW700AI", "EarFun Air Pro 4+AI", true, true),
        ("TW800", "EarFun Air Pro 5", true, true),
        ("TW500", "EarFun Air Pro 3", true, true),
        ("TW500_3072", "EarFun Air Pro 3 (3072)", true, true),
        ("TWA01", "EarFun Air S", true, false),
        ("ATVEL_TW201", "ATVEL Tune S5 ProX", false, false),
        ("TW101S", "EarFun Free 2S", false, false),
        ("HP1000", "EarFun Wave Pro X", true, true),
    ];

    for (model_code, model_name, supports_noise, supports_dual) in models {
        let profile = profile_for_identity(0, Some(model_code))
            .unwrap_or_else(|| panic!("missing profile for {model_code}"));
        assert_eq!(profile.model, model_name);
        assert_eq!(profile.product_id, None);
        assert_eq!(
            profile.supports_noise_control(),
            supports_noise,
            "{model_code} noise"
        );
        assert_eq!(
            profile.supports_dual_device(),
            supports_dual,
            "{model_code} dual"
        );
        assert!(profile.supports_custom_equalizer());
    }

    let tw700 = profile_for_identity(0, Some("TW700")).unwrap();
    assert!(tw700.supports_wear_detection());
    assert!(tw700.supports_audio_codec());
    assert!(tw700.supports_touch_customization());
    assert!(tw700.supports_touch_lock());
    assert!(crate::compatibility::earfun_dual_device_restart_required(
        0,
        Some("TW700")
    ));
    assert!(!crate::compatibility::earfun_dual_device_restart_required(
        0,
        Some("TW500")
    ));
    assert!(crate::compatibility::earfun_is_headphones(
        0,
        Some("HP1000")
    ));
    assert!(!crate::compatibility::earfun_has_case_battery(
        0,
        Some("HP1000")
    ));
    assert_eq!(
        profile_for_identity(EARFUN_FREE_PRO_3_PRODUCT_ID, Some("TW700"))
            .unwrap()
            .model,
        "EarFun Free Pro 3"
    );
    assert!(profile_for_identity(0, Some("not-a-model")).is_none());
    assert!(profile_for_product_id(0).is_none());
}

#[test]
fn test_global_gesture_dispatch_and_brand_isolation() {
    // EarFun resolution
    let earfun_rows = global_gesture_rows(EARFUN_AIR_PRO_4_PRODUCT_ID);
    assert_eq!(earfun_rows.len(), 4);
    assert!(!earfun_rows.iter().any(|r| r.is_call_control));

    // Function 13 in EarFun is Redial
    assert_eq!(
        gesture_function_label_key_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID, 13),
        "controls-function-redial"
    );
    // Function 3 in EarFun is Volume Down
    assert_eq!(
        gesture_function_label_key_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID, 3),
        "controls-function-volume-down"
    );

    // OPPO resolution (e.g. Enco X4 = 0x06EC10)
    let oppo_rows = global_gesture_rows(0x06ec10);
    assert_eq!(oppo_rows.len(), 7);
    assert!(oppo_rows.iter().any(|r| r.is_call_control));

    // Function 13 in OPPO is Switch Devices
    assert_eq!(
        gesture_function_label_key_for_product(0x06ec10, 13),
        "controls-function-switch-devices"
    );
    // Function 3 in OPPO is Voice Assistant
    assert_eq!(
        gesture_function_label_key_for_product(0x06ec10, 3),
        "controls-function-voice-assistant"
    );

    // Function pointer on GestureRow itself resolves directly without brand branching
    assert_eq!(
        (earfun_rows[0].function_label_key)(13),
        "controls-function-redial"
    );
    assert_eq!(
        (oppo_rows[0].function_label_key)(13),
        "controls-function-switch-devices"
    );
}

#[test]
fn test_earfun_equalizer_presets_and_gains() {
    // Air S presets & gains
    let airs_presets = equalizer_presets_for_product(EARFUN_AIR_S_PRODUCT_ID).unwrap();
    assert_eq!(airs_presets.len(), 9);
    assert_eq!(airs_presets[0].key, "equalizer-preset-classic");
    assert_eq!(airs_presets[2].key, "equalizer-preset-bass-boost");

    let airs_classic = preset_gains_for_product(EARFUN_AIR_S_PRODUCT_ID, 0).unwrap();
    assert_eq!(airs_classic, &[0.0; 6]);
    let airs_bass = preset_gains_for_product(EARFUN_AIR_S_PRODUCT_ID, 2).unwrap();
    assert_eq!(airs_bass, &[8.0, 3.0, 2.0, 0.0, 0.0, 0.0]);

    // Out of range preset index
    assert!(preset_gains_for_product(EARFUN_AIR_S_PRODUCT_ID, 9).is_none());

    // Air Pro 4 / Free Pro 3 10-band presets & gains
    let pro4_presets = equalizer_presets_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID).unwrap();
    assert_eq!(pro4_presets.len(), 9);
    assert_eq!(pro4_presets[8].key, "equalizer-preset-pure-vocal");

    let pro4_flat = preset_gains_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID, 0).unwrap();
    assert_eq!(pro4_flat, &[0.0; 10]);
    let pro4_bass = preset_gains_for_product(EARFUN_AIR_PRO_4_PRODUCT_ID, 2).unwrap();
    assert_eq!(
        pro4_bass,
        &[8.0, 6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    );

    // Unknown product
    assert!(equalizer_presets_for_product(0x999999).is_none());
    assert!(preset_gains_for_product(0x999999, 0).is_none());
}

#[test]
fn test_earfun_build_equalizer_bands_and_dummy_padding() {
    // Air S: 6 active bands + 4 dummy bands = 10 bands
    let airs_bands = equalizer_bands_for_preset(EARFUN_AIR_S_PRODUCT_ID, 2).unwrap();
    assert_eq!(airs_bands.len(), 10);

    // Active bands 0..5
    assert_eq!(airs_bands[0].band_id, 0xA1);
    assert_eq!(airs_bands[0].frequency, 0x00BD);
    assert_eq!(airs_bands[0].gain_db, 8.0);

    assert_eq!(airs_bands[5].band_id, 0xA6);
    assert_eq!(airs_bands[5].frequency, 0xAFC8);
    assert_eq!(airs_bands[5].gain_db, 0.0);

    // Dummy bands 6..9 (ID 0xA7..0xAA)
    assert_eq!(airs_bands[6].band_id, 0xA7);
    assert_eq!(airs_bands[6].gain_db, 4.5);
    assert_eq!(airs_bands[7].band_id, 0xA8);
    assert_eq!(airs_bands[7].gain_db, -4.5);
    assert_eq!(airs_bands[8].band_id, 0xA9);
    assert_eq!(airs_bands[8].gain_db, 1.5);
    assert_eq!(airs_bands[9].band_id, 0xAA);
    assert_eq!(airs_bands[9].gain_db, 3.5);

    // Air Pro 4: 10 active bands
    let pro4_bands = equalizer_bands_for_preset(EARFUN_AIR_PRO_4_PRODUCT_ID, 2).unwrap();
    assert_eq!(pro4_bands.len(), 10);
    assert_eq!(pro4_bands[0].band_id, 0xA1);
    assert_eq!(pro4_bands[0].frequency, 0x005E);
    assert_eq!(pro4_bands[0].q_factor, 0x0B33);
    assert_eq!(pro4_bands[0].gain_db, 8.0);
    assert_eq!(pro4_bands[9].band_id, 0xAA);
    assert_eq!(pro4_bands[9].frequency, 0xBB80);
    assert_eq!(pro4_bands[9].q_factor, 0x0B33);
    assert_eq!(pro4_bands[9].gain_db, 0.0);

    // Wrong number of bands returns None
    assert!(build_equalizer_bands(EARFUN_AIR_S_PRODUCT_ID, &[0.0; 10]).is_none());
    assert!(build_equalizer_bands(EARFUN_AIR_PRO_4_PRODUCT_ID, &[0.0; 6]).is_none());
}

#[test]
fn test_earfun_build_equalizer_bands_from_custom() {
    let custom_10 = vec![
        (31, -2),
        (62, 0),
        (125, 1),
        (250, 3),
        (500, 4),
        (1000, 5),
        (2000, 2),
        (4000, 1),
        (8000, -1),
        (16000, -3),
    ];

    // Air Pro 4: uses all 10 bands directly
    let pro4_custom =
        build_equalizer_bands_from_custom(EARFUN_AIR_PRO_4_PRODUCT_ID, &custom_10).unwrap();
    assert_eq!(pro4_custom.len(), 10);
    assert_eq!(pro4_custom[0].band_id, 0xA1);
    assert_eq!(pro4_custom[0].gain_db, -2.0);
    assert_eq!(pro4_custom[5].gain_db, 5.0);
    assert_eq!(pro4_custom[9].gain_db, -3.0);

    // Air S with 10 bands: maps closest active bands and pads dummy bands
    let airs_custom =
        build_equalizer_bands_from_custom(EARFUN_AIR_S_PRODUCT_ID, &custom_10).unwrap();
    assert_eq!(airs_custom.len(), 10);
    // Band 0 is from custom_10[1] (62 Hz -> 63 Hz): gain 0.0
    assert_eq!(airs_custom[0].gain_db, 0.0);
    // Band 4 is from custom_10[8] (8000 Hz): gain -1.0
    assert_eq!(airs_custom[4].gain_db, -1.0);
    // Band 5 is from custom_10[9] (16000 Hz -> 15000 Hz): gain -3.0
    assert_eq!(airs_custom[5].gain_db, -3.0);
    // Dummy bands 6..9
    assert_eq!(airs_custom[6].gain_db, 4.5);
    assert_eq!(airs_custom[7].gain_db, -4.5);
    assert_eq!(airs_custom[8].gain_db, 1.5);
    assert_eq!(airs_custom[9].gain_db, 3.5);

    // Invalid product ID
    assert!(build_equalizer_bands_from_custom(0x999999, &custom_10).is_none());
}
