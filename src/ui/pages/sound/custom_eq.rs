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

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, Entry, ListBox, Orientation, Scale};

use crate::ui::components::muted;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct CustomEqualizerWidgets {
    pub(crate) preset_rows: Rc<RefCell<Vec<(u8, CheckButton, adw::ActionRow)>>>,
    pub(crate) equalizers: Rc<RefCell<Vec<crate::protocol::CustomEqualizer>>>,
    pub(crate) custom_list: ListBox,
    pub(crate) add_custom_row: adw::ActionRow,
    pub(crate) custom_editor: GtkBox,
    pub(crate) custom_name_entry: Entry,
    pub(crate) rename_btn: Button,
    pub(crate) delete_btn: Button,
    pub(crate) band_box: GtkBox,
}

pub(crate) fn build_custom_eq_section() -> CustomEqualizerWidgets {
    let add_custom_row = adw::ActionRow::builder()
        .title(&crate::t!("equalizer-add-custom"))
        .activatable(true)
        .visible(false)
        .build();
    add_custom_row.set_sensitive(false);
    let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
    add_custom_row.add_prefix(&add_icon);

    // Custom EQ list (initially empty until device reports or user creates custom presets)
    let custom_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();

    // Custom EQ Editor (initially hidden until custom EQ is active)
    let custom_editor = GtkBox::new(Orientation::Vertical, 10);
    custom_editor.set_visible(false);

    let editor_top_row = GtkBox::new(Orientation::Horizontal, 8);
    let custom_name_entry = Entry::builder()
        .placeholder_text(&crate::t!("equalizer-custom-title"))
        .hexpand(true)
        .max_length(42)
        .build();
    let rename_btn = Button::with_label(&crate::t!("common-rename"));
    let delete_btn = Button::with_label(&crate::t!("common-delete"));
    delete_btn.add_css_class("destructive-action");
    editor_top_row.append(&custom_name_entry);
    editor_top_row.append(&rename_btn);
    editor_top_row.append(&delete_btn);
    custom_editor.append(&editor_top_row);

    // 10 Frequency Bands: 31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz
    let band_box = GtkBox::new(Orientation::Horizontal, 6);
    band_box.set_homogeneous(true);
    let frequencies = [
        "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ];
    for freq in frequencies {
        let band_col = GtkBox::new(Orientation::Vertical, 4);
        band_col.set_halign(gtk::Align::Center);
        let val_label = muted("0");
        val_label.set_halign(gtk::Align::Center);
        let scale = Scale::with_range(Orientation::Vertical, -6.0, 6.0, 1.0);
        scale.set_inverted(true); // Higher values at the top
        scale.set_height_request(130);
        scale.set_draw_value(false);
        scale.set_sensitive(false);
        let freq_label = muted(freq);
        freq_label.set_halign(gtk::Align::Center);

        band_col.append(&val_label);
        band_col.append(&scale);
        band_col.append(&freq_label);
        band_box.append(&band_col);
    }
    custom_editor.append(&band_box);

    CustomEqualizerWidgets {
        preset_rows: Rc::new(RefCell::new(Vec::new())),
        equalizers: Rc::new(RefCell::new(Vec::new())),
        custom_list,
        add_custom_row,
        custom_editor,
        custom_name_entry,
        rename_btn,
        delete_btn,
        band_box,
    }
}
