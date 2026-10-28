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

use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Orientation, Scale};

use super::styles::muted;

/// Creates a horizontal slider row bounded by descriptive text labels on both sides (e.g. "弱" / "强")
/// that are vertically aligned with the slider trough.
#[allow(dead_code)]
pub(crate) fn labeled_slider(
    min_label: &str,
    max_label: &str,
    min: f64,
    max: f64,
    step: f64,
) -> (GtkBox, Scale) {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_hexpand(true);
    row.set_valign(Align::Center);

    let left = muted(min_label);
    left.set_valign(Align::Center);

    let slider = Scale::with_range(Orientation::Horizontal, min, max, step);
    slider.add_css_class("labeled-slider");
    slider.set_hexpand(true);
    slider.set_valign(Align::Center);
    slider.set_draw_value(true);
    slider.set_round_digits(0);
    slider.set_increments(step, step);
    slider.set_format_value_func(|_, val| {
        crate::ui::sound_state::format_bass_level(val.round() as i8)
    });
    if step > 0.0 {
        let mut v = min;
        while v <= max + 0.001 {
            slider.add_mark(v, gtk::PositionType::Bottom, None);
            v += step;
        }
    }

    let right = muted(max_label);
    right.set_valign(Align::Center);

    row.append(&left);
    row.append(&slider);
    row.append(&right);

    (row, slider)
}
