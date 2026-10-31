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
use gtk::{Align, Box as GtkBox, Orientation, Switch};

use crate::{
    compatibility::{self, EqualizerPreset},
    protocol::CustomEqualizer,
    ui::{
        ReentrancyFlag,
        components::{ComboRowExtManual, combo_row, switch_row},
    },
};

#[derive(Clone, Copy)]
pub(crate) struct HomeSoundFeaturesViewState<'a> {
    pub supports_equalizer: bool,
    pub supports_spatial_audio: bool,
    pub supports_head_tracking: bool,
    pub supports_game_mode: bool,
    pub presets: &'a [EqualizerPreset],
    pub custom_equalizers: &'a [CustomEqualizer],
}

#[derive(Clone)]
pub(crate) struct QuickSoundWidgets {
    sound_card_section: GtkBox,
    eq_row: adw::ComboRow,
    spatial_row: adw::ComboRow,
    game_mode_section: adw::ActionRow,
    game_mode_switch: Switch,
    eq_flag: ReentrancyFlag,
    spatial_flag: ReentrancyFlag,
    game_mode_flag: ReentrancyFlag,
}

impl QuickSoundWidgets {
    pub fn update_sound_features(&self, state: &HomeSoundFeaturesViewState) {
        let any_supported =
            state.supports_equalizer || state.supports_spatial_audio || state.supports_game_mode;
        self.sound_card_section.set_visible(any_supported);

        self.update_equalizer_items(
            state.supports_equalizer,
            state.presets,
            state.custom_equalizers,
            None,
        );

        self.spatial_row.set_visible(state.supports_spatial_audio);
        {
            let _guard = self.spatial_flag.enter();
            let prev_spatial = self.spatial_row.selected_id();
            self.spatial_row.clear_items();
            let supported_modes = compatibility::spatial_audio_modes_for_profile(
                state.supports_spatial_audio,
                state.supports_head_tracking,
            );
            let spatial_items: Vec<(u8, String)> = supported_modes
                .iter()
                .map(|(mode, label)| (*mode, crate::t!(*label)))
                .collect();
            self.spatial_row.set_items(&spatial_items);
            if let Some(id) = prev_spatial {
                self.spatial_row.set_selected_id(Some(id));
            }
            self.spatial_row
                .set_sensitive(state.supports_spatial_audio && !supported_modes.is_empty());
        }

        self.game_mode_section.set_visible(state.supports_game_mode);
        if !state.supports_game_mode {
            let _guard = self.game_mode_flag.enter();
            self.game_mode_switch.set_state(false);
            self.game_mode_switch.set_active(false);
            self.game_mode_switch.set_sensitive(false);
        }
    }

    pub fn update_equalizer_items(
        &self,
        supports_equalizer: bool,
        presets: &[EqualizerPreset],
        custom_equalizers: &[CustomEqualizer],
        selected_id: Option<u8>,
    ) {
        self.eq_row.set_visible(supports_equalizer);
        let _guard = self.eq_flag.enter();
        let prev_eq = selected_id.or_else(|| self.eq_row.selected_id());
        self.eq_row.clear_items();
        let mut eq_items: Vec<(u8, String)> = presets
            .iter()
            .map(|preset| (preset.index, crate::t!(preset.key)))
            .collect();
        for custom in custom_equalizers {
            eq_items.push((custom.id, custom.name.clone()));
        }
        self.eq_row.set_items(&eq_items);
        if let Some(id) = prev_eq {
            self.eq_row.set_selected_id(Some(id));
        }
        self.eq_row
            .set_sensitive(supports_equalizer && !eq_items.is_empty());
    }

    pub fn update_equalizer(&self, current_preset: Option<u8>) {
        let _guard = self.eq_flag.enter();
        self.eq_row.set_selected_id(current_preset);
    }

    pub fn update_spatial_audio(&self, current_mode: Option<u8>) {
        let _guard = self.spatial_flag.enter();
        self.spatial_row.set_selected_id(current_mode);
    }

    pub fn update_game_mode(&self, enabled: Option<bool>) {
        let _guard = self.game_mode_flag.enter();
        if let Some(enabled) = enabled {
            self.game_mode_switch.set_state(enabled);
            self.game_mode_switch.set_active(enabled);
            self.game_mode_switch.set_sensitive(true);
        } else {
            self.game_mode_switch.set_state(false);
            self.game_mode_switch.set_active(false);
            self.game_mode_switch.set_sensitive(false);
        }
    }

    pub fn set_eq_sensitive(&self, sensitive: bool) {
        self.eq_row.set_sensitive(sensitive);
    }

    pub fn set_spatial_sensitive(&self, sensitive: bool) {
        self.spatial_row.set_sensitive(sensitive);
    }

    pub fn set_game_mode_sensitive(&self, sensitive: bool) {
        self.game_mode_switch.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        self.sound_card_section.set_visible(true);
        self.eq_row.set_visible(true);
        self.spatial_row.set_visible(true);
        self.game_mode_section.set_visible(true);

        {
            let _guard = self.eq_flag.enter();
            self.eq_row.clear_items();
            self.eq_row.set_sensitive(false);
        }

        {
            let _guard = self.spatial_flag.enter();
            self.spatial_row.clear_items();
            self.spatial_row.set_sensitive(false);
        }

        {
            let _guard = self.game_mode_flag.enter();
            self.game_mode_switch.set_state(false);
            self.game_mode_switch.set_active(false);
            self.game_mode_switch.set_sensitive(false);
        }
    }

    pub fn connect_eq_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let flag = self.eq_flag.clone();
        self.eq_row.connect_selected_notify(move |row| {
            if flag.is_updating() {
                return;
            }
            if let Some(id) = row.selected_id() {
                callback(id);
            }
        });
    }

    pub fn connect_spatial_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let flag = self.spatial_flag.clone();
        self.spatial_row.connect_selected_notify(move |row| {
            if flag.is_updating() {
                return;
            }
            if let Some(id) = row.selected_id() {
                callback(id);
            }
        });
    }

    pub fn connect_game_mode_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let flag = self.game_mode_flag.clone();
        self.game_mode_switch
            .connect_state_set(move |switch, state| {
                if flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                switch.set_sensitive(false);
                callback(state);
                gtk::glib::Propagation::Stop
            });
    }
}

pub(crate) fn build_quick_sound() -> (GtkBox, QuickSoundWidgets) {
    let sound_card_section = GtkBox::new(Orientation::Vertical, 0);
    sound_card_section.set_hexpand(true);
    sound_card_section.set_halign(Align::Fill);

    let sound_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("sound-quality-and-effects"))
        .build();

    let eq_row = combo_row(
        &crate::t!("equalizer-title"),
        Some(&crate::t!("equalizer-select-preset")),
    );

    let spatial_row = combo_row(
        &crate::t!("spatial-audio-title"),
        Some(&crate::t!("spatial-audio-effects-desc")),
    );

    let (game_mode_section, game_mode_switch) = switch_row(
        &crate::t!("game-mode-title"),
        Some(&crate::t!("game-mode-desc")),
    );

    let sound_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    sound_list.append(&eq_row);
    sound_list.append(&spatial_row);
    sound_list.append(&game_mode_section);

    sound_group.add(&sound_list);
    sound_card_section.append(&sound_group);

    let widgets = QuickSoundWidgets {
        sound_card_section: sound_card_section.clone(),
        eq_row,
        spatial_row,
        game_mode_section,
        game_mode_switch,
        eq_flag: ReentrancyFlag::new(),
        spatial_flag: ReentrancyFlag::new(),
        game_mode_flag: ReentrancyFlag::new(),
    };

    (sound_card_section, widgets)
}

#[cfg(test)]
#[allow(dead_code)]
impl QuickSoundWidgets {
    pub fn set_visible(&self, visible: bool) {
        self.sound_card_section.set_visible(visible);
    }

    pub fn sound_card_section(&self) -> &GtkBox {
        &self.sound_card_section
    }

    pub fn eq_row(&self) -> &adw::ComboRow {
        &self.eq_row
    }

    pub fn spatial_row(&self) -> &adw::ComboRow {
        &self.spatial_row
    }

    pub fn game_mode_switch(&self) -> &Switch {
        &self.game_mode_switch
    }
}
