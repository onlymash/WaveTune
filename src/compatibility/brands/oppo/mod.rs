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

pub mod oppo_catalog;
pub mod oppo_equalizer;
pub mod oppo_gestures;

#[cfg(test)]
mod tests;

pub use oppo_catalog::{DEVICE_PROFILES, OPPO_DEVICE_PROFILES};
pub use oppo_equalizer::{
    DYNAUDIO_EQ_PRESETS, STANDARD_EQ_PRESETS, X3_EQ_PRESETS, equalizer_presets_for_product,
    game_mode_feature_id_for_product,
};
pub use oppo_gestures::{
    STANDARD_GESTURE_ROWS, gesture_function_label_key, gesture_rows_for_product,
};
