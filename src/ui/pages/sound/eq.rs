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
use crate::ui::components::radio_row;

#[derive(Clone)]
pub(crate) struct EqualizerWidgets {
    pub(crate) card: adw::PreferencesGroup,
    pub(crate) preset_list: ListBox,
    pub(crate) preset_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
    pub(crate) first_radio: CheckButton,
    pub(crate) flag: crate::ui::ReentrancyFlag,
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

    preset_list.append(&custom_eq.add_custom_row);
    eq_card.add(&preset_list);
    eq_card.add(&custom_eq.custom_list);
    eq_card.add(&custom_eq.custom_editor);

    let widgets = EqualizerWidgets {
        card: eq_card.clone(),
        preset_list,
        preset_radios,
        first_radio: first_radio.expect("first_radio should be initialized"),
        flag: crate::ui::ReentrancyFlag::new(),
    };

    (eq_card, widgets)
}

