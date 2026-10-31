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

pub mod earfun_catalog;
pub mod earfun_equalizer;
pub mod earfun_gestures;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use earfun_catalog::{
    EARFUN_AIR_PRO_4_NOISE_MODES, EARFUN_AIR_S_NOISE_MODES, EARFUN_DEVICE_PROFILES,
    supported_earfun_codecs,
};
#[allow(unused_imports)]
pub use earfun_equalizer::{
    EARFUN_AIR_PRO_4_EQ_PRESETS, EARFUN_AIR_S_EQ_PRESETS, EARFUN_TEN_BAND_EQ_PRESETS,
    build_equalizer_bands, equalizer_bands_for_preset, equalizer_presets_for_product,
    preset_gains_for_product,
};
#[allow(unused_imports)]
pub use earfun_gestures::{
    EARFUN_AIR_PRO_4_GESTURE_ROWS, EARFUN_AIR_S_GESTURE_ROWS, EARFUN_FREE_PRO_3_GESTURE_ROWS,
    earfun_gesture_function_label_key, gesture_rows_for_product, is_earfun_product,
};
