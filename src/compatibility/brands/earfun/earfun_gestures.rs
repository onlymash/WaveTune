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

//! EarFun gesture profiles and action definitions.

use crate::compatibility::GestureRow;

pub const EARFUN_AIR_PRO_4_PRODUCT_ID: u32 = 0x004535;
pub const EARFUN_FREE_PRO_3_PRODUCT_ID: u32 = 0x004534;
pub const EARFUN_AIR_S_PRODUCT_ID: u32 = 0x004c45;

/// Allowed functions for EarFun Air Pro 4 and Free Pro 3 (13 options).
pub static EARFUN_PRO_ALLOWED_FUNCTIONS: &[u8] = &[16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13];

/// Allowed functions for EarFun Air S (9 options).
pub static EARFUN_AIR_S_ALLOWED_FUNCTIONS: &[u8] = &[16, 1, 2, 3, 4, 5, 6, 7, 8];

pub fn earfun_gesture_function_label_key(function: u8) -> &'static str {
    match function {
        16 => "controls-function-none",
        1 => "controls-function-play-pause",
        2 => "controls-function-volume-up",
        3 => "controls-function-volume-down",
        4 => "controls-function-previous",
        5 => "controls-function-next",
        6 => "controls-function-voice-assistant",
        7 => "controls-function-game-mode",
        8 => "controls-function-noise-control-all",
        9 => "controls-function-noise-normal-wind",
        10 => "controls-function-noise-normal-ambient",
        11 => "controls-function-noise-wind-ambient",
        13 => "controls-function-redial",
        _ => "controls-function-unknown",
    }
}

pub static EARFUN_AIR_PRO_4_GESTURE_ROWS: &[GestureRow] = &[
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_PRO_ALLOWED_FUNCTIONS,
        default_function: Some(16),
        label_key: "controls-gesture-single",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_PRO_ALLOWED_FUNCTIONS,
        default_function: Some(1),
        label_key: "controls-gesture-double",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::TripleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_PRO_ALLOWED_FUNCTIONS,
        default_function: Some(6),
        label_key: "controls-gesture-triple",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::SqueezeAndHold,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_PRO_ALLOWED_FUNCTIONS,
        default_function: Some(8),
        label_key: "controls-gesture-hold",
        function_label_key: earfun_gesture_function_label_key,
    },
];

pub static EARFUN_FREE_PRO_3_GESTURE_ROWS: &[GestureRow] = EARFUN_AIR_PRO_4_GESTURE_ROWS;

pub static EARFUN_AIR_S_GESTURE_ROWS: &[GestureRow] = &[
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_AIR_S_ALLOWED_FUNCTIONS,
        default_function: Some(16),
        label_key: "controls-gesture-single",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_AIR_S_ALLOWED_FUNCTIONS,
        default_function: Some(1),
        label_key: "controls-gesture-double",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::TripleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_AIR_S_ALLOWED_FUNCTIONS,
        default_function: Some(6),
        label_key: "controls-gesture-triple",
        function_label_key: earfun_gesture_function_label_key,
    },
    GestureRow {
        action: crate::protocol::GestureAction::SqueezeAndHold,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: EARFUN_AIR_S_ALLOWED_FUNCTIONS,
        default_function: Some(8),
        label_key: "controls-gesture-hold",
        function_label_key: earfun_gesture_function_label_key,
    },
];

pub fn gesture_rows_for_product(product_id: u32) -> Option<&'static [GestureRow]> {
    match product_id {
        EARFUN_AIR_PRO_4_PRODUCT_ID => Some(EARFUN_AIR_PRO_4_GESTURE_ROWS),
        EARFUN_FREE_PRO_3_PRODUCT_ID => Some(EARFUN_FREE_PRO_3_GESTURE_ROWS),
        EARFUN_AIR_S_PRODUCT_ID => Some(EARFUN_AIR_S_GESTURE_ROWS),
        _ => None,
    }
}

pub fn is_earfun_product(product_id: u32) -> bool {
    matches!(
        product_id,
        EARFUN_AIR_PRO_4_PRODUCT_ID | EARFUN_FREE_PRO_3_PRODUCT_ID | EARFUN_AIR_S_PRODUCT_ID
    )
}
