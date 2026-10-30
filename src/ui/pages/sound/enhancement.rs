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
use gtk::{Box as GtkBox, ListBox, Scale, Switch};

use crate::ui::components::{labeled_slider, preferences_content_row, switch_row};

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct BassEngineWidgets {
    pub(crate) bass_switch_row: adw::ActionRow,
    pub(crate) bass_switch: Switch,
    pub(crate) bass_slider_row: adw::PreferencesRow,
    pub(crate) bass_slider_box: GtkBox,
    pub(crate) bass_slider: Scale,
    pub(crate) bass_switch_flag: crate::ui::ReentrancyFlag,
    pub(crate) bass_slider_flag: crate::ui::ReentrancyFlag,
}

#[derive(Clone)]
pub(crate) struct EnhancementWidgets {
    pub(crate) card: adw::PreferencesGroup,
    pub(crate) hires_switch_row: adw::ActionRow,
    pub(crate) hires_switch: Switch,
    pub(crate) codec_row: adw::ComboRow,
    pub(crate) hires_switch_flag: crate::ui::ReentrancyFlag,
    pub(crate) codec_flag: crate::ui::ReentrancyFlag,
}

pub(crate) fn build_enhancement_card()
-> (adw::PreferencesGroup, BassEngineWidgets, EnhancementWidgets) {
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

    let bass_widgets = BassEngineWidgets {
        bass_switch_row,
        bass_switch,
        bass_slider_row,
        bass_slider_box,
        bass_slider,
        bass_switch_flag: crate::ui::ReentrancyFlag::new(),
        bass_slider_flag: crate::ui::ReentrancyFlag::new(),
    };

    let enhancement_widgets = EnhancementWidgets {
        card: enhancement_card.clone(),
        hires_switch_row,
        hires_switch,
        codec_row,
        hires_switch_flag: crate::ui::ReentrancyFlag::new(),
        codec_flag: crate::ui::ReentrancyFlag::new(),
    };

    (enhancement_card, bass_widgets, enhancement_widgets)
}
