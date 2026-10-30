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

use crate::{
    app::state::SoundState,
    protocol::CustomEqualizer,
    ui::{
        HomeWidgetRefs, SoundWidgets,
        home_state::{
            HomeSoundFeaturesViewState, reset_sound_controls, update_equalizer,
            update_equalizer_items, update_game_mode, update_sound_features, update_spatial_audio,
        },
        sound_state::{self, SoundPageFeaturesViewState},
    },
};

#[derive(Clone)]
pub(crate) struct SoundComponent {
    pub home_widgets: HomeWidgetRefs,
    pub sound_widgets: SoundWidgets,
}

impl SoundComponent {
    pub fn new(home_widgets: HomeWidgetRefs, sound_widgets: SoundWidgets) -> Self {
        Self {
            home_widgets,
            sound_widgets,
        }
    }

    pub fn render_features(&self, state: &SoundState) {
        update_sound_features(
            &self.home_widgets,
            &HomeSoundFeaturesViewState {
                supports_equalizer: state.supports_eq,
                supports_spatial_audio: state.supports_spatial,
                supports_head_tracking: state.supports_head_tracking,
                supports_game_mode: state.supports_game_mode,
                presets: &state.presets,
                custom_equalizers: &state.custom_equalizers,
            },
        );
        sound_state::update_sound_page_features(
            &self.sound_widgets,
            &SoundPageFeaturesViewState {
                supports_equalizer: state.supports_eq,
                supports_custom_equalizer: state.supports_custom_eq,
                supports_spatial_audio: state.supports_spatial,
                supports_head_tracking: state.supports_head_tracking,
                supports_bass_engine: state.supports_bass_engine,
                supports_hi_res: state.supports_hi_res,
                supports_audio_codec: state.supports_audio_codec,
                available_codecs: &state.available_codecs,
                presets: &state.presets,
            },
        );
    }

    pub fn render_equalizer(&self, state: &SoundState) {
        update_equalizer(&self.home_widgets, state.current_eq_preset);
        self.home_widgets.eq_row.set_sensitive(state.supports_eq);
        sound_state::update_sound_page_equalizer(&self.sound_widgets, state.current_eq_preset);
    }

    pub fn render_custom_equalizers<FSelect, FEdit, FDelete>(
        &self,
        state: &SoundState,
        on_select: FSelect,
        on_edit: FEdit,
        on_delete: FDelete,
    ) where
        FSelect: Fn(u8) + Clone + 'static,
        FEdit: Fn(&CustomEqualizer, &gtk::Widget) + Clone + 'static,
        FDelete: Fn(u8, &gtk::Widget) + Clone + 'static,
    {
        update_equalizer_items(
            &self.home_widgets,
            state.supports_eq,
            &state.presets,
            &state.custom_equalizers,
            state.current_eq_preset,
        );
        sound_state::update_sound_page_custom_equalizers(
            &self.sound_widgets,
            &state.custom_equalizers,
            state.supports_custom_eq,
            on_select,
            on_edit,
            on_delete,
        );
    }

    pub fn render_spatial(&self, state: &SoundState) {
        update_spatial_audio(&self.home_widgets, state.current_spatial_mode);
        self.home_widgets
            .spatial_row
            .set_sensitive(state.supports_spatial);
        sound_state::update_sound_page_spatial(&self.sound_widgets, state.current_spatial_mode);
    }

    pub fn render_game_mode(&self, state: &SoundState) {
        update_game_mode(&self.home_widgets, state.current_game_mode);
        self.home_widgets
            .game_mode_switch
            .set_sensitive(state.supports_game_mode);
    }

    pub fn render_bass(&self, state: &SoundState) {
        if let Some(enabled) = state.bass_engine_enabled {
            sound_state::update_sound_page_bass_enabled(&self.sound_widgets, enabled);
        }
        if let Some(level) = state.bass_engine_level {
            sound_state::update_sound_page_bass_level(&self.sound_widgets, level);
        }
        self.sound_widgets
            .bass
            .bass_switch
            .set_sensitive(state.supports_bass_engine);
        self.sound_widgets
            .bass
            .bass_switch_row
            .set_sensitive(state.supports_bass_engine);
    }

    pub fn render_hi_res(&self, state: &SoundState) {
        if let Some(enabled) = state.hi_res_enabled {
            sound_state::update_sound_page_hi_res(&self.sound_widgets, enabled);
        }
        self.sound_widgets
            .enhancement
            .hires_switch
            .set_sensitive(state.supports_hi_res);
        self.sound_widgets
            .enhancement
            .hires_switch_row
            .set_sensitive(state.supports_hi_res);
    }

    pub fn render_codec(&self, state: &SoundState) {
        if let Some(codec) = state.current_audio_codec {
            sound_state::update_sound_page_audio_codec(&self.sound_widgets, codec);
        }
        self.sound_widgets
            .enhancement
            .codec_row
            .set_sensitive(state.supports_audio_codec);
    }

    pub fn reset(&self) {
        reset_sound_controls(&self.home_widgets);
        sound_state::reset_sound_page(&self.sound_widgets);
    }
}
