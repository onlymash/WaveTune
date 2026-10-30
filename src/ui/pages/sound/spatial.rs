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

use crate::ui::components::radio_row;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SpatialAudioWidgets {
    pub(crate) card: adw::PreferencesGroup,
    pub(crate) spatial_list: ListBox,
    pub(crate) spatial_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
    pub(crate) flag: crate::ui::ReentrancyFlag,
}

pub(crate) fn build_spatial_card() -> (adw::PreferencesGroup, SpatialAudioWidgets) {
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

    let widgets = SpatialAudioWidgets {
        card: spatial_card.clone(),
        spatial_list,
        spatial_radios,
        flag: crate::ui::ReentrancyFlag::new(),
    };

    (spatial_card, widgets)
}
