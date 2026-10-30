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
pub(crate) struct SpatialAudioWidgets {
    card: adw::PreferencesGroup,
    spatial_radios: Vec<(u8, CheckButton, adw::ActionRow)>,
    flag: crate::ui::ReentrancyFlag,
}

impl SpatialAudioWidgets {
    pub fn set_supported(&self, supports_spatial: bool, supports_head_tracking: bool) {
        self.card.set_visible(supports_spatial);
        let _guard = self.flag.enter();
        for (mode, radio, row) in &self.spatial_radios {
            match *mode {
                0 | 1 => {
                    row.set_visible(true);
                    radio.set_sensitive(supports_spatial);
                }
                2 => {
                    row.set_visible(supports_head_tracking);
                    radio.set_sensitive(supports_spatial && supports_head_tracking);
                }
                _ => {}
            }
        }
    }

    pub fn set_selected(&self, current_spatial: Option<u8>) {
        let _guard = self.flag.enter();
        if let Some(selected) = current_spatial {
            for (mode, radio, _) in &self.spatial_radios {
                if *mode == selected {
                    radio.set_active(true);
                }
            }
        }
    }

    pub fn reset(&self) {
        self.card.set_visible(false);
        let _guard = self.flag.enter();
        for (_, radio, _) in &self.spatial_radios {
            radio.set_sensitive(false);
        }
    }

    pub fn connect_mode_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + Clone + 'static,
    {
        for (mode, radio, _) in &self.spatial_radios {
            let mode = *mode;
            let radio = radio.clone();
            let flag = self.flag.clone();
            let cb = callback.clone();
            radio.connect_toggled(move |r| {
                if flag.is_updating() || !r.is_active() {
                    return;
                }
                cb(mode);
            });
        }
    }
}

#[cfg(test)]
impl SpatialAudioWidgets {
    pub fn is_card_visible(&self) -> bool {
        self.card.is_visible()
    }

    pub fn is_mode_visible(&self, mode: u8) -> bool {
        self.spatial_radios
            .iter()
            .find(|(m, _, _)| *m == mode)
            .map(|(_, _, row)| row.is_visible())
            .unwrap_or(false)
    }

    pub fn is_mode_sensitive(&self, mode: u8) -> bool {
        self.spatial_radios
            .iter()
            .find(|(m, _, _)| *m == mode)
            .map(|(_, radio, _)| radio.is_sensitive())
            .unwrap_or(false)
    }

    pub fn is_mode_active(&self, mode: u8) -> bool {
        self.spatial_radios
            .iter()
            .find(|(m, _, _)| *m == mode)
            .map(|(_, radio, _)| radio.is_active())
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.spatial_radios.len()
    }

    pub fn first_child_exists(&self) -> bool {
        self.card.first_child().is_some()
    }
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
        spatial_radios,
        flag: crate::ui::ReentrancyFlag::new(),
    };

    (spatial_card, widgets)
}
