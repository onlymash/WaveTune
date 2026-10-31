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

use std::{fs, time::SystemTime};

use crate::app::events::UiUpdate;

use super::{
    cache::generic_artwork_path,
    fetcher::spawn_artwork_fetch,
    icons::{
        AppIconStyle, apply_user_icon_override_to_dir, ensure_app_icon_cache_in,
        resolve_tray_icon_theme_path_with,
    },
    preferences::{
        autostart_file_in, get_autostart_background_in, read_custom_equalizers_from,
        read_equalizer_preset_from, read_run_in_background_from, set_autostart_background_in,
        write_custom_equalizers_to, write_equalizer_preset_to, write_run_in_background_to,
    },
};

#[test]
fn loads_generic_artwork() {
    let generic_path = generic_artwork_path(false);
    assert!(generic_path.exists());
    let meta = fs::metadata(&generic_path).expect("should read metadata");
    assert!(meta.len() > 0);

    let dark_path = generic_artwork_path(true);
    assert!(dark_path.exists());
    let meta_dark = fs::metadata(&dark_path).expect("should read metadata");
    assert!(meta_dark.len() > 0);
}

#[test]
fn app_icon_style_round_trips_and_writes_user_hicolor_override() {
    for (idx, style) in AppIconStyle::ALL.into_iter().enumerate() {
        assert_eq!(style.to_index(), idx as u32);
        assert_eq!(AppIconStyle::from_index(idx as u32), style);
        assert_eq!(AppIconStyle::from_config_str(style.as_config_str()), style);
        assert!(style.svg_content().contains("<svg"));
    }

    let temp_dir = std::env::temp_dir().join(format!(
        "wavetune_icon_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let target_svg = temp_dir
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps")
        .join("io.github.onlymash.WaveTune.svg");

    apply_user_icon_override_to_dir(&temp_dir, AppIconStyle::Circle)
        .expect("circle override should succeed");
    assert_eq!(
        fs::read_to_string(&target_svg).expect("override svg should exist"),
        AppIconStyle::Circle.svg_content()
    );

    apply_user_icon_override_to_dir(&temp_dir, AppIconStyle::TwsCircle)
        .expect("tws-circle override should succeed");
    assert_eq!(
        fs::read_to_string(&target_svg).expect("override svg should update"),
        AppIconStyle::TwsCircle.svg_content()
    );

    apply_user_icon_override_to_dir(&temp_dir, AppIconStyle::Default)
        .expect("default cleanup should succeed");
    assert!(!target_svg.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn background_and_autostart_preferences_round_trip() {
    let temp_dir = std::env::temp_dir().join(format!(
        "wavetune_bg_pref_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let settings_path = temp_dir.join("wavetune").join("settings.ini");

    // Default when settings.ini does not exist is true
    assert!(read_run_in_background_from(&settings_path));

    write_run_in_background_to(&settings_path, false).expect("should write false");
    assert!(!read_run_in_background_from(&settings_path));

    write_run_in_background_to(&settings_path, true).expect("should write true");
    assert!(read_run_in_background_from(&settings_path));

    // Autostart defaults to false, creates desktop entry when enabled, removes when disabled
    assert!(!get_autostart_background_in(&temp_dir));
    set_autostart_background_in(&temp_dir, true).expect("should create autostart file");
    assert!(get_autostart_background_in(&temp_dir));
    let desktop_content =
        fs::read_to_string(autostart_file_in(&temp_dir)).expect("desktop file should exist");
    assert!(desktop_content.contains("Exec=wavetune --background"));

    set_autostart_background_in(&temp_dir, false).expect("should remove autostart file");
    assert!(!get_autostart_background_in(&temp_dir));
    assert!(!autostart_file_in(&temp_dir).exists());

    let cache_icons = temp_dir.join("cache_icons");
    ensure_app_icon_cache_in(&cache_icons);
    assert!(cache_icons.join("hicolor").join("index.theme").exists());
    assert!(
        cache_icons
            .join("io.github.onlymash.WaveTune-symbolic.svg")
            .exists()
    );

    let empty_xdg = vec![temp_dir.join("empty_share")];
    assert_eq!(
        resolve_tray_icon_theme_path_with(
            "io.github.onlymash.WaveTune-symbolic",
            &empty_xdg,
            &cache_icons,
        ),
        cache_icons.to_string_lossy()
    );

    let installed_share = temp_dir.join("installed_share");
    apply_user_icon_override_to_dir(&installed_share, AppIconStyle::Circle)
        .expect("should write symbolic icon into xdg share");
    assert_eq!(
        resolve_tray_icon_theme_path_with(
            "io.github.onlymash.WaveTune-symbolic",
            std::slice::from_ref(&installed_share),
            &cache_icons,
        ),
        ""
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn equalizer_preset_persistence_round_trip() {
    let temp_dir = std::env::temp_dir().join(format!(
        "wavetune_eq_pref_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let settings_path = temp_dir.join("wavetune").join("settings.ini");

    // Returns None when file doesn't exist
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, None),
        None
    );

    let mac_a = "11:22:33:44:55:66";
    let mac_b = "AA:BB:CC:DD:EE:FF";

    // Save preset 2 (Bass Boost) for device A
    write_equalizer_preset_to(&settings_path, 0x004535, Some(mac_a), 2)
        .expect("should write preset");
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, Some(mac_a)),
        Some(2)
    );

    // MAC case-insensitivity check (e.g. lowercase vs uppercase)
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, Some("11:22:33:44:55:66")),
        Some(2)
    );

    // Device B on the same model returns None (uniqueness per device)
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, Some(mac_b)),
        None
    );

    // Save preset 5 (Dynamic) for device B
    write_equalizer_preset_to(&settings_path, 0x004535, Some(mac_b), 5)
        .expect("should write preset");
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, Some(mac_b)),
        Some(5)
    );
    // Device A remains 2
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, Some(mac_a)),
        Some(2)
    );

    // Fallback: save generic product preset and query with None
    write_equalizer_preset_to(&settings_path, 0x004535, None, 1).expect("should write preset");
    assert_eq!(
        read_equalizer_preset_from(&settings_path, 0x004535, None),
        Some(1)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn custom_equalizer_persistence_round_trip() {
    let temp_dir = std::env::temp_dir().join(format!(
        "wavetune_custom_eq_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let settings_path = temp_dir.join("wavetune").join("settings.ini");

    // Returns empty when file doesn't exist
    assert!(read_custom_equalizers_from(&settings_path, 0x004535, None).is_empty());

    let mac = "11:22:33:44:55:66";
    let custom_list = vec![crate::protocol::CustomEqualizer {
        id: 100,
        name: "Rock Test".to_string(),
        min_gain: -6,
        max_gain: 6,
        is_selected: true,
        bands: vec![(1000, 3), (2000, -2)],
    }];

    write_custom_equalizers_to(&settings_path, 0x004535, Some(mac), &custom_list)
        .expect("should write custom equalizers");

    let loaded = read_custom_equalizers_from(&settings_path, 0x004535, Some(mac));
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, 100);
    assert_eq!(loaded[0].name, "Rock Test");
    assert!(loaded[0].is_selected);
    assert_eq!(loaded[0].bands, vec![(1000, 3), (2000, -2)]);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore]
fn downloads_and_caches_x3_3d_scene() {
    let (tx, rx) = std::sync::mpsc::channel();
    spawn_artwork_fetch(0x067410, Some(3), tx);
    let start = std::time::Instant::now();
    let mut got_scene = false;
    while start.elapsed() < std::time::Duration::from_secs(30) {
        if let Ok(msg) = rx.recv_timeout(std::time::Duration::from_millis(500)) {
            if let UiUpdate::DeviceScene(path) = msg {
                assert!(path.exists());
                got_scene = true;
                break;
            }
        }
    }
    assert!(got_scene, "Should receive DeviceScene for Enco X3");
}
