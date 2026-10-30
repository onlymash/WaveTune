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
use gtk::{Box as GtkBox, Button, CheckButton, Entry, ListBox, Orientation, Scale, Switch};

use crate::ui::components::{
    AdaptiveColumns, labeled_slider, muted, page_container, preferences_content_row, radio_row,
    switch_row,
};

pub mod state;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SoundWidgets {
    pub(crate) sound_columns: AdaptiveColumns,
    pub(crate) eq_card: adw::PreferencesGroup,
    pub(crate) preset_list: ListBox,
    pub(crate) preset_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
    pub(crate) first_radio: CheckButton,
    pub(crate) custom_preset_rows:
        std::rc::Rc<std::cell::RefCell<Vec<(u8, CheckButton, adw::ActionRow)>>>,
    pub(crate) custom_equalizers:
        std::rc::Rc<std::cell::RefCell<Vec<crate::protocol::CustomEqualizer>>>,
    pub(crate) custom_list: ListBox,
    pub(crate) add_custom_row: adw::ActionRow,
    pub(crate) custom_editor: GtkBox,
    pub(crate) custom_name_entry: Entry,
    pub(crate) rename_btn: Button,
    pub(crate) delete_btn: Button,
    pub(crate) band_box: GtkBox,
    pub(crate) enhancement_card: adw::PreferencesGroup,
    pub(crate) hires_switch_row: adw::ActionRow,
    pub(crate) hires_switch: Switch,
    pub(crate) codec_row: adw::ComboRow,
    pub(crate) bass_switch_row: adw::ActionRow,
    pub(crate) bass_switch: Switch,
    pub(crate) bass_slider_row: adw::PreferencesRow,
    pub(crate) bass_slider_box: GtkBox,
    pub(crate) bass_slider: Scale,
    pub(crate) spatial_card: adw::PreferencesGroup,
    pub(crate) spatial_list: ListBox,
    pub(crate) spatial_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
}

pub(crate) fn build_sound_page() -> (GtkBox, SoundWidgets) {
    let page = page_container();
    let sound_columns = AdaptiveColumns::new();

    // === LEFT COLUMN: EQ Card ===
    let eq_card = adw::PreferencesGroup::builder()
        .title(&crate::t!("equalizer-title"))
        .description(&crate::t!("equalizer-desc"))
        .build();

    // Built-in presets boxed list
    let preset_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let mut preset_radios = Vec::new();
    let mut first_radio: Option<CheckButton> = None;

    let default_presets: [(u8, &str); 9] = [
        (0, &crate::t!("equalizer-preset-dynaudio-classic")),
        (7, &crate::t!("equalizer-preset-dynaudio-hi-res")),
        (2, &crate::t!("equalizer-preset-pure-vocal")),
        (1, &crate::t!("equalizer-preset-clear-treble")),
        (3, &crate::t!("equalizer-preset-bass-boost")),
        (4, &crate::t!("equalizer-preset-soft")),
        (5, &crate::t!("equalizer-preset-dynamic")),
        (6, &crate::t!("equalizer-preset-clear")),
        (8, &crate::t!("equalizer-preset-natural")),
    ];

    for (index, name) in default_presets {
        let (row, radio) = radio_row(name, first_radio.as_ref());
        if first_radio.is_none() {
            first_radio = Some(radio.clone());
        }
        preset_list.append(&row);
        preset_radios.push((index, radio, row));
    }

    let add_custom_row = adw::ActionRow::builder()
        .title(&crate::t!("equalizer-add-custom"))
        .activatable(true)
        .visible(false)
        .build();
    add_custom_row.set_sensitive(false);
    let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
    add_custom_row.add_prefix(&add_icon);
    preset_list.append(&add_custom_row);

    eq_card.add(&preset_list);

    // Custom EQ list (initially empty until device reports or user creates custom presets)
    let custom_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();
    eq_card.add(&custom_list);

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
    eq_card.add(&custom_editor);

    sound_columns.left().append(&eq_card);

    // === RIGHT COLUMN ===
    // 1. Audio Enhancement Card (Hi-Res Mode + BassWave)
    let enhancement_card = adw::PreferencesGroup::builder()
        .title(&crate::t!("sound-audio-enhancement"))
        .build();

    let (hires_switch_row, hires_switch) = switch_row(
        &crate::t!("hi-res-title"),
        Some(&crate::t!("hi-res-summary")),
    );
    hires_switch.set_sensitive(false);
    hires_switch_row.set_sensitive(false);

    let codec_row = crate::ui::components::combo_row(
        &crate::t!("audio-codec-title"),
        Some(&crate::t!("audio-codec-summary")),
    );
    codec_row.set_sensitive(false);
    codec_row.set_visible(false);

    let (bass_switch_row, bass_switch) = switch_row(
        &crate::t!("bass-engine-title"),
        Some(&crate::t!("bass-engine-summary")),
    );

    let (bass_slider_box, bass_slider) = labeled_slider(
        &crate::t!("bass-engine-weak"),
        &crate::t!("bass-engine-strong"),
        -5.0,
        5.0,
        1.0,
    );
    bass_slider.set_value(0.0);
    bass_slider.set_sensitive(false);
    bass_slider_box.set_margin_start(12);
    bass_slider_box.set_margin_end(12);
    bass_slider_box.set_margin_top(4);
    bass_slider_box.set_margin_bottom(6);
    let bass_slider_row = preferences_content_row(&bass_slider_box);

    let enhancement_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    enhancement_list.append(&hires_switch_row);
    enhancement_list.append(&codec_row);
    enhancement_list.append(&bass_switch_row);
    enhancement_list.append(&bass_slider_row);
    enhancement_card.add(&enhancement_list);

    sound_columns.right().append(&enhancement_card);

    // 2. Spatial Audio Card
    let spatial_card = adw::PreferencesGroup::builder()
        .title(&crate::t!("spatial-audio-title"))
        .description(&crate::t!("spatial-audio-effects-desc"))
        .build();

    let spatial_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let mut spatial_radios = Vec::new();
    let mut first_spatial_radio: Option<CheckButton> = None;

    let spatial_modes: [(u8, &str); 3] = [
        (0, &crate::t!("common-off")),
        (1, &crate::t!("spatial-audio-mode-fixed")),
        (2, &crate::t!("spatial-audio-mode-head-tracking")),
    ];

    for (mode, name) in spatial_modes {
        let (row, radio) = radio_row(name, first_spatial_radio.as_ref());
        if first_spatial_radio.is_none() {
            first_spatial_radio = Some(radio.clone());
        }
        spatial_list.append(&row);
        spatial_radios.push((mode, radio, row));
    }
    spatial_card.add(&spatial_list);

    sound_columns.right().append(&spatial_card);

    page.append(sound_columns.widget());

    (
        page,
        SoundWidgets {
            sound_columns,
            eq_card,
            preset_list,
            preset_radios,
            first_radio: first_radio.expect("first_radio should be initialized"),
            custom_preset_rows: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            custom_equalizers: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            custom_list,
            add_custom_row,
            custom_editor,
            custom_name_entry,
            rename_btn,
            delete_btn,
            band_box,
            enhancement_card,
            hires_switch_row,
            hires_switch,
            codec_row,
            bass_switch_row,
            bass_switch,
            bass_slider_row,
            bass_slider_box,
            bass_slider,
            spatial_card,
            spatial_list,
            spatial_radios,
        },
    )
}
