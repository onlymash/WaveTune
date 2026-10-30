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

use std::cell::Cell;

use adw::prelude::*;

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
}

#[derive(Clone, Copy)]
pub(crate) struct SoundPageUpdateFlags<'a> {
    pub updating_eq: &'a Cell<bool>,
    pub updating_spatial: &'a Cell<bool>,
    pub updating_codec: &'a Cell<bool>,
}

pub(crate) fn update_sound_page_features(
    sound_widgets: &SoundWidgets,
    state: &SoundPageFeaturesViewState,
    flags: &SoundPageUpdateFlags,
) {
    sound_widgets.eq_card.set_visible(state.supports_equalizer);
    sound_widgets
        .spatial_card
        .set_visible(state.supports_spatial_audio);
    sound_widgets.enhancement_card.set_visible(
        state.supports_hi_res || state.supports_bass_engine || state.supports_audio_codec,
    );
    sound_widgets
        .hires_switch_row
        .set_visible(state.supports_hi_res);
    sound_widgets
        .bass_switch_row
        .set_visible(state.supports_bass_engine);
    sound_widgets
        .bass_slider_row
        .set_visible(state.supports_bass_engine);

    sound_widgets
        .codec_row
        .set_visible(state.supports_audio_codec);
    sound_widgets
        .codec_row
        .set_sensitive(state.supports_audio_codec);
    if state.supports_audio_codec {
        flags.updating_codec.set(true);
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
        crate::ui::components::ComboRowExtManual::set_items(&sound_widgets.codec_row, &items);
        flags.updating_codec.set(false);
    }

    flags.updating_eq.set(true);
    for (idx, radio, row) in &sound_widgets.preset_radios {
        if let Some(matching) = state.presets.iter().find(|p| p.index == *idx) {
            row.set_visible(true);
            row.set_title(&crate::t!(matching.key));
            radio.set_sensitive(state.supports_equalizer);
        } else {
            row.set_visible(false);
            radio.set_sensitive(false);
        }
    }
    flags.updating_eq.set(false);
    let custom_count = sound_widgets.custom_equalizers.borrow().len();
    let can_add_custom =
        state.supports_equalizer && state.supports_custom_equalizer && custom_count < 3;
    sound_widgets.add_custom_row.set_visible(can_add_custom);
    sound_widgets.add_custom_row.set_sensitive(can_add_custom);
    if !state.supports_custom_equalizer {
        sound_widgets.custom_list.set_visible(false);
        sound_widgets.custom_editor.set_visible(false);
    }

    flags.updating_spatial.set(true);
    for (mode, radio, row) in &sound_widgets.spatial_radios {
        match *mode {
            0 | 1 => {
                row.set_visible(true);
                radio.set_sensitive(state.supports_spatial_audio);
            }
            2 => {
                row.set_visible(state.supports_head_tracking);
                radio.set_sensitive(state.supports_spatial_audio && state.supports_head_tracking);
            }
            _ => {}
        }
    }
    flags.updating_spatial.set(false);

    sound_widgets
        .bass_switch
        .set_sensitive(state.supports_bass_engine);
    sound_widgets
        .bass_switch_row
        .set_sensitive(state.supports_bass_engine);

    sound_widgets
        .hires_switch
        .set_sensitive(state.supports_hi_res);
    sound_widgets
        .hires_switch_row
        .set_sensitive(state.supports_hi_res);
}

pub(crate) fn update_sound_page_equalizer(
    sound_widgets: &SoundWidgets,
    selected_preset: Option<u8>,
    updating_eq: &Cell<bool>,
) {
    updating_eq.set(true);
    if let Some(selected) = selected_preset {
        for (idx, radio, _) in &sound_widgets.preset_radios {
            if *idx == selected {
                radio.set_active(true);
            }
        }
        for (idx, radio, _) in sound_widgets.custom_preset_rows.borrow().iter() {
            if *idx == selected {
                radio.set_active(true);
            }
        }
    }
    updating_eq.set(false);
}

pub(crate) fn update_sound_page_custom_equalizers<FSelect, FEdit, FDelete>(
    sound_widgets: &SoundWidgets,
    custom_eqs: &[crate::protocol::CustomEqualizer],
    supports_custom_equalizer: bool,
    updating_eq: &std::rc::Rc<Cell<bool>>,
    on_select: FSelect,
    on_edit: FEdit,
    on_delete: FDelete,
) where
    FSelect: Fn(u8) + Clone + 'static,
    FEdit: Fn(&crate::protocol::CustomEqualizer, &gtk::Widget) + Clone + 'static,
    FDelete: Fn(u8, &gtk::Widget) + Clone + 'static,
{
    // 1. Remove old custom preset rows from preset_list
    {
        let mut old_rows = sound_widgets.custom_preset_rows.borrow_mut();
        for (_, _, row) in old_rows.drain(..) {
            sound_widgets.preset_list.remove(&row);
        }
    }

    sound_widgets.custom_equalizers.replace(custom_eqs.to_vec());

    // 2. Insert new custom preset rows before add_custom_row
    let insert_pos = sound_widgets.add_custom_row.index();
    let mut new_rows = Vec::new();

    updating_eq.set(true);
    for (i, entry) in custom_eqs.iter().enumerate() {
        let (row, radio) =
            crate::ui::components::radio_row(&entry.name, Some(&sound_widgets.first_radio));
        radio.set_sensitive(supports_custom_equalizer);

        // Menu button with popover
        let popover = gtk::Popover::new();
        let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        menu_box.set_margin_top(4);
        menu_box.set_margin_bottom(4);
        menu_box.set_margin_start(4);
        menu_box.set_margin_end(4);

        let edit_btn = gtk::Button::builder()
            .label(&crate::t!("equalizer-custom-modify"))
            .has_frame(false)
            .build();
        edit_btn.set_halign(gtk::Align::Fill);

        let delete_btn = gtk::Button::builder()
            .label(&crate::t!("equalizer-custom-delete"))
            .has_frame(false)
            .build();
        delete_btn.add_css_class("destructive-action");
        delete_btn.set_halign(gtk::Align::Fill);

        menu_box.append(&edit_btn);
        menu_box.append(&delete_btn);
        popover.set_child(Some(&menu_box));

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .popover(&popover)
            .valign(gtk::Align::Center)
            .has_frame(false)
            .tooltip_text(&crate::t!("common-options", "EQ"))
            .build();

        row.add_suffix(&menu_btn);

        // Wiring signals
        {
            let on_select = on_select.clone();
            let id = entry.id;
            let updating_eq = std::rc::Rc::clone(updating_eq);
            radio.connect_toggled(move |r| {
                if r.is_active() && !updating_eq.get() {
                    on_select(id);
                }
            });
        }

        {
            let on_edit = on_edit.clone();
            let entry_clone = entry.clone();
            let popover_weak = popover.downgrade();
            let row_widget = row.clone();
            edit_btn.connect_clicked(move |_| {
                if let Some(pop) = popover_weak.upgrade() {
                    pop.popdown();
                }
                on_edit(&entry_clone, row_widget.upcast_ref());
            });
        }

        {
            let on_delete = on_delete.clone();
            let id = entry.id;
            let popover_weak = popover.downgrade();
            let row_widget = row.clone();
            delete_btn.connect_clicked(move |_| {
                if let Some(pop) = popover_weak.upgrade() {
                    pop.popdown();
                }
                on_delete(id, row_widget.upcast_ref());
            });
        }

        if entry.is_selected {
            radio.set_active(true);
        }

        let pos = if insert_pos >= 0 {
            insert_pos + i as i32
        } else {
            -1
        };
        sound_widgets.preset_list.insert(&row, pos);
        new_rows.push((entry.id, radio, row));
    }
    updating_eq.set(false);

    sound_widgets.custom_preset_rows.replace(new_rows);

    // 3. Hide add_custom_row if max presets reached
    let max_reached = custom_eqs.len() >= 3;
    let can_add = supports_custom_equalizer && !max_reached;
    sound_widgets.add_custom_row.set_visible(can_add);
    sound_widgets.add_custom_row.set_sensitive(can_add);
}

pub(crate) fn update_sound_page_spatial(
    sound_widgets: &SoundWidgets,
    current_spatial: Option<u8>,
    updating_spatial: &Cell<bool>,
) {
    updating_spatial.set(true);
    if let Some(selected) = current_spatial {
        for (mode, radio, _) in &sound_widgets.spatial_radios {
            if *mode == selected {
                radio.set_active(true);
            }
        }
    }
    updating_spatial.set(false);
}

pub(crate) fn update_sound_page_bass_enabled(
    sound_widgets: &SoundWidgets,
    enabled: bool,
    updating_bass_switch: &Cell<bool>,
) {
    updating_bass_switch.set(true);
    sound_widgets.bass_switch.set_state(enabled);
    sound_widgets.bass_switch.set_active(enabled);
    sound_widgets.bass_slider.set_sensitive(enabled);
    updating_bass_switch.set(false);
}

pub(crate) fn format_bass_level(val: i8) -> String {
    if val > 0 {
        format!("+{val}")
    } else {
        format!("{val}")
    }
}

pub(crate) fn update_sound_page_bass_level(
    sound_widgets: &SoundWidgets,
    level: BassEngineLevel,
    updating_bass_slider: &Cell<bool>,
) {
    updating_bass_slider.set(true);
    let min = level.min as f64;
    let max = level.max as f64;
    sound_widgets.bass_slider.set_range(min, max);
    sound_widgets.bass_slider.set_increments(1.0, 1.0);
    sound_widgets.bass_slider.set_round_digits(0);
    sound_widgets.bass_slider.clear_marks();
    let mut v = min;
    while v <= max + 0.001 {
        sound_widgets
            .bass_slider
            .add_mark(v, gtk::PositionType::Bottom, None);
        v += 1.0;
    }
    sound_widgets.bass_slider.set_value(level.current as f64);
    updating_bass_slider.set(false);
}

pub(crate) fn update_sound_page_hi_res(
    sound_widgets: &SoundWidgets,
    hi_res_enabled: bool,
    updating_hires_switch: &Cell<bool>,
) {
    updating_hires_switch.set(true);
    sound_widgets.hires_switch.set_state(hi_res_enabled);
    sound_widgets.hires_switch.set_active(hi_res_enabled);
    updating_hires_switch.set(false);
}

pub(crate) fn update_sound_page_audio_codec(
    sound_widgets: &SoundWidgets,
    codec: u8,
    updating_flag: &Cell<bool>,
) {
    updating_flag.set(true);
    crate::ui::components::ComboRowExtManual::set_selected_id(
        &sound_widgets.codec_row,
        Some(codec),
    );
    updating_flag.set(false);
}

pub(crate) fn reset_sound_page(
    sound_widgets: &SoundWidgets,
    updating_eq: &Cell<bool>,
    updating_spatial: &Cell<bool>,
    updating_bass_switch: &Cell<bool>,
    updating_bass_slider: &Cell<bool>,
    updating_hires_switch: &Cell<bool>,
    updating_codec: &Cell<bool>,
) {
    sound_widgets.eq_card.set_visible(false);
    sound_widgets.spatial_card.set_visible(false);
    sound_widgets.enhancement_card.set_visible(false);

    updating_codec.set(true);
    sound_widgets.codec_row.set_sensitive(false);
    sound_widgets.codec_row.set_visible(false);
    crate::ui::components::ComboRowExtManual::set_selected_id(&sound_widgets.codec_row, None);
    updating_codec.set(false);

    updating_eq.set(true);
    for (_, radio, _) in &sound_widgets.preset_radios {
        radio.set_sensitive(false);
    }
    updating_eq.set(false);
    {
        let mut old_rows = sound_widgets.custom_preset_rows.borrow_mut();
        for (_, _, row) in old_rows.drain(..) {
            sound_widgets.preset_list.remove(&row);
        }
    }
    sound_widgets.custom_equalizers.borrow_mut().clear();
    sound_widgets.add_custom_row.set_visible(false);
    sound_widgets.add_custom_row.set_sensitive(false);
    sound_widgets.custom_list.set_visible(false);
    sound_widgets.custom_editor.set_visible(false);

    updating_spatial.set(true);
    for (_, radio, _) in &sound_widgets.spatial_radios {
        radio.set_sensitive(false);
    }
    updating_spatial.set(false);

    updating_bass_switch.set(true);
    sound_widgets.bass_switch.set_sensitive(false);
    sound_widgets.bass_switch.set_state(false);
    sound_widgets.bass_switch.set_active(false);
    updating_bass_switch.set(false);

    updating_bass_slider.set(true);
    sound_widgets.bass_slider.set_sensitive(false);
    sound_widgets.bass_slider.set_value(0.0);
    updating_bass_slider.set(false);

    updating_hires_switch.set(true);
    sound_widgets.hires_switch.set_sensitive(false);
    sound_widgets.hires_switch.set_state(false);
    sound_widgets.hires_switch.set_active(false);
    updating_hires_switch.set(false);
}
