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
use gtk::{Orientation, Scale};

use crate::{
    protocol::{CustomEqualizer, DEFAULT_EQ_FREQUENCIES, DEFAULT_MAX_GAIN, DEFAULT_MIN_GAIN},
    ui::components::preferences_content_row,
};

pub fn show_custom_eq_dialog(
    parent: &impl IsA<gtk::Widget>,
    initial: Option<&CustomEqualizer>,
    default_name: &str,
    on_save: impl Fn(String, Vec<(u16, i8)>) + 'static,
) {
    let is_edit = initial.is_some();
    let title = if is_edit {
        crate::t!("equalizer-custom-edit-title")
    } else {
        crate::t!("equalizer-custom-add-title")
    };

    let dialog = adw::Dialog::builder()
        .content_width(420)
        .title(&title)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    let header_bar = adw::HeaderBar::new();

    let cancel_btn = gtk::Button::builder()
        .label(&crate::t!("common-cancel"))
        .build();
    header_bar.pack_start(&cancel_btn);

    let save_label = if is_edit {
        crate::t!("equalizer-custom-save")
    } else {
        crate::t!("equalizer-custom-add")
    };
    let save_btn = gtk::Button::builder()
        .label(&save_label)
        .css_classes(["suggested-action"])
        .build();
    header_bar.pack_end(&save_btn);

    toolbar_view.add_top_bar(&header_bar);

    let page = adw::PreferencesPage::new();

    let name_group = adw::PreferencesGroup::new();
    let current_name = initial.map(|p| p.name.as_str()).unwrap_or(default_name);
    let name_row = adw::EntryRow::builder()
        .title(&crate::t!("equalizer-custom-name"))
        .text(current_name)
        .build();
    name_group.add(&name_row);
    page.add(&name_group);

    let bands_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("equalizer-custom-title"))
        .build();

    let band_box = gtk::Box::new(Orientation::Horizontal, 8);
    band_box.set_homogeneous(true);
    band_box.set_margin_top(8);
    band_box.set_margin_bottom(8);
    band_box.set_margin_start(10);
    band_box.set_margin_end(10);

    let freq_labels = [
        "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ];
    let mut scales = Vec::new();

    for (i, &freq) in DEFAULT_EQ_FREQUENCIES.iter().enumerate() {
        let initial_gain = initial
            .and_then(|p| p.bands.iter().find(|(f, _)| *f == freq).map(|(_, g)| *g))
            .unwrap_or(0);

        let band_col = gtk::Box::new(Orientation::Vertical, 4);
        band_col.set_halign(gtk::Align::Center);

        let val_label = gtk::Label::builder()
            .label(&format!("{initial_gain}"))
            .css_classes(["dim-label", "caption"])
            .halign(gtk::Align::Center)
            .build();

        let scale = Scale::with_range(
            Orientation::Vertical,
            DEFAULT_MIN_GAIN as f64,
            DEFAULT_MAX_GAIN as f64,
            1.0,
        );
        scale.set_inverted(true);
        scale.set_value(initial_gain as f64);
        scale.set_height_request(130);
        scale.set_draw_value(false);

        let f_label = gtk::Label::builder()
            .label(freq_labels[i])
            .css_classes(["caption"])
            .halign(gtk::Align::Center)
            .build();

        {
            let val_label = val_label.clone();
            scale.connect_value_changed(move |s| {
                let v = s.value().round() as i32;
                val_label.set_label(&format!("{v}"));
            });
        }

        band_col.append(&val_label);
        band_col.append(&scale);
        band_col.append(&f_label);
        band_box.append(&band_col);

        scales.push((freq, scale));
    }

    let pref_row = preferences_content_row(&band_box);
    bands_group.add(&pref_row);
    page.add(&bands_group);

    toolbar_view.set_content(Some(&page));
    dialog.set_child(Some(&toolbar_view));

    {
        let dialog = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dialog.close();
        });
    }

    {
        let dialog = dialog.clone();
        let name_row = name_row.clone();
        let fallback_name = default_name.to_string();
        save_btn.connect_clicked(move |_| {
            let mut name = name_row.text().as_str().trim().to_string();
            if name.is_empty() {
                name = fallback_name.clone();
            }
            let bands = scales
                .iter()
                .map(|(f, s)| (*f, s.value().round() as i8))
                .collect();
            on_save(name, bands);
            dialog.close();
        });
    }

    dialog.present(Some(parent));
}
