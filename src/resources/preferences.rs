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

//! User preferences and device state persistence in settings.ini.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn config_file_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("wavetune")
        .join("settings.ini")
}

pub(crate) fn autostart_file_in(config_dir: &Path) -> PathBuf {
    config_dir
        .join("autostart")
        .join("io.github.onlymash.WaveTune.desktop")
}

pub(crate) fn read_run_in_background_from(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
        && let Ok(enabled) = keyfile.boolean("General", "run_in_background")
    {
        return enabled;
    }
    true
}

pub(crate) fn write_run_in_background_to(path: &Path, enabled: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let keyfile = gtk::glib::KeyFile::new();
    let _ = keyfile.load_from_file(path, gtk::glib::KeyFileFlags::NONE);
    keyfile.set_boolean("General", "run_in_background", enabled);
    keyfile
        .save_to_file(path)
        .map_err(|err| io::Error::other(err.to_string()))
}

pub fn get_run_in_background() -> bool {
    read_run_in_background_from(&config_file_path())
}

pub fn set_run_in_background(enabled: bool) -> io::Result<()> {
    write_run_in_background_to(&config_file_path(), enabled)
}

pub(crate) fn device_config_group(product_id: u32, mac: Option<&str>) -> String {
    if let Some(mac) = mac {
        let trimmed = mac.trim();
        if !trimmed.is_empty() {
            if let Some(hash) = gtk::glib::compute_checksum_for_string(
                gtk::glib::ChecksumType::Sha256,
                trimmed.to_ascii_uppercase().as_str(),
            ) {
                return format!("Device_{:06X}_{}", product_id, &hash[..12]);
            }
        }
    }
    format!("Product_{:06X}", product_id)
}

pub(crate) fn read_equalizer_preset_from(
    path: &Path,
    product_id: u32,
    mac: Option<&str>,
) -> Option<u8> {
    if !path.exists() {
        return None;
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        // 1. Try device-specific hashed group first
        if mac.is_some() {
            let specific_group = device_config_group(product_id, mac);
            if let Ok(preset) = keyfile.integer(&specific_group, "equalizer_preset") {
                if (0..=255).contains(&preset) {
                    return Some(preset as u8);
                }
            }
        }
        // 2. Fall back to generic product group
        let generic_group = device_config_group(product_id, None);
        if let Ok(preset) = keyfile.integer(&generic_group, "equalizer_preset") {
            if (0..=255).contains(&preset) {
                return Some(preset as u8);
            }
        }
    }
    None
}

pub(crate) fn write_equalizer_preset_to(
    path: &Path,
    product_id: u32,
    mac: Option<&str>,
    preset: u8,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let keyfile = gtk::glib::KeyFile::new();
    let _ = keyfile.load_from_file(path, gtk::glib::KeyFileFlags::NONE);
    let group = device_config_group(product_id, mac);
    keyfile.set_integer(&group, "equalizer_preset", preset as i32);
    keyfile
        .save_to_file(path)
        .map_err(|err| io::Error::other(err.to_string()))
}

pub fn get_saved_equalizer_preset(product_id: u32, mac: Option<&str>) -> Option<u8> {
    read_equalizer_preset_from(&config_file_path(), product_id, mac)
}

pub fn save_equalizer_preset(product_id: u32, mac: Option<&str>, preset: u8) -> io::Result<()> {
    write_equalizer_preset_to(&config_file_path(), product_id, mac, preset)
}

pub(crate) fn read_custom_equalizers_from(
    path: &Path,
    product_id: u32,
    mac: Option<&str>,
) -> Vec<crate::protocol::CustomEqualizer> {
    if !path.exists() {
        return Vec::new();
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        // 1. Try device-specific hashed group first
        if mac.is_some() {
            let specific_group = device_config_group(product_id, mac);
            if let Ok(json_str) = keyfile.string(&specific_group, "custom_equalizers") {
                if let Ok(eqs) =
                    serde_json::from_str::<Vec<crate::protocol::CustomEqualizer>>(&json_str)
                {
                    return eqs;
                }
            }
        }
        // 2. Fall back to generic product group
        let generic_group = device_config_group(product_id, None);
        if let Ok(json_str) = keyfile.string(&generic_group, "custom_equalizers") {
            if let Ok(eqs) =
                serde_json::from_str::<Vec<crate::protocol::CustomEqualizer>>(&json_str)
            {
                return eqs;
            }
        }
    }
    Vec::new()
}

pub(crate) fn write_custom_equalizers_to(
    path: &Path,
    product_id: u32,
    mac: Option<&str>,
    equalizers: &[crate::protocol::CustomEqualizer],
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let keyfile = gtk::glib::KeyFile::new();
    let _ = keyfile.load_from_file(path, gtk::glib::KeyFileFlags::NONE);
    let group = device_config_group(product_id, mac);
    let json_str =
        serde_json::to_string(equalizers).map_err(|err| io::Error::other(err.to_string()))?;
    keyfile.set_string(&group, "custom_equalizers", &json_str);
    keyfile
        .save_to_file(path)
        .map_err(|err| io::Error::other(err.to_string()))
}

pub fn get_saved_custom_equalizers(
    product_id: u32,
    mac: Option<&str>,
) -> Vec<crate::protocol::CustomEqualizer> {
    read_custom_equalizers_from(&config_file_path(), product_id, mac)
}

pub fn save_custom_equalizers(
    product_id: u32,
    mac: Option<&str>,
    equalizers: &[crate::protocol::CustomEqualizer],
) -> io::Result<()> {
    write_custom_equalizers_to(&config_file_path(), product_id, mac, equalizers)
}

const AUTOSTART_DESKTOP_ENTRY: &str = "\
[Desktop Entry]
Type=Application
Name=WaveTune
Comment=Wireless headset control center background service
Comment[zh_CN]=无线耳机控制中心后台服务
Exec=wavetune --background
Icon=io.github.onlymash.WaveTune
Terminal=false
StartupNotify=false
X-GNOME-Autostart-enabled=true
";

pub(crate) fn get_autostart_background_in(config_dir: &Path) -> bool {
    let path = autostart_file_in(config_dir);
    if !path.exists() {
        return false;
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(&path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        if let Ok(hidden) = keyfile.boolean("Desktop Entry", "Hidden")
            && hidden
        {
            return false;
        }
        if let Ok(gn_enabled) = keyfile.boolean("Desktop Entry", "X-GNOME-Autostart-enabled") {
            return gn_enabled;
        }
    }
    true
}

pub(crate) fn set_autostart_background_in(config_dir: &Path, enabled: bool) -> io::Result<()> {
    let path = autostart_file_in(config_dir);
    if enabled {
        crate::resources::icons::write_if_changed(&path, AUTOSTART_DESKTOP_ENTRY)
    } else if path.exists() {
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn get_autostart_background() -> bool {
    get_autostart_background_in(&gtk::glib::user_config_dir())
}

pub fn set_autostart_background(enabled: bool) -> io::Result<()> {
    set_autostart_background_in(&gtk::glib::user_config_dir(), enabled)
}
