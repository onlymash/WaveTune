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
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
    thread,
    time::{Duration, SystemTime},
};

use crate::app::events::UiUpdate;

static API_BASE_URL: &str = "https://encoapi.vercel.app";

const GENERIC_HEADSET_SVG_TEMPLATE: &str = include_str!("../../assets/images/generic_headset.svg");
const APP_ICON_DEFAULT_SVG: &str =
    include_str!("../../assets/icons/io.github.onlymash.WaveTune.svg");
const APP_ICON_CIRCLE_SVG: &str =
    include_str!("../../assets/icons/io.github.onlymash.WaveTune-circle.svg");
const APP_ICON_TWS_SVG: &str =
    include_str!("../../assets/icons/io.github.onlymash.WaveTune-tws.svg");
const APP_ICON_TWS_CIRCLE_SVG: &str =
    include_str!("../../assets/icons/io.github.onlymash.WaveTune-tws-circle.svg");
const APP_ICON_SYMBOLIC_SVG: &str =
    include_str!("../../assets/icons/io.github.onlymash.WaveTune-symbolic.svg");
const APP_ICON_THEME_INDEX: &str = "\
[Icon Theme]
Name=Hicolor
Comment=WaveTune application icons
Directories=scalable/apps,symbolic/apps

[scalable/apps]
Size=48
Type=Scalable
MinSize=1
MaxSize=512

[symbolic/apps]
Size=16
Type=Scalable
MinSize=1
MaxSize=512
";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppIconStyle {
    #[default]
    Default,
    Circle,
    Tws,
    TwsCircle,
}

impl AppIconStyle {
    pub const ALL: [Self; 4] = [Self::Default, Self::Circle, Self::Tws, Self::TwsCircle];

    pub fn from_config_str(value: &str) -> Self {
        match value.trim() {
            "circle" => Self::Circle,
            "tws" => Self::Tws,
            "tws-circle" => Self::TwsCircle,
            _ => Self::Default,
        }
    }

    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Circle => "circle",
            Self::Tws => "tws",
            Self::TwsCircle => "tws-circle",
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Circle,
            2 => Self::Tws,
            3 => Self::TwsCircle,
            _ => Self::Default,
        }
    }

    pub fn to_index(self) -> u32 {
        match self {
            Self::Default => 0,
            Self::Circle => 1,
            Self::Tws => 2,
            Self::TwsCircle => 3,
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Default => "io.github.onlymash.WaveTune",
            Self::Circle => "io.github.onlymash.WaveTune-circle",
            Self::Tws => "io.github.onlymash.WaveTune-tws",
            Self::TwsCircle => "io.github.onlymash.WaveTune-tws-circle",
        }
    }

    pub fn svg_content(self) -> &'static str {
        match self {
            Self::Default => APP_ICON_DEFAULT_SVG,
            Self::Circle => APP_ICON_CIRCLE_SVG,
            Self::Tws => APP_ICON_TWS_SVG,
            Self::TwsCircle => APP_ICON_TWS_CIRCLE_SVG,
        }
    }
}

fn config_file_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("wavetune")
        .join("settings.ini")
}

fn autostart_file_in(config_dir: &Path) -> PathBuf {
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
        write_if_changed(&path, AUTOSTART_DESKTOP_ENTRY)
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

pub fn get_configured_icon_style() -> AppIconStyle {
    let path = config_file_path();
    if !path.exists() {
        return AppIconStyle::Default;
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(&path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        if let Ok(value) = keyfile.string("General", "icon_style") {
            return AppIconStyle::from_config_str(&value);
        }
    }
    AppIconStyle::Default
}

fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    let needs_write = match fs::read_to_string(path) {
        Ok(existing) => existing != content,
        Err(_) => true,
    };
    if needs_write {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }
    Ok(())
}

pub(crate) fn apply_user_icon_override_to_dir(
    data_dir: &Path,
    style: AppIconStyle,
) -> io::Result<()> {
    let hicolor_dir = data_dir.join("icons").join("hicolor");
    let target_svg = hicolor_dir
        .join("scalable")
        .join("apps")
        .join("io.github.onlymash.WaveTune.svg");
    let target_symbolic_svg = hicolor_dir
        .join("symbolic")
        .join("apps")
        .join("io.github.onlymash.WaveTune-symbolic.svg");

    match style {
        AppIconStyle::Default => {
            if target_svg.exists() {
                fs::remove_file(&target_svg)?;
            }
            if target_symbolic_svg.exists() {
                fs::remove_file(&target_symbolic_svg)?;
            }
        }
        AppIconStyle::Circle | AppIconStyle::Tws | AppIconStyle::TwsCircle => {
            write_if_changed(&target_svg, style.svg_content())?;
            write_if_changed(&target_symbolic_svg, APP_ICON_SYMBOLIC_SVG)?;
        }
    }

    if hicolor_dir.exists() {
        if let Ok(dir_file) = fs::File::open(&hicolor_dir) {
            let _ = dir_file.set_modified(SystemTime::now());
        }
        if hicolor_dir.join("index.theme").exists() || hicolor_dir.join("icon-theme.cache").exists()
        {
            let _ = Command::new("gtk-update-icon-cache")
                .args(["-f", "-q"])
                .arg(&hicolor_dir)
                .status();
        }
    }

    Ok(())
}

pub fn set_configured_icon_style(style: AppIconStyle) -> io::Result<()> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let keyfile = gtk::glib::KeyFile::new();
    let _ = keyfile.load_from_file(&path, gtk::glib::KeyFileFlags::NONE);
    keyfile.set_string("General", "icon_style", style.as_config_str());
    let _ = keyfile.save_to_file(&path);

    apply_user_icon_override_to_dir(&gtk::glib::user_data_dir(), style)?;
    gtk::Window::set_default_icon_name(style.icon_name());
    Ok(())
}

pub(crate) fn app_icon_cache_root() -> PathBuf {
    gtk::glib::user_cache_dir().join("wavetune").join("icons")
}

pub(crate) fn ensure_app_icon_cache_in(icon_search_root: &Path) {
    let scalable_dir = icon_search_root
        .join("hicolor")
        .join("scalable")
        .join("apps");
    let symbolic_dir = icon_search_root
        .join("hicolor")
        .join("symbolic")
        .join("apps");
    let _ = write_if_changed(
        &icon_search_root.join("hicolor").join("index.theme"),
        APP_ICON_THEME_INDEX,
    );

    for style in AppIconStyle::ALL {
        let filename = format!("{}.svg", style.icon_name());
        let _ = write_if_changed(&scalable_dir.join(&filename), style.svg_content());
        let _ = write_if_changed(&icon_search_root.join(&filename), style.svg_content());
    }
    let symbolic_filename = "io.github.onlymash.WaveTune-symbolic.svg";
    let _ = write_if_changed(&symbolic_dir.join(symbolic_filename), APP_ICON_SYMBOLIC_SVG);
    let _ = write_if_changed(
        &icon_search_root.join(symbolic_filename),
        APP_ICON_SYMBOLIC_SVG,
    );
}

/// Writes the application icons into the private cache icon theme and returns its search root.
pub(crate) fn ensure_app_icon_cache() -> PathBuf {
    let icon_search_root = app_icon_cache_root();
    ensure_app_icon_cache_in(&icon_search_root);
    icon_search_root
}

fn icon_exists_in_xdg_data_dir(data_dir: &Path, icon_name: &str) -> bool {
    let hicolor = data_dir.join("icons").join("hicolor");
    let filename = format!("{icon_name}.svg");
    hicolor
        .join("symbolic")
        .join("apps")
        .join(&filename)
        .exists()
        || hicolor
            .join("scalable")
            .join("apps")
            .join(&filename)
            .exists()
}

pub(crate) fn resolve_tray_icon_theme_path_with(
    icon_name: &str,
    xdg_data_dirs: &[PathBuf],
    cache_icon_root: &Path,
) -> String {
    if xdg_data_dirs
        .iter()
        .any(|dir| icon_exists_in_xdg_data_dir(dir, icon_name))
    {
        // Returning an empty path lets GNOME Shell's AppIndicator extension load a native
        // `Gio.ThemedIcon` from the standard icon theme so `-symbolic` icons recolor cleanly.
        String::new()
    } else {
        cache_icon_root.to_string_lossy().into_owned()
    }
}

pub(crate) fn resolve_tray_icon_theme_path(icon_name: &str) -> String {
    let cache_icon_root = ensure_app_icon_cache();
    let system_dirs = gtk::glib::system_data_dirs();
    resolve_tray_icon_theme_path_with(icon_name, &system_dirs, &cache_icon_root)
}

pub fn init_app_icons() {
    let icon_search_root = ensure_app_icon_cache();

    if let Some(display) = gtk::gdk::Display::default() {
        let theme = gtk::IconTheme::for_display(&display);
        theme.add_search_path(&icon_search_root);
    }

    let style = get_configured_icon_style();
    if style != AppIconStyle::Default {
        let _ = apply_user_icon_override_to_dir(&gtk::glib::user_data_dir(), style);
    }
    gtk::Window::set_default_icon_name(style.icon_name());
}

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

pub fn is_generic_artwork(path: &std::path::Path) -> bool {
    path.file_name()
        .map(|name| name == "generic_headset.svg" || name == "generic_headset_dark.svg")
        .unwrap_or(false)
}

pub mod turntable;
pub mod vfxms;

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

pub fn spawn_artwork_fetch(product_id: u32, color_id: Option<u8>, updates_tx: Sender<UiUpdate>) {
    let cached_artwork = load_cached_artwork(product_id, color_id);
    let cached_model = load_cached_model(product_id, color_id);
    let cached_scene = load_cached_scene(product_id, color_id);

    if let Some(ref model_path) = cached_model {
        let _ = updates_tx.send(UiUpdate::DeviceModel(model_path.clone()));
    } else if let Some(ref scene_path) = cached_scene {
        let _ = updates_tx.send(UiUpdate::DeviceScene(scene_path.clone()));
    }

    if let Some(ref artwork_path) = cached_artwork {
        let _ = updates_tx.send(UiUpdate::DeviceArtwork(artwork_path.clone()));
    } else if cached_model.is_none() && cached_scene.is_none() {
        let _ = updates_tx.send(UiUpdate::DeviceArtwork(generic_artwork_path(false)));
    }

    let need_artwork = cached_artwork.is_none();
    let need_model = cached_model.is_none() && cached_scene.is_none();

    if !need_artwork && !need_model {
        return;
    }

    let _ = thread::Builder::new()
        .name("enco-artwork".to_string())
        .spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
            {
                Ok(c) => c,
                Err(_) => return,
            };

            let mut colors_to_try = Vec::new();
            if let Some(c) = color_id {
                colors_to_try.push(c);
            }
            for fallback in [1, 2, 3, 0] {
                if !colors_to_try.contains(&fallback) {
                    colors_to_try.push(fallback);
                }
            }

            // 1. Fetch static artwork if needed
            if need_artwork {
                for &color in &colors_to_try {
                    let url = format!(
                        "{API_BASE_URL}/v1/models/{product_id:06X}/assets/device?color={color}&theme=light"
                    );
                    if let Ok(response) = client.get(&url).send() {
                        if response.status().is_success() {
                            if let Ok(bytes) = response.bytes() {
                                if !bytes.is_empty() {
                                    if let Ok(saved_path) =
                                        save_cached_artwork(product_id, Some(color), &bytes)
                                    {
                                        let _ = updates_tx.send(UiUpdate::DeviceArtwork(saved_path));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Fetch 360° turntable model or 3D scene if needed
            if need_model {
                for &color in &colors_to_try {
                    let config_url = format!(
                        "{API_BASE_URL}/v1/models/{product_id:06X}/files/model?color={color}&path=config.json"
                    );
                    let mut model_webp_path: Option<String> = None;
                    let mut model_scene_path: Option<String> = None;

                    if let Ok(resp) = client.get(&config_url).send() {
                        if resp.status().is_success() {
                            if let Ok(text) = resp.text() {
                                if let Some(pos) = text.find("\"modelWebp\"") {
                                    if let Some(path_pos) = text[pos..].find("\"path\"") {
                                        if let Some(sub) = text[pos + path_pos..].split('"').nth(3) {
                                            if !sub.trim().is_empty() {
                                                model_webp_path = Some(sub.trim().to_string());
                                            }
                                        }
                                    }
                                } else if let Some(pos) = text.find("\"model3D\"") {
                                    if let Some(path_pos) = text[pos..].find("\"path\"") {
                                        if let Some(sub) = text[pos + path_pos..].split('"').nth(3) {
                                            if !sub.trim().is_empty() {
                                                model_scene_path = Some(sub.trim().to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(ref rel_path) = model_webp_path {
                        let model_url = format!(
                            "{API_BASE_URL}/v1/models/{product_id:06X}/files/model?color={color}&path={model_path}",
                            model_path = rel_path
                        );
                        if let Ok(resp) = client.get(&model_url).send() {
                            if resp.status().is_success() {
                                if let Ok(bytes) = resp.bytes() {
                                    if bytes.len() >= 12
                                        && &bytes[0..4] == b"RIFF"
                                        && &bytes[8..12] == b"WEBP"
                                    {
                                        if let Ok(saved_model) =
                                            save_cached_model(product_id, Some(color), &bytes)
                                        {
                                            let _ = updates_tx.send(UiUpdate::DeviceModel(saved_model));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(ref rel_path) = model_scene_path {
                        let download_url = format!(
                            "{API_BASE_URL}/v1/models/{product_id:06X}/download/model?color={color}"
                        );
                        if let Ok(resp) = client.get(&download_url).send() {
                            if resp.status().is_success() {
                                if let Ok(bytes) = resp.bytes() {
                                    if let Ok(mut archive) =
                                        zip::ZipArchive::new(std::io::Cursor::new(bytes))
                                    {
                                        use std::io::Read;
                                        let mut vfx_bytes = Vec::new();
                                        let mut found = false;
                                        if let Ok(mut file) = archive.by_name(rel_path) {
                                            if file.read_to_end(&mut vfx_bytes).is_ok() {
                                                found = true;
                                            }
                                        }
                                        if !found {
                                            for i in 0..archive.len() {
                                                if let Ok(mut file) = archive.by_index(i) {
                                                    if file
                                                        .name()
                                                        .is_ok_and(|name| name.ends_with(".vfxms"))
                                                    {
                                                        if file.read_to_end(&mut vfx_bytes).is_ok()
                                                        {
                                                            found = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if found && vfxms::VfxScene::parse(&vfx_bytes).is_ok() {
                                            if let Ok(saved_scene) =
                                                save_cached_scene(product_id, Some(color), &vfx_bytes)
                                            {
                                                let _ = updates_tx
                                                    .send(UiUpdate::DeviceScene(saved_scene));
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
