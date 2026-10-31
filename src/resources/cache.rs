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

//! Local device assets, 3D scenes, models, and artwork cache.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const GENERIC_HEADSET_SVG_TEMPLATE: &str = include_str!("../../assets/images/generic_headset.svg");

pub fn cache_dir() -> PathBuf {
    let wavetune_path = gtk::glib::user_cache_dir().join("wavetune").join("devices");
    if wavetune_path.exists() {
        return wavetune_path;
    }
    if !wavetune_path.exists() {
        let _ = fs::create_dir_all(&wavetune_path);
    }
    wavetune_path
}

pub fn generic_artwork_path(dark: bool) -> PathBuf {
    let color = if dark { "#f0f0f0" } else { "#333333" };
    let file_name = if dark {
        "generic_headset_dark.svg"
    } else {
        "generic_headset.svg"
    };
    let file_path = cache_dir().join(file_name);
    let tinted_svg = GENERIC_HEADSET_SVG_TEMPLATE.replace("currentColor", color);
    if !file_path.exists()
        || fs::read_to_string(&file_path)
            .map(|content| content != tinted_svg)
            .unwrap_or(true)
    {
        let _ = fs::write(&file_path, tinted_svg);
    }
    file_path
}

pub fn is_generic_artwork(path: &Path) -> bool {
    path.file_name()
        .map(|name| name == "generic_headset.svg" || name == "generic_headset_dark.svg")
        .unwrap_or(false)
}

pub fn load_cached_scene(product_id: u32, color_id: Option<u8>) -> Option<PathBuf> {
    if let Some(color) = color_id {
        let file_path = cache_dir().join(format!("{product_id:06X}_{color}_scene.vfxms"));
        if file_path.exists() {
            if let Ok(meta) = fs::metadata(&file_path) {
                if meta.len() > 0 {
                    return Some(file_path);
                }
            }
        }
    }
    let file_path = cache_dir().join(format!("{product_id:06X}_scene.vfxms"));
    if file_path.exists() {
        if let Ok(meta) = fs::metadata(&file_path) {
            if meta.len() > 0 {
                return Some(file_path);
            }
        }
    }

    None
}

pub fn save_cached_scene(
    product_id: u32,
    color_id: Option<u8>,
    data: &[u8],
) -> io::Result<PathBuf> {
    let file_path = match color_id {
        Some(color) => cache_dir().join(format!("{product_id:06X}_{color}_scene.vfxms")),
        None => cache_dir().join(format!("{product_id:06X}_scene.vfxms")),
    };
    fs::write(&file_path, data)?;
    if color_id.is_some() {
        let generic_path = cache_dir().join(format!("{product_id:06X}_scene.vfxms"));
        if !generic_path.exists() {
            let _ = fs::write(&generic_path, data);
        }
    }
    Ok(file_path)
}

pub fn load_cached_artwork(product_id: u32, color_id: Option<u8>) -> Option<PathBuf> {
    if let Some(color) = color_id {
        let file_path = cache_dir().join(format!("{product_id:06X}_{color}.png"));
        if file_path.exists() {
            if let Ok(meta) = fs::metadata(&file_path) {
                if meta.len() > 0 {
                    return Some(file_path);
                }
            }
        }
    }
    let file_path = cache_dir().join(format!("{product_id:06X}.png"));
    if file_path.exists() {
        if let Ok(meta) = fs::metadata(&file_path) {
            if meta.len() > 0 {
                return Some(file_path);
            }
        }
    }

    None
}

pub fn save_cached_artwork(
    product_id: u32,
    color_id: Option<u8>,
    data: &[u8],
) -> io::Result<PathBuf> {
    let file_path = match color_id {
        Some(color) => cache_dir().join(format!("{product_id:06X}_{color}.png")),
        None => cache_dir().join(format!("{product_id:06X}.png")),
    };
    fs::write(&file_path, data)?;
    if color_id.is_some() {
        let generic_path = cache_dir().join(format!("{product_id:06X}.png"));
        if !generic_path.exists() {
            let _ = fs::write(&generic_path, data);
        }
    }
    Ok(file_path)
}

pub fn load_cached_model(product_id: u32, color_id: Option<u8>) -> Option<PathBuf> {
    if let Some(color) = color_id {
        let file_path = cache_dir().join(format!("{product_id:06X}_{color}_model.webp"));
        if file_path.exists() {
            if let Ok(meta) = fs::metadata(&file_path) {
                if meta.len() > 0 {
                    return Some(file_path);
                }
            }
        }
    }
    let file_path = cache_dir().join(format!("{product_id:06X}_model.webp"));
    if file_path.exists() {
        if let Ok(meta) = fs::metadata(&file_path) {
            if meta.len() > 0 {
                return Some(file_path);
            }
        }
    }

    None
}

pub fn save_cached_model(
    product_id: u32,
    color_id: Option<u8>,
    data: &[u8],
) -> io::Result<PathBuf> {
    let file_path = match color_id {
        Some(color) => cache_dir().join(format!("{product_id:06X}_{color}_model.webp")),
        None => cache_dir().join(format!("{product_id:06X}_model.webp")),
    };
    fs::write(&file_path, data)?;
    if color_id.is_some() {
        let generic_path = cache_dir().join(format!("{product_id:06X}_model.webp"));
        if !generic_path.exists() {
            let _ = fs::write(&generic_path, data);
        }
    }
    Ok(file_path)
}

