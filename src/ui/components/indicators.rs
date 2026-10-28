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
use gtk::{Align, Box as GtkBox, Image, Label, Orientation};

#[derive(Clone)]
pub(crate) struct BatteryIndicator {
    pub(crate) percentage: Label,
    pub(crate) charging: Label,
}

pub(crate) fn battery_indicator(letter: Option<&str>) -> (GtkBox, BatteryIndicator) {
    let item = GtkBox::new(Orientation::Horizontal, 6);
    item.set_halign(Align::Center);
    item.set_valign(Align::Center);

    let icon_background = GtkBox::new(Orientation::Vertical, 0);
    icon_background.set_halign(Align::Center);
    icon_background.set_valign(Align::Center);
    icon_background.set_size_request(20, 20);
    icon_background.set_css_classes(&["battery-device-icon"]);
    if let Some(letter) = letter {
        let icon = Label::builder()
            .label(letter)
            .css_classes(["battery-device-letter"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon_background.append(&icon);
    } else {
        let icon = Image::from_icon_name("audio-headphones-symbolic");
        icon.set_pixel_size(12);
        icon.set_halign(Align::Center);
        icon.set_valign(Align::Center);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon_background.append(&icon);
    }

    let percentage = Label::builder()
        .label("—")
        .valign(Align::Center)
        .css_classes(["battery-value"])
        .build();
    let charging = Label::builder()
        .label("⚡︎")
        .valign(Align::Center)
        .css_classes(["battery-charging"])
        .visible(false)
        .build();
    charging.set_tooltip_text(Some(&crate::t!("battery-charging")));

    item.append(&icon_background);
    item.append(&percentage);
    item.append(&charging);

    (
        item,
        BatteryIndicator {
            percentage,
            charging,
        },
    )
}
