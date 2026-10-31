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

use crate::{
    app::state::SoundState,
    protocol::CustomEqualizer,
    ui::{
        SoundWidgets,
        pages::home::sound::{HomeSoundFeaturesViewState, QuickSoundWidgets},
        sound_state::{self, SoundPageFeaturesViewState},
    },
};

#[derive(Clone)]
pub(crate) struct SoundComponent {
    pub quick: QuickSoundWidgets,
    pub sound_widgets: SoundWidgets,
}

impl SoundComponent {
    pub fn new(quick: QuickSoundWidgets, sound_widgets: SoundWidgets) -> Self {
        Self {
            quick,
            sound_widgets,
        }
    }

    pub fn render_features(&self, state: &SoundState) {
        self.quick
            .update_sound_features(&HomeSoundFeaturesViewState {
                supports_equalizer: state.supports_eq,
                supports_spatial_audio: state.supports_spatial,
                supports_head_tracking: state.supports_head_tracking,
                supports_game_mode: state.supports_game_mode,
                presets: &state.presets,
                custom_equalizers: &state.custom_equalizers,
            });
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
                custom_equalizers: &state.custom_equalizers,
            },
        );
    }

    pub fn render_equalizer(&self, state: &SoundState) {
        self.quick.update_equalizer(state.current_eq_preset);
        self.quick.set_eq_sensitive(state.supports_eq);
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
        self.quick.update_equalizer_items(
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
        self.quick.update_spatial_audio(state.current_spatial_mode);
        self.quick.set_spatial_sensitive(state.supports_spatial);
        sound_state::update_sound_page_spatial(&self.sound_widgets, state.current_spatial_mode);
    }

    pub fn render_game_mode(&self, state: &SoundState) {
        self.quick.update_game_mode(state.current_game_mode);
        self.quick.set_game_mode_sensitive(state.supports_game_mode);
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
            .set_switch_sensitive(state.supports_bass_engine);
    }

    pub fn render_hi_res(&self, state: &SoundState) {
        if let Some(enabled) = state.hi_res_enabled {
            sound_state::update_sound_page_hi_res(&self.sound_widgets, enabled);
        }
        self.sound_widgets
            .enhancement
            .set_hires_sensitive(state.supports_hi_res);
    }

    pub fn render_codec(&self, state: &SoundState) {
        if let Some(codec) = state.current_audio_codec {
            sound_state::update_sound_page_audio_codec(&self.sound_widgets, codec);
        }
        self.sound_widgets
            .enhancement
            .set_codec_sensitive(state.supports_audio_codec);
    }

    pub fn reset(&self) {
        self.quick.reset();
        sound_state::reset_sound_page(&self.sound_widgets);
    }
}
