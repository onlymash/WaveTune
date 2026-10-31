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

//! EarFun equalizer presets, hardware PEQ filters, and product mappings.

use crate::compatibility::EqualizerPreset;
use crate::protocol::brands::earfun::GaiaEqBand;

pub use super::earfun_gestures::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, EARFUN_FREE_PRO_3_PRODUCT_ID,
};

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

// --- Hardware PEQ Filter Specifications ---

/// Frequencies for EarFun Air S (6 active bands + 4 dummy bands).
pub const EARFUN_AIR_S_FREQUENCIES: [u16; 10] = [
    0x00BD, 0x021C, 0x05DC, 0x0BB8, 0x5DC0, 0xAFC8, // 63, 180, 500, 1000, 8000, 15000 Hz
    0x0063, 0x0258, 0x1770, 0xAFC8, // dummy bands 7..10
];

/// Q-factors for EarFun Air S.
pub const EARFUN_AIR_S_Q_FACTORS: [u16; 10] = [
    0x0B33, 0x0999, 0x0999, 0x0999, 0x0999, 0x0999, 0x0800, 0x0999, 0x0E66, 0x0999,
];

/// Fixed dummy gains for EarFun Air S dummy bands (bands 7..10).
pub const EARFUN_AIR_S_DUMMY_GAINS: [f64; 4] = [4.5, -4.5, 1.5, 3.5];

/// Gains for the 9 presets on EarFun Air S (6 active bands each).
pub static EARFUN_AIR_S_PRESET_GAINS: &[[f64; 6]] = &[
    [0.0; 6],                          // 0: Classic
    [0.0, 0.0, 1.0, 1.0, 2.0, 3.0],    // 1: Natural
    [8.0, 3.0, 2.0, 0.0, 0.0, 0.0],    // 2: Bass Boost
    [0.0, 0.0, 0.0, 2.0, 3.0, 5.0],    // 3: Clear Treble
    [-5.0, -2.0, 2.0, 3.0, 0.0, -3.0], // 4: Soft
    [7.0, 3.0, 2.0, 3.0, 5.0, 7.0],    // 5: Dynamic
    [2.0, 0.0, 3.0, 5.0, 3.0, 5.0],    // 6: Clear
    [2.0, 1.0, 0.0, -1.0, -3.0, -5.0], // 7: Relaxed
    [-2.0, 0.0, 4.0, 5.0, 2.0, -1.0],  // 8: Pure Vocal
];

/// Frequencies for EarFun 10-band models (31.5 Hz to 16 kHz).
pub const EARFUN_TEN_BAND_FREQUENCIES: [u16; 10] = [
    0x005E, 0x00BD, 0x0177, 0x02EE, 0x05DC, 0x0BB8, 0x1770, 0x2EE0, 0x5DC0, 0xBB80,
];

/// Uniform Q-factor for 10-band models.
pub const EARFUN_TEN_BAND_Q_FACTOR: u16 = 0x0B33;

/// Gains for the 9 presets on 10-band EarFun models (10 bands each).
pub static EARFUN_TEN_BAND_PRESET_GAINS: &[[f64; 10]] = &[
    [0.0; 10],                                                // 0: Classic / Flat
    [0.0, 0.0, 1.0, 2.0, 2.0, -1.0, -1.0, -1.0, -2.0, 1.0],   // 1: Natural
    [8.0, 6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],       // 2: Bass Boost
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 3.0, 4.0],       // 3: Clear Treble
    [-5.0, -4.0, -2.0, 0.0, 2.0, 3.0, 0.0, -2.0, -3.0, -5.0], // 4: Soft
    [6.0, 6.0, 4.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],       // 5: Dynamic
    [3.0, 2.0, 2.0, 2.0, 3.0, 5.0, 3.0, 3.0, 4.0, 5.0],       // 6: Clear
    [2.0, 1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -3.0, -4.0, -5.0],  // 7: Relaxed
    [-3.0, -2.0, 0.0, 2.0, 3.0, 5.0, 3.0, 2.0, 0.0, -1.0],    // 8: Pure Vocal
];

/// Returns the EarFun equalizer presets for the given product ID, if recognized.
pub fn equalizer_presets_for_product(product_id: u32) -> Option<&'static [EqualizerPreset]> {
    match product_id {
        EARFUN_AIR_S_PRODUCT_ID => Some(EARFUN_AIR_S_EQ_PRESETS),
        EARFUN_AIR_PRO_4_PRODUCT_ID | EARFUN_FREE_PRO_3_PRODUCT_ID => {
            Some(EARFUN_TEN_BAND_EQ_PRESETS)
        }
        _ => None,
    }
}

/// Returns the gain values for a specific preset on a product.
pub fn preset_gains_for_product(product_id: u32, preset_index: u8) -> Option<&'static [f64]> {
    let index = preset_index as usize;
    match product_id {
        EARFUN_AIR_S_PRODUCT_ID => EARFUN_AIR_S_PRESET_GAINS.get(index).map(|g| g.as_slice()),
        EARFUN_AIR_PRO_4_PRODUCT_ID | EARFUN_FREE_PRO_3_PRODUCT_ID => EARFUN_TEN_BAND_PRESET_GAINS
            .get(index)
            .map(|g| g.as_slice()),
        _ => None,
    }
}

/// Builds the 10 GaiaEqBand structures required by Qualcomm GAIA V3 for the given product and gain settings.
/// For Air S, takes 6 active gains and automatically adds the 4 dummy bands.
/// For 10-band models, takes 10 gains.
pub fn build_equalizer_bands(product_id: u32, active_gains: &[f64]) -> Option<[GaiaEqBand; 10]> {
    match product_id {
        EARFUN_AIR_S_PRODUCT_ID => {
            if active_gains.len() != 6 {
                return None;
            }
            let mut bands = [GaiaEqBand {
                band_id: 0,
                frequency: 0,
                q_factor: 0,
                gain_db: 0.0,
            }; 10];

            for i in 0..10 {
                let gain = if i < 6 {
                    active_gains[i]
                } else {
                    EARFUN_AIR_S_DUMMY_GAINS[i - 6]
                };
                bands[i] = GaiaEqBand {
                    band_id: 0xA1 + i as u8,
                    frequency: EARFUN_AIR_S_FREQUENCIES[i],
                    q_factor: EARFUN_AIR_S_Q_FACTORS[i],
                    gain_db: gain,
                };
            }
            Some(bands)
        }
        EARFUN_AIR_PRO_4_PRODUCT_ID | EARFUN_FREE_PRO_3_PRODUCT_ID => {
            if active_gains.len() != 10 {
                return None;
            }
            let mut bands = [GaiaEqBand {
                band_id: 0,
                frequency: 0,
                q_factor: 0,
                gain_db: 0.0,
            }; 10];

            for i in 0..10 {
                bands[i] = GaiaEqBand {
                    band_id: 0xA1 + i as u8,
                    frequency: EARFUN_TEN_BAND_FREQUENCIES[i],
                    q_factor: EARFUN_TEN_BAND_Q_FACTOR,
                    gain_db: active_gains[i],
                };
            }
            Some(bands)
        }
        _ => None,
    }
}

/// Returns the 10 GaiaEqBand configurations for a given product and preset index.
pub fn equalizer_bands_for_preset(product_id: u32, preset_index: u8) -> Option<[GaiaEqBand; 10]> {
    let gains = preset_gains_for_product(product_id, preset_index)?;
    build_equalizer_bands(product_id, gains)
}

/// Builds the 10 GaiaEqBand configurations from custom equalizer bands (frequencies and gains in dB).
pub fn build_equalizer_bands_from_custom(
    product_id: u32,
    custom_bands: &[(u16, i8)],
) -> Option<[GaiaEqBand; 10]> {
    match product_id {
        EARFUN_AIR_S_PRODUCT_ID => {
            let gains = if custom_bands.len() == 6 {
                [
                    custom_bands[0].1 as f64,
                    custom_bands[1].1 as f64,
                    custom_bands[2].1 as f64,
                    custom_bands[3].1 as f64,
                    custom_bands[4].1 as f64,
                    custom_bands[5].1 as f64,
                ]
            } else if custom_bands.len() >= 10 {
                [
                    custom_bands[1].1 as f64,
                    ((custom_bands[2].1 as f64 + custom_bands[3].1 as f64) / 2.0).round(),
                    custom_bands[4].1 as f64,
                    custom_bands[5].1 as f64,
                    custom_bands[8].1 as f64,
                    custom_bands[9].1 as f64,
                ]
            } else {
                return None;
            };
            build_equalizer_bands(product_id, &gains)
        }
        EARFUN_AIR_PRO_4_PRODUCT_ID | EARFUN_FREE_PRO_3_PRODUCT_ID => {
            if custom_bands.len() != 10 {
                return None;
            }
            let mut gains = [0.0; 10];
            for i in 0..10 {
                gains[i] = custom_bands[i].1 as f64;
            }
            build_equalizer_bands(product_id, &gains)
        }
        _ => None,
    }
}
