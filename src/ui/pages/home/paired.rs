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

use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Label, Orientation};

use crate::ui::components::configure_label_wrapping;

#[derive(Clone)]
pub(crate) struct PairedDevicesCardWidgets {
    paired_devices_section: GtkBox,
    paired_devices: gtk::ListBox,
}

impl PairedDevicesCardWidgets {
    pub fn set_visible(&self, visible: bool) {
        self.paired_devices_section.set_visible(visible);
    }

    pub fn clear_items(&self) {
        while let Some(child) = self.paired_devices.first_child() {
            self.paired_devices.remove(&child);
        }
    }

    pub fn append_row(&self, row: &impl IsA<gtk::Widget>) {
        self.paired_devices.append(row);
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl PairedDevicesCardWidgets {
    pub fn section(&self) -> &GtkBox {
        &self.paired_devices_section
    }

    pub fn paired_devices(&self) -> &gtk::ListBox {
        &self.paired_devices
    }

    pub fn reset(&self) {
        self.set_visible(true);
        self.clear_items();
    }
}

pub(crate) fn build_paired_devices_card() -> (GtkBox, PairedDevicesCardWidgets) {
    let paired_devices_section = GtkBox::new(Orientation::Vertical, 0);
    paired_devices_section.set_hexpand(true);
    paired_devices_section.set_halign(Align::Fill);

    let paired_devices_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("device-management"))
        .build();

    let paired_devices = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let placeholder = Label::builder()
        .label(&crate::t!("dual-device-read-after-connecting"))
        .xalign(0.5)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(["dim-label"])
        .build();
    configure_label_wrapping(&placeholder);
    paired_devices.set_placeholder(Some(&placeholder));

    paired_devices_group.add(&paired_devices);
    paired_devices_section.append(&paired_devices_group);

    let widgets = PairedDevicesCardWidgets {
        paired_devices_section: paired_devices_section.clone(),
        paired_devices,
    };

    (paired_devices_section, widgets)
}
