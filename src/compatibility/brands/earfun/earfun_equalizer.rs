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

//! EarFun equalizer presets and product mappings.

use crate::compatibility::EqualizerPreset;

/// 6-band equalizer presets for EarFun Air S
pub static EARFUN_AIR_S_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-classic",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-natural",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-bass-boost",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-clear-treble",
    },
    EqualizerPreset {
        index: 4,
        key: "equalizer-preset-soft",
    },
    EqualizerPreset {
        index: 5,
        key: "equalizer-preset-dynamic",
    },
    EqualizerPreset {
        index: 6,
        key: "equalizer-preset-clear",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-relaxed",
    },
    EqualizerPreset {
        index: 8,
        key: "equalizer-preset-pure-vocal",
    },
];

/// 10-band equalizer presets for EarFun Air Pro 4 and Free Pro 3
pub static EARFUN_TEN_BAND_EQ_PRESETS: &[EqualizerPreset] = &[
    EqualizerPreset {
        index: 0,
        key: "equalizer-preset-classic",
    },
    EqualizerPreset {
        index: 1,
        key: "equalizer-preset-natural",
    },
    EqualizerPreset {
        index: 2,
        key: "equalizer-preset-bass-boost",
    },
    EqualizerPreset {
        index: 3,
        key: "equalizer-preset-clear-treble",
    },
    EqualizerPreset {
        index: 4,
        key: "equalizer-preset-soft",
    },
    EqualizerPreset {
        index: 5,
        key: "equalizer-preset-dynamic",
    },
    EqualizerPreset {
        index: 6,
        key: "equalizer-preset-clear",
    },
    EqualizerPreset {
        index: 7,
        key: "equalizer-preset-relaxed",
    },
    EqualizerPreset {
        index: 8,
        key: "equalizer-preset-pure-vocal",
    },
];

#[allow(dead_code)]
pub static EARFUN_AIR_PRO_4_EQ_PRESETS: &[EqualizerPreset] = EARFUN_TEN_BAND_EQ_PRESETS;

/// Returns the EarFun equalizer presets for the given product ID, if recognized.
pub fn equalizer_presets_for_product(product_id: u32) -> Option<&'static [EqualizerPreset]> {
    if product_id == 0x004c45 {
        Some(EARFUN_AIR_S_EQ_PRESETS)
    } else if product_id == 0x004534 || product_id == 0x004535 {
        Some(EARFUN_TEN_BAND_EQ_PRESETS)
    } else {
        None
    }
}
