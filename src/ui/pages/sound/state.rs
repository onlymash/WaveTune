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

use super::SoundWidgets;
use crate::{compatibility::EqualizerPreset, protocol::BassEngineLevel};

#[derive(Clone, Copy)]
pub(crate) struct SoundPageFeaturesViewState<'a> {
    pub supports_equalizer: bool,
    pub supports_custom_equalizer: bool,
    pub supports_spatial_audio: bool,
    pub supports_head_tracking: bool,
    pub supports_bass_engine: bool,
    pub supports_hi_res: bool,
    pub supports_audio_codec: bool,
    pub available_codecs: &'a [u8],
    pub presets: &'a [EqualizerPreset],
    pub custom_equalizers: &'a [crate::protocol::CustomEqualizer],
}

pub(crate) fn update_sound_page_features(
    sound_widgets: &SoundWidgets,
    state: &SoundPageFeaturesViewState,
) {
    sound_widgets
        .eq
        .set_supported(state.supports_equalizer, state.presets);
    sound_widgets.custom_eq.update_features(
        state.supports_equalizer && state.supports_custom_equalizer,
        state.custom_equalizers.len(),
    );
    sound_widgets
        .spatial
        .set_supported(state.supports_spatial_audio, state.supports_head_tracking);

    let supports_enhancement =
        state.supports_hi_res || state.supports_bass_engine || state.supports_audio_codec;
    sound_widgets
        .enhancement
        .set_card_visible(supports_enhancement);
    sound_widgets
        .enhancement
        .set_hires_supported(state.supports_hi_res);
    sound_widgets
        .enhancement
        .set_codec_supported(state.supports_audio_codec);

    if state.supports_audio_codec {
        let items: Vec<(u8, String)> = state
            .available_codecs
            .iter()
            .map(|&c| {
                let name = match crate::protocol::brands::earfun::EarFunAudioCodec::from_u8(c) {
                    Some(codec) => crate::t!(codec.key()),
                    None => format!("Codec {c}"),
                };
                (c, name)
            })
            .collect();
        sound_widgets.enhancement.set_available_codecs(&items);
    }

    sound_widgets.bass.set_supported(state.supports_bass_engine);
}

pub(crate) fn update_sound_page_equalizer(
    sound_widgets: &SoundWidgets,
    selected_preset: Option<u8>,
) {
    sound_widgets.eq.set_selected(selected_preset);
    sound_widgets
        .custom_eq
        .set_selected(sound_widgets.eq.flag(), selected_preset);
}

pub(crate) fn update_sound_page_custom_equalizers<FSelect, FEdit, FDelete>(
    sound_widgets: &SoundWidgets,
    custom_eqs: &[crate::protocol::CustomEqualizer],
    supports_custom_equalizer: bool,
    on_select: FSelect,
    on_edit: FEdit,
    on_delete: FDelete,
) where
    FSelect: Fn(u8) + Clone + 'static,
    FEdit: Fn(&crate::protocol::CustomEqualizer, &gtk::Widget) + Clone + 'static,
    FDelete: Fn(u8, &gtk::Widget) + Clone + 'static,
{
    sound_widgets.custom_eq.update_presets(
        sound_widgets.eq.preset_list(),
        sound_widgets.eq.first_radio(),
        sound_widgets.eq.flag(),
        custom_eqs,
        supports_custom_equalizer,
        on_select,
        on_edit,
        on_delete,
    );
}

pub(crate) fn update_sound_page_spatial(sound_widgets: &SoundWidgets, current_spatial: Option<u8>) {
    sound_widgets.spatial.set_selected(current_spatial);
}

pub(crate) fn update_sound_page_bass_enabled(sound_widgets: &SoundWidgets, enabled: bool) {
    sound_widgets.bass.set_enabled(enabled);
}

pub(crate) fn format_bass_level(val: i8) -> String {
    if val > 0 {
        format!("+{val}")
    } else {
        format!("{val}")
    }
}

pub(crate) fn update_sound_page_bass_level(sound_widgets: &SoundWidgets, level: BassEngineLevel) {
    sound_widgets.bass.set_level(level);
}

pub(crate) fn update_sound_page_hi_res(sound_widgets: &SoundWidgets, hi_res_enabled: bool) {
    sound_widgets.enhancement.set_hires_enabled(hi_res_enabled);
}

pub(crate) fn update_sound_page_audio_codec(sound_widgets: &SoundWidgets, codec: u8) {
    sound_widgets.enhancement.set_selected_codec(Some(codec));
}

pub(crate) fn reset_sound_page(sound_widgets: &SoundWidgets) {
    sound_widgets.eq.reset();
    sound_widgets
        .custom_eq
        .reset(sound_widgets.eq.preset_list());
    sound_widgets.spatial.reset();
    sound_widgets.enhancement.reset();
    sound_widgets.bass.reset();
}
