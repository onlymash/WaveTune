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

//! Resource management, local asset caching, preferences, and icons.

pub mod cache;
pub mod fetcher;
pub mod icons;
pub mod preferences;
pub mod turntable;
pub mod vfxms;

#[cfg(test)]
mod tests;

// Re-export cache items
#[allow(unused_imports)]
pub use cache::{
    cache_dir, generic_artwork_path, is_generic_artwork, load_cached_artwork, load_cached_model,
    load_cached_scene, save_cached_artwork, save_cached_model, save_cached_scene,
};

// Re-export remote fetcher
#[allow(unused_imports)]
pub use fetcher::spawn_artwork_fetch;

// Re-export icons and desktop integration
#[allow(unused_imports)]
pub use icons::{
    AppIconStyle, get_configured_icon_style, init_app_icons, resolve_tray_icon_theme_path,
    set_configured_icon_style,
};

// Re-export user preferences & persistence
#[allow(unused_imports)]
pub use preferences::{
    get_autostart_background, get_run_in_background, get_saved_custom_equalizers,
    get_saved_equalizer_preset, save_custom_equalizers, save_equalizer_preset,
    set_autostart_background, set_run_in_background,
};
