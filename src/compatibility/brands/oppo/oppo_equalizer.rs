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

//! OPPO, OnePlus, and realme equalizer presets and product mappings.

use crate::compatibility::EqualizerPreset;

pub static DYNAUDIO_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-dynaudio-classic",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-dynaudio-hi-res",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-clear-treble",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-bass-boost",
    },
];

pub static STANDARD_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-classic",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-dynamic-bass",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-clear-vocal",
    },
];

pub static X3_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-ultimate-sound",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-hd-clarity",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-pure-vocal",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-bass-boost",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-dynaudio-tuning",
    },
];

/// Returns the OPPO-family equalizer presets for the given product ID, if recognized.
pub fn equalizer_presets_for_product(product_id: u32) -> Option<&'static [EqualizerPreset]> {
    if product_id == 0x06ec10 {
        Some(DYNAUDIO_EQ_PRESETS)
    } else if product_id == 0x067410 {
        Some(X3_EQ_PRESETS)
    } else if super::oppo_catalog::OPPO_DEVICE_PROFILES
        .iter()
        .any(|p| p.product_id == Some(product_id))
    {
        Some(STANDARD_EQ_PRESETS)
    } else {
        None
    }
}

/// Returns the game mode feature ID for OPPO models (0x28 for Enco X4, 0x06 default).
pub fn game_mode_feature_id_for_product(product_id: u32) -> u8 {
    if product_id == 0x06ec10 { 0x28 } else { 0x06 }
}
