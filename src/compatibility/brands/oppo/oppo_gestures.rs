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

use crate::compatibility::GestureRow;

pub static STANDARD_GESTURE_ROWS: &[GestureRow] = &[
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 1],
        default_function: Some(1),
        label_key: "controls-gesture-single",
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 1, 5, 6, 3, 17],
        default_function: Some(6),
        label_key: "controls-gesture-double",
    },
    GestureRow {
        action: crate::protocol::GestureAction::TripleSqueeze,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 5, 6, 3, 17],
        default_function: Some(5),
        label_key: "controls-gesture-triple",
    },
    GestureRow {
        action: crate::protocol::GestureAction::Slide,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: false,
        allowed_functions: &[0, 7, 10],
        default_function: Some(7),
        label_key: "controls-gesture-slide",
    },
    GestureRow {
        action: crate::protocol::GestureAction::SqueezeAndHold,
        is_call_control: false,
        button: 1,
        both_ears: false,
        edits_noise_cycle: true,
        allowed_functions: &[0, 8, 3, 17],
        default_function: Some(8),
        label_key: "controls-gesture-hold",
    },
    GestureRow {
        action: crate::protocol::GestureAction::SingleSqueeze,
        is_call_control: true,
        button: crate::protocol::KeyFunction::CALL_BUTTON,
        both_ears: true,
        edits_noise_cycle: false,
        allowed_functions: &[0, 29],
        default_function: Some(29),
        label_key: "controls-gesture-call-single",
    },
    GestureRow {
        action: crate::protocol::GestureAction::DoubleSqueeze,
        is_call_control: true,
        button: crate::protocol::KeyFunction::CALL_BUTTON,
        both_ears: true,
        edits_noise_cycle: false,
        allowed_functions: &[0, 28],
        default_function: Some(28),
        label_key: "controls-gesture-call-double",
    },
];

pub fn gesture_rows_for_product(_product_id: u32) -> &'static [GestureRow] {
    STANDARD_GESTURE_ROWS
}

pub fn gesture_function_label_key(function: u8) -> &'static str {
    match function {
        0 => "controls-function-none",
        1 => "controls-function-play-pause",
        3 => "controls-function-voice-assistant",
        5 => "controls-function-previous",
        6 => "controls-function-next",
        7 => "controls-function-volume-control",
        8 => "controls-function-noise-control",
        10 => "controls-function-switch-track",
        11 => "controls-function-volume-up",
        12 => "controls-function-volume-down",
        13 => "controls-function-switch-devices",
        17 => "controls-function-game-mode",
        28 => "controls-function-decline-end-call",
        29 => "controls-function-answer-end-call",
        _ => "controls-function-unknown",
    }
}
