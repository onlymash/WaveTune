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

use std::{cell::RefCell, rc::Rc, time::Duration};

use adw::prelude::*;

use super::component::SoundComponent;
use crate::{
    app::{
        events::{SettingDomain, UiUpdate, WorkerCommand},
        features::FeatureContext,
    },
    protocol::CustomEqualizer,
};

#[derive(Clone)]
pub(crate) struct SoundController {
    component: SoundComponent,
    context: FeatureContext,
    bass_debounce_source: Rc<RefCell<Option<gtk::glib::SourceId>>>,
}

impl SoundController {
    pub fn new(component: SoundComponent, context: FeatureContext) -> Self {
        Self {
            component,
            context,
            bass_debounce_source: Rc::new(RefCell::new(None)),
        }
    }

    pub fn bind_events(&self) {
        self.bind_equalizer_events();
        self.bind_spatial_events();
        self.bind_game_mode_events();
        self.bind_bass_events();
        self.bind_hi_res_events();
        self.bind_codec_events();
    }

    fn bind_equalizer_events(&self) {
        // 1. Home EQ ComboRow
        let sound_widgets = self.component.sound_widgets.clone();
        let context = self.context.clone();
        let quick = self.component.quick.clone();

        self.component.quick.connect_eq_selected(move |preset_id| {
            quick.set_eq_sensitive(false);

            let is_custom = {
                let st = context.session_state.borrow();
                st.sound.custom_equalizers.iter().any(|c| c.id == preset_id)
            };
            let cmd = if is_custom {
                WorkerCommand::SelectCustomEqualizer(preset_id)
            } else {
                WorkerCommand::SetEqualizer(preset_id)
            };

            match context.send_command(cmd) {
                Ok(()) => {
                    crate::ui::sound_state::update_sound_page_equalizer(
                        &sound_widgets,
                        Some(preset_id),
                    );
                }
                Err(error) => {
                    context.send_status(crate::t!("equalizer-send-failed", error));
                    quick.set_eq_sensitive(true);
                    let cur = context.session_state.borrow().sound.current_eq_preset;
                    quick.update_equalizer(cur);
                }
            }
        });

        // 2. Sound page preset radios
        let quick = self.component.quick.clone();
        let sound_widgets = self.component.sound_widgets.clone();
        let context = self.context.clone();

        self.component
            .sound_widgets
            .eq
            .connect_preset_selected(move |preset_index| {
                let cmd = WorkerCommand::SetEqualizer(preset_index);
                match context.send_command(cmd) {
                    Ok(()) => {
                        quick.update_equalizer(Some(preset_index));
                        quick.set_eq_sensitive(false);
                    }
                    Err(error) => {
                        context.send_status(crate::t!("equalizer-send-failed", error));
                        let cur = context.session_state.borrow().sound.current_eq_preset;
                        sound_widgets.eq.set_selected(cur);
                    }
                }
            });
    }

    fn bind_spatial_events(&self) {
        // 1. Home Spatial ComboRow
        let sound_widgets = self.component.sound_widgets.clone();
        let context = self.context.clone();
        let quick = self.component.quick.clone();

        self.component.quick.connect_spatial_selected(move |mode| {
            quick.set_spatial_sensitive(false);

            let cmd = WorkerCommand::SetSpatialAudio(mode);
            match context.send_command(cmd) {
                Ok(()) => {
                    crate::ui::sound_state::update_sound_page_spatial(&sound_widgets, Some(mode));
                }
                Err(error) => {
                    context.send_setting_error(
                        SettingDomain::SpatialAudio,
                        crate::t!("spatial-audio-send-failed", error),
                    );
                }
            }
        });

        // 2. Sound page spatial radios
        let quick = self.component.quick.clone();
        let sound_widgets = self.component.sound_widgets.clone();
        let context = self.context.clone();

        self.component
            .sound_widgets
            .spatial
            .connect_mode_selected(move |mode| {
                let cmd = WorkerCommand::SetSpatialAudio(mode);
                match context.send_command(cmd) {
                    Ok(()) => {
                        quick.update_spatial_audio(Some(mode));
                        quick.set_spatial_sensitive(false);
                    }
                    Err(error) => {
                        context.send_setting_error(
                            SettingDomain::SpatialAudio,
                            crate::t!("spatial-audio-send-failed", error),
                        );
                        let cur = context.session_state.borrow().sound.current_spatial_mode;
                        sound_widgets.spatial.set_selected(cur);
                    }
                }
            });
    }

    fn bind_game_mode_events(&self) {
        let context = self.context.clone();

        self.component
            .quick
            .connect_game_mode_toggled(move |state| {
                let cmd = WorkerCommand::SetGameMode(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(error) => {
                        context.send_setting_error(
                            SettingDomain::GameMode,
                            crate::t!("game-mode-send-failed", error),
                        );
                    }
                }
            });
    }

    fn bind_bass_events(&self) {
        // 1. Bass switch
        let context = self.context.clone();
        let sound_widgets = self.component.sound_widgets.clone();

        self.component
            .sound_widgets
            .bass
            .connect_switch_toggled(move |state| {
                let cmd = WorkerCommand::SetBassEngineEnabled(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(error) => {
                        context.send_setting_error(
                            SettingDomain::BassEngine,
                            crate::t!("bass-engine-send-failed", error),
                        );
                        let cur = context.session_state.borrow().sound.bass_engine_enabled;
                        if let Some(en) = cur {
                            sound_widgets.bass.set_enabled(en);
                        }
                    }
                }
            });

        // 2. Bass slider (debounced)
        let bass_debounce = self.bass_debounce_source.clone();
        let context = self.context.clone();

        self.component
            .sound_widgets
            .bass
            .connect_slider_changed(move |val| {
                if let Some(source_id) = bass_debounce.borrow_mut().take() {
                    source_id.remove();
                }

                let debounce_clone = bass_debounce.clone();
                let context = context.clone();

                let source_id =
                    gtk::glib::timeout_add_local(Duration::from_millis(500), move || {
                        *debounce_clone.borrow_mut() = None;
                        let cmd = WorkerCommand::SetBassEngineLevel(val);
                        if let Err(error) = context.send_command(cmd) {
                            context.send_status(crate::t!("bass-engine-send-failed", error));
                        }
                        gtk::glib::ControlFlow::Break
                    });
                *bass_debounce.borrow_mut() = Some(source_id);
            });
    }

    fn bind_hi_res_events(&self) {
        let context = self.context.clone();
        let sound_widgets = self.component.sound_widgets.clone();

        self.component
            .sound_widgets
            .enhancement
            .connect_hires_toggled(move |state| {
                let cmd = WorkerCommand::SetHiRes(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(error) => {
                        context.send_status(crate::t!("hi-res-send-failed", error));
                        let cur = context.session_state.borrow().sound.hi_res_enabled;
                        if let Some(enabled) = cur {
                            sound_widgets.enhancement.set_hires_enabled(enabled);
                        }
                    }
                }
            });
    }

    fn bind_codec_events(&self) {
        let context = self.context.clone();
        let sound_widgets = self.component.sound_widgets.clone();

        self.component
            .sound_widgets
            .enhancement
            .connect_codec_selected(move |codec_idx| {
                let is_dual_active = context
                    .session_state
                    .borrow()
                    .dual_device
                    .current_dual_device
                    == Some(true);
                let is_ldac =
                    codec_idx == crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8();

                if is_ldac && is_dual_active {
                    let context_clone = context.clone();
                    let current_codec = context.session_state.borrow().sound.current_audio_codec;
                    let sound_widgets_clone = sound_widgets.clone();

                    let dialog = adw::AlertDialog::builder()
                        .heading(&crate::t!("dialog-reboot-required-title"))
                        .body(&crate::t!("dialog-ldac-conflict-message"))
                        .build();
                    dialog.add_response("cancel", &crate::t!("common-cancel"));
                    dialog.add_response("confirm", &crate::t!("common-confirm"));
                    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

                    dialog.choose(
                        None::<&gtk::Widget>,
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "confirm" {
                                let _ = context_clone
                                    .send_command(WorkerCommand::SetAudioCodec(codec_idx));
                            } else {
                                sound_widgets_clone
                                    .enhancement
                                    .set_selected_codec(current_codec);
                            }
                        },
                    );
                    return;
                }

                sound_widgets.enhancement.set_codec_sensitive(false);
                let cmd = WorkerCommand::SetAudioCodec(codec_idx);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(error) => {
                        context.send_status(crate::t!("audio-codec-send-failed", error));
                        let cur = context.session_state.borrow().sound.current_audio_codec;
                        sound_widgets.enhancement.set_selected_codec(cur);
                    }
                }
            });
    }

    pub fn on_update(&self, update: &UiUpdate) -> bool {
        match update {
            UiUpdate::ProductId { product_id, .. } => {
                let profile = crate::compatibility::profile_for_product_id(*product_id);
                let supports_eq = profile.is_some_and(|p| p.supports_equalizer());
                let supports_custom_eq = profile.is_some_and(|p| p.supports_custom_equalizer());
                let supports_spatial = profile.is_some_and(|p| p.supports_spatial_audio());
                let supports_game = profile.is_some_and(|p| p.supports_game_mode());
                let supports_head_track = profile.is_some_and(|p| p.supports_head_tracking());
                let supports_bass = profile.is_some_and(|p| p.supports_bass_engine());
                let supports_hires = profile.is_some_and(|p| p.supports_hi_res());
                let supports_codec = profile.is_some_and(|p| p.supports_audio_codec());
                let available_codecs =
                    crate::compatibility::supported_codecs_for_product(*product_id).to_vec();
                let presets =
                    crate::compatibility::equalizer_presets_for_product(*product_id).to_vec();

                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.supports_eq = supports_eq;
                    st.sound.supports_custom_eq = supports_custom_eq;
                    st.sound.supports_spatial = supports_spatial;
                    st.sound.supports_head_tracking = supports_head_track;
                    st.sound.supports_game_mode = supports_game;
                    st.sound.supports_bass_engine = supports_bass;
                    st.sound.supports_hi_res = supports_hires;
                    st.sound.supports_audio_codec = supports_codec;
                    st.sound.available_codecs = available_codecs;
                    st.sound.presets = presets;
                }
                self.component
                    .render_features(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::SoundFeatures {
                supports_equalizer,
                supports_custom_equalizer,
                supports_spatial_audio,
                supports_head_tracking,
                supports_game_mode,
                supports_bass_engine,
                supports_hi_res,
                supports_audio_codec,
                available_codecs,
                presets,
            } => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.supports_eq = *supports_equalizer;
                    st.sound.supports_custom_eq = *supports_custom_equalizer;
                    st.sound.supports_spatial = *supports_spatial_audio;
                    st.sound.supports_head_tracking = *supports_head_tracking;
                    st.sound.supports_game_mode = *supports_game_mode;
                    st.sound.supports_bass_engine = *supports_bass_engine;
                    st.sound.supports_hi_res = *supports_hi_res;
                    st.sound.supports_audio_codec = *supports_audio_codec;
                    st.sound.available_codecs = available_codecs.clone();
                    st.sound.presets = presets.clone();
                }
                self.component
                    .render_features(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::Equalizer(preset) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.current_eq_preset = Some(*preset);
                }
                self.component
                    .render_equalizer(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::CustomEqualizers(entries) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.custom_equalizers = entries.clone();
                    if let Some(selected) = entries.iter().find(|e| e.is_selected) {
                        st.sound.current_eq_preset = Some(selected.id);
                    }
                }
                self.render_custom_equalizers();
                true
            }
            UiUpdate::SpatialAudio(mode) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.current_spatial_mode = Some(*mode);
                }
                self.component
                    .render_spatial(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::GameMode(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.current_game_mode = Some(*enabled);
                }
                self.component
                    .render_game_mode(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::BassEngineEnabled(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.bass_engine_enabled = Some(*enabled);
                }
                self.component
                    .render_bass(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::BassEngineLevel(level) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.bass_engine_level = Some(*level);
                }
                self.component
                    .render_bass(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::HiRes(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.hi_res_enabled = Some(*enabled);
                }
                self.component
                    .render_hi_res(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::AudioCodec(codec) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.sound.current_audio_codec = Some(*codec);
                }
                self.component
                    .render_codec(&self.context.session_state.borrow().sound);
                true
            }
            UiUpdate::SettingError { domain, .. } => match domain {
                SettingDomain::Equalizer => {
                    self.component
                        .render_equalizer(&self.context.session_state.borrow().sound);
                    true
                }
                SettingDomain::SpatialAudio => {
                    self.component
                        .render_spatial(&self.context.session_state.borrow().sound);
                    true
                }
                SettingDomain::GameMode => {
                    self.component
                        .render_game_mode(&self.context.session_state.borrow().sound);
                    true
                }
                SettingDomain::BassEngine => {
                    self.component
                        .render_bass(&self.context.session_state.borrow().sound);
                    true
                }
                SettingDomain::HiRes => {
                    self.component
                        .render_hi_res(&self.context.session_state.borrow().sound);
                    true
                }
                SettingDomain::AudioCodec => {
                    self.component
                        .render_codec(&self.context.session_state.borrow().sound);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn render_custom_equalizers(&self) {
        let context = self.context.clone();
        let quick = self.component.quick.clone();

        let on_select = move |id: u8| {
            let cmd = WorkerCommand::SelectCustomEqualizer(id);
            match context.send_command(cmd) {
                Ok(()) => {
                    quick.update_equalizer(Some(id));
                    quick.set_eq_sensitive(false);
                }
                Err(error) => {
                    context.send_status(crate::t!("equalizer-send-failed", error));
                }
            }
        };

        let context_edit = self.context.clone();
        let on_edit = move |preset: &CustomEqualizer, row_widget: &gtk::Widget| {
            let context = context_edit.clone();
            let id = preset.id;
            crate::ui::dialogs::show_custom_eq_dialog(
                row_widget,
                Some(preset),
                &preset.name,
                move |name, bands| {
                    let cmd = WorkerCommand::SaveCustomEqualizer { id, name, bands };
                    if let Err(err) = context.send_command(cmd) {
                        context.send_status(crate::t!("equalizer-send-failed", err));
                    }
                },
            );
        };

        let context_del = self.context.clone();
        let on_delete = move |id: u8, row_widget: &gtk::Widget| {
            let context = context_del.clone();
            let alert = adw::AlertDialog::builder()
                .heading(&crate::t!("equalizer-custom-delete-confirm"))
                .build();
            alert.add_response("cancel", &crate::t!("common-cancel"));
            alert.add_response("delete", &crate::t!("common-delete"));
            alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            alert.choose(
                Some(row_widget),
                None::<&gtk::gio::Cancellable>,
                move |resp| {
                    if resp == "delete" {
                        let cmd = WorkerCommand::DeleteCustomEqualizer(id);
                        if let Err(err) = context.send_command(cmd) {
                            context.send_status(crate::t!("equalizer-send-failed", err));
                        }
                    }
                },
            );
        };

        self.component.render_custom_equalizers(
            &self.context.session_state.borrow().sound,
            on_select,
            on_edit,
            on_delete,
        );
    }

    pub fn reset(&self) {
        if let Some(source_id) = self.bass_debounce_source.borrow_mut().take() {
            source_id.remove();
        }
        self.component.reset();
    }
}
