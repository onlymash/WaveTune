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
use gtk::{CheckButton, ListBox};

use super::custom_eq::CustomEqualizerWidgets;
use crate::{compatibility::EqualizerPreset, ui::components::radio_row};

#[derive(Clone)]
pub(crate) struct EqualizerWidgets {
    card: adw::PreferencesGroup,
    preset_list: ListBox,
    preset_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
    first_radio: CheckButton,
    flag: crate::ui::ReentrancyFlag,
}

impl EqualizerWidgets {
    pub fn set_supported(&self, supports_equalizer: bool, presets: &[EqualizerPreset]) {
        self.card.set_visible(supports_equalizer);
        let _guard = self.flag.enter();
        for (idx, radio, row) in &self.preset_radios {
            if let Some(matching) = presets.iter().find(|p| p.index == *idx) {
                row.set_visible(true);
                row.set_title(&crate::t!(matching.key));
                radio.set_sensitive(supports_equalizer);
            } else {
                row.set_visible(false);
                radio.set_sensitive(false);
            }
        }
    }

    pub fn set_selected(&self, selected_preset: Option<u8>) {
        let _guard = self.flag.enter();
        if let Some(selected) = selected_preset {
            for (idx, radio, _) in &self.preset_radios {
                if *idx == selected {
                    radio.set_active(true);
                }
            }
        }
    }

    pub fn reset(&self) {
        self.card.set_visible(false);
        let _guard = self.flag.enter();
        for (_, radio, _) in &self.preset_radios {
            radio.set_sensitive(false);
        }
    }

    pub fn connect_preset_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + Clone + 'static,
    {
        for (preset_index, radio, _) in &self.preset_radios {
            let preset_index = *preset_index;
            let radio = radio.clone();
            let flag = self.flag.clone();
            let cb = callback.clone();
            radio.connect_toggled(move |r| {
                if flag.is_updating() || !r.is_active() {
                    return;
                }
                cb(preset_index);
            });
        }
    }

    pub fn flag(&self) -> &crate::ui::ReentrancyFlag {
        &self.flag
    }

    pub fn first_radio(&self) -> &CheckButton {
        &self.first_radio
    }

    pub fn preset_list(&self) -> &ListBox {
        &self.preset_list
    }
}

#[cfg(test)]
impl EqualizerWidgets {
    pub fn is_card_visible(&self) -> bool {
        self.card.is_visible()
    }

    pub fn is_preset_visible(&self, idx: u8) -> bool {
        self.preset_radios
            .iter()
            .find(|(i, _, _)| *i == idx)
            .map(|(_, _, row)| row.is_visible())
            .unwrap_or(false)
    }

    pub fn is_preset_sensitive(&self, idx: u8) -> bool {
        self.preset_radios
            .iter()
            .find(|(i, _, _)| *i == idx)
            .map(|(_, radio, _)| radio.is_sensitive())
            .unwrap_or(false)
    }

    pub fn is_preset_active(&self, idx: u8) -> bool {
        self.preset_radios
            .iter()
            .find(|(i, _, _)| *i == idx)
            .map(|(_, radio, _)| radio.is_active())
            .unwrap_or(false)
    }

    pub fn preset_count(&self) -> usize {
        self.preset_radios.len()
    }

    pub fn first_child_exists(&self) -> bool {
        self.preset_list.first_child().is_some()
    }
}

pub(crate) fn build_eq_card(
    custom_eq: &CustomEqualizerWidgets,
) -> (adw::PreferencesGroup, EqualizerWidgets) {
    let eq_card = adw::PreferencesGroup::builder()
        .title(&crate::t!("equalizer-title"))
        .description(&crate::t!("equalizer-desc"))
        .build();

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

    preset_list.append(custom_eq.add_custom_row());
    eq_card.add(&preset_list);
    eq_card.add(custom_eq.custom_list());
    eq_card.add(custom_eq.custom_editor());

    let widgets = EqualizerWidgets {
        card: eq_card.clone(),
        preset_list,
        preset_radios,
        first_radio: first_radio.expect("first_radio should be initialized"),
        flag: crate::ui::ReentrancyFlag::new(),
    };

    (eq_card, widgets)
}
