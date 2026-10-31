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

//! Application icons, desktop theming, and XDG hicolor integration.

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use super::preferences::config_file_path;

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

pub(crate) fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
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

pub(crate) fn icon_exists_in_xdg_data_dir(data_dir: &Path, icon_name: &str) -> bool {
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

pub fn resolve_tray_icon_theme_path(icon_name: &str) -> String {
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
