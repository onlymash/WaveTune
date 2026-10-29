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
use gtk::{Align, Box as GtkBox, Label, Orientation, Scale};

/// Creates a slider layout with the slider on top and descriptive text labels underneath on both sides (e.g. "弱" / "强").
#[allow(dead_code)]
pub(crate) fn labeled_slider(
    min_label: &str,
    max_label: &str,
    min: f64,
    max: f64,
    step: f64,
) -> (GtkBox, Scale) {
    let container = GtkBox::new(Orientation::Vertical, 2);
    container.set_hexpand(true);

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

    let labels_row = GtkBox::new(Orientation::Horizontal, 0);
    labels_row.set_hexpand(true);
    labels_row.set_margin_start(6);
    labels_row.set_margin_end(6);
    labels_row.set_margin_top(6);
    labels_row.set_margin_bottom(6);

    let left = Label::builder()
        .label(min_label)
        .css_classes(["dim-label", "caption"])
        .halign(Align::Start)
        .hexpand(true)
        .wrap(false)
        .build();

    let right = Label::builder()
        .label(max_label)
        .css_classes(["dim-label", "caption"])
        .halign(Align::End)
        .hexpand(true)
        .wrap(false)
        .build();

    labels_row.append(&left);
    labels_row.append(&right);

    container.append(&slider);
    container.append(&labels_row);

    (container, slider)
}
