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
use gtk::{ListBox, Scale, Switch};

use crate::{
    protocol::BassEngineLevel,
    ui::components::{labeled_slider, preferences_content_row, switch_row},
};

#[derive(Clone)]
pub(crate) struct BassEngineWidgets {
    switch_row: adw::ActionRow,
    switch: Switch,
    slider_row: adw::PreferencesRow,
    slider: Scale,
    switch_flag: crate::ui::ReentrancyFlag,
    slider_flag: crate::ui::ReentrancyFlag,
}

impl BassEngineWidgets {
    pub fn set_supported(&self, supported: bool) {
        self.switch_row.set_visible(supported);
        self.slider_row.set_visible(supported);
        self.switch.set_sensitive(supported);
        self.switch_row.set_sensitive(supported);
    }

    pub fn set_enabled(&self, enabled: bool) {
        let _guard = self.switch_flag.enter();
        self.switch.set_state(enabled);
        self.switch.set_active(enabled);
        self.slider.set_sensitive(enabled);
    }

    pub fn set_level(&self, level: BassEngineLevel) {
        let _guard = self.slider_flag.enter();
        let min = level.min as f64;
        let max = level.max as f64;
        self.slider.set_range(min, max);
        self.slider.set_increments(1.0, 1.0);
        self.slider.set_round_digits(0);
        self.slider.clear_marks();
        let mut v = min;
        while v <= max + 0.001 {
            self.slider.add_mark(v, gtk::PositionType::Bottom, None);
            v += 1.0;
        }
        self.slider.set_value(level.current as f64);
    }

    pub fn set_switch_sensitive(&self, sensitive: bool) {
        self.switch.set_sensitive(sensitive);
    }

    #[allow(dead_code)]
    pub fn set_slider_sensitive(&self, sensitive: bool) {
        self.slider.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        {
            let _guard = self.switch_flag.enter();
            self.switch.set_sensitive(false);
            self.switch.set_state(false);
            self.switch.set_active(false);
        }
        {
            let _guard = self.slider_flag.enter();
            self.slider.set_sensitive(false);
            self.slider.set_value(0.0);
        }
    }

    pub fn connect_switch_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let switch_flag = self.switch_flag.clone();
        let slider = self.slider.clone();
        self.switch.connect_state_set(move |switch, state| {
            if switch_flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            switch.set_sensitive(false);
            slider.set_sensitive(false);
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_slider_changed<F>(&self, callback: F)
    where
        F: Fn(i8) + 'static,
    {
        let slider_flag = self.slider_flag.clone();
        let slider_ref = self.slider.clone();
        self.slider.connect_value_changed(move |slider| {
            let rounded = slider.value().round();
            if (slider.value() - rounded).abs() > 0.001 {
                slider.set_value(rounded);
                return;
            }
            let val = rounded as i8;
            slider_ref.set_tooltip_text(Some(&crate::ui::sound_state::format_bass_level(val)));

            if slider_flag.is_updating() {
                return;
            }
            callback(val);
        });
    }
}

#[cfg(test)]
impl BassEngineWidgets {
    pub fn is_switch_active(&self) -> bool {
        self.switch.is_active()
    }

    pub fn is_switch_sensitive(&self) -> bool {
        self.switch.is_sensitive()
    }

    pub fn is_switch_row_visible(&self) -> bool {
        self.switch_row.is_visible()
    }

    pub fn is_slider_row_visible(&self) -> bool {
        self.slider_row.is_visible()
    }

    pub fn is_slider_sensitive(&self) -> bool {
        self.slider.is_sensitive()
    }

    pub fn slider_value(&self) -> f64 {
        self.slider.value()
    }

    pub fn slider_range(&self) -> (f64, f64) {
        let adj = self.slider.adjustment();
        (adj.lower(), adj.upper())
    }
}

#[derive(Clone)]
pub(crate) struct EnhancementWidgets {
    card: adw::PreferencesGroup,
    hires_switch_row: adw::ActionRow,
    hires_switch: Switch,
    codec_row: adw::ComboRow,
    hires_switch_flag: crate::ui::ReentrancyFlag,
    codec_flag: crate::ui::ReentrancyFlag,
}

impl EnhancementWidgets {
    pub fn set_card_visible(&self, visible: bool) {
        self.card.set_visible(visible);
    }

    pub fn set_hires_supported(&self, supported: bool) {
        self.hires_switch_row.set_visible(supported);
        self.hires_switch.set_sensitive(supported);
        self.hires_switch_row.set_sensitive(supported);
    }

    pub fn set_hires_enabled(&self, enabled: bool) {
        let _guard = self.hires_switch_flag.enter();
        self.hires_switch.set_state(enabled);
        self.hires_switch.set_active(enabled);
    }

    pub fn set_hires_sensitive(&self, sensitive: bool) {
        self.hires_switch.set_sensitive(sensitive);
    }

    pub fn set_codec_supported(&self, supported: bool) {
        self.codec_row.set_visible(supported);
        self.codec_row.set_sensitive(supported);
    }

    pub fn set_available_codecs(&self, codecs: &[(u8, String)]) {
        let _guard = self.codec_flag.enter();
        crate::ui::components::ComboRowExtManual::set_items(&self.codec_row, codecs);
    }

    pub fn set_selected_codec(&self, codec: Option<u8>) {
        let _guard = self.codec_flag.enter();
        crate::ui::components::ComboRowExtManual::set_selected_id(&self.codec_row, codec);
    }

    pub fn set_codec_sensitive(&self, sensitive: bool) {
        self.codec_row.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        self.card.set_visible(false);
        {
            let _guard = self.codec_flag.enter();
            self.codec_row.set_sensitive(false);
            self.codec_row.set_visible(false);
            crate::ui::components::ComboRowExtManual::set_selected_id(&self.codec_row, None);
        }
        {
            let _guard = self.hires_switch_flag.enter();
            self.hires_switch.set_sensitive(false);
            self.hires_switch.set_state(false);
            self.hires_switch.set_active(false);
        }
    }

    pub fn connect_hires_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let hires_switch_flag = self.hires_switch_flag.clone();
        self.hires_switch.connect_state_set(move |switch, state| {
            if hires_switch_flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            switch.set_sensitive(false);
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_codec_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let codec_flag = self.codec_flag.clone();
        self.codec_row.connect_selected_notify(move |row| {
            if codec_flag.is_updating() {
                return;
            }
            if let Some(codec_idx) = crate::ui::components::ComboRowExtManual::selected_id(row) {
                callback(codec_idx);
            }
        });
    }
}

#[cfg(test)]
impl EnhancementWidgets {
    pub fn is_card_visible(&self) -> bool {
        self.card.is_visible()
    }

    pub fn is_hires_visible(&self) -> bool {
        self.hires_switch_row.is_visible()
    }

    pub fn is_hires_sensitive(&self) -> bool {
        self.hires_switch.is_sensitive()
    }

    pub fn is_hires_active(&self) -> bool {
        self.hires_switch.is_active()
    }

    pub fn is_codec_visible(&self) -> bool {
        self.codec_row.is_visible()
    }

    pub fn is_codec_sensitive(&self) -> bool {
        self.codec_row.is_sensitive()
    }

    pub fn selected_codec(&self) -> Option<u8> {
        crate::ui::components::ComboRowExtManual::selected_id(&self.codec_row)
    }
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
        switch_row: bass_switch_row,
        switch: bass_switch,
        slider_row: bass_slider_row,
        slider: bass_slider,
        switch_flag: crate::ui::ReentrancyFlag::new(),
        slider_flag: crate::ui::ReentrancyFlag::new(),
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
