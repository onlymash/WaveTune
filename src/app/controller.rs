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

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::mpsc,
    time::Duration,
};

use adw::prelude::*;
use gtk::{Button, Label};

use crate::{
    app::events::{SettingDomain, UiUpdate, WorkerCommand},
    bluetooth,
    compatibility::{EqualizerPreset, GestureRow},
    protocol::{
        BatteryState, CustomEqualizer, EarSide, KeyFunction, NoiseCycleState, PairedDevice,
        PairedDeviceAddress,
    },
    ui::home_state::{
        HomeSoundFeaturesViewState, NoiseControlsViewState, merge_battery, merge_wear,
        reset_sound_controls, set_battery, update_equalizer, update_equalizer_items,
        update_game_mode, update_noise_controls, update_sound_features, update_spatial_audio,
        wear_part_name, widgets_ref,
    },
    ui::{
        ConnectionStateWidgets, ControlsWidgets, DeviceWidgets, HomeWidgets, SoundWidgets,
        components::ComboRowExtManual,
        controls_state::{self, ControlsFeaturesViewState},
        dialogs::{paired_device_icon_name, show_paired_device_dialog},
        sound_state::{self, SoundPageFeaturesViewState},
    },
};

use super::{
    dbus_service::{
        DesktopSnapshotInput, QuickSettingsService, build_snapshot, is_non_off_writable_mode,
        resolve_toggle_target,
    },
    device_manager::{start_device_discovery, start_monitor_for_device},
    state::HeadsetSessionState,
    tray_service::TrayService,
};

fn sync_desktop_services(
    quick_settings: &Option<Rc<QuickSettingsService>>,
    tray: &Option<Rc<TrayService>>,
    state: &HeadsetSessionState,
) {
    if quick_settings.is_none() && tray.is_none() {
        return;
    }
    let snap = build_snapshot(&DesktopSnapshotInput {
        connected: state.connected,
        device_name: &state.device_name,
        noise_controls_enabled: state.noise.enabled,
        supports_wear_detection: state.supports_wear_detection,
        wear: &state.wear,
        battery: &state.battery,
        case_battery_stale: state.case_battery_is_stale,
        current_noise_mode: state.noise.current_mode,
        supported_indices: &state.noise.supported_indices,
        modes: &state.noise.modes,
    });
    if let Some(service) = quick_settings {
        service.update(snap.clone());
    }
    if let Some(tray) = tray {
        tray.update(snap);
    }
}

pub(crate) struct AppUiWidgets {
    pub home: HomeWidgets,
    pub sound: SoundWidgets,
    pub controls: ControlsWidgets,
    pub device: DeviceWidgets,
    pub connection_state: ConnectionStateWidgets,
}

pub(crate) struct DesktopServices {
    pub quick_settings: Option<Rc<QuickSettingsService>>,
    pub tray: Option<Rc<TrayService>>,
}

pub(crate) struct AppActions {
    pub present_window: crate::ui::PresentWindowFn,
    pub quit_app: crate::ui::QuitAppFn,
}

pub(crate) struct AppControllerContext {
    pub widgets: AppUiWidgets,
    pub services: DesktopServices,
    pub actions: AppActions,
}

pub(crate) fn wire_bluetooth_controls(ctx: AppControllerContext) {
    let AppControllerContext {
        widgets:
            AppUiWidgets {
                home: widgets,
                sound: sound_widgets,
                controls: controls_widgets,
                device: device_widgets,
                connection_state,
            },
        services: DesktopServices {
            quick_settings,
            tray,
        },
        actions: AppActions {
            present_window,
            quit_app,
        },
    } = ctx;
    let (updates_tx, updates_rx) = mpsc::channel();
    let (async_tx, mut async_rx) = tokio::sync::mpsc::unbounded_channel::<UiUpdate>();
    std::thread::Builder::new()
        .name("wavetune-ui-bridge".to_string())
        .spawn(move || {
            while let Ok(msg) = updates_rx.recv() {
                if async_tx.send(msg).is_err() {
                    break;
                }
            }
        })
        .expect("Failed to spawn UI update bridge thread");
    let session_state = Rc::new(RefCell::new(HeadsetSessionState::new()));
    let worker_commands: Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>> =
        Rc::new(RefCell::new(None));
    let device_choices = Rc::new(RefCell::new(Vec::<bluetooth::DeviceChoice>::new()));
    let connected_device = Rc::new(RefCell::new(None::<bluetooth::DeviceChoice>));
    let pending_connect = Rc::new(RefCell::new(None::<bluetooth::DeviceChoice>));
    let paired_action_pending = Rc::new(Cell::new(false));
    let scanning = Rc::new(Cell::new(false));
    let auto_connect_pending = Rc::new(Cell::new(true));

    {
        let updates_tx = updates_tx.clone();
        let refresh_button = widgets.refresh_devices.clone();
        let scanning = Rc::clone(&scanning);
        widgets.refresh_devices.connect_clicked(move |_| {
            start_device_discovery(
                updates_tx.clone(),
                refresh_button.clone(),
                Rc::clone(&scanning),
                true,
            );
        });
    }

    let primary_buttons: Vec<(u8, Button)> = widgets
        .primary_noise_controls
        .iter()
        .map(|control| (control.index, control.button.clone()))
        .collect();
    for (protocol_index, button) in &primary_buttons {
        let protocol_index = *protocol_index;
        let button = button.clone();
        let noise_buttons = primary_buttons.clone();
        let noise_row = widgets.noise_option_row.clone();
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        button.connect_clicked(move |_| {
            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(WorkerCommand::SetNoiseMode(protocol_index)));
            match command_result {
                Some(Ok(())) => {
                    for (_, button) in &noise_buttons {
                        button.set_sensitive(false);
                    }
                    noise_row.set_sensitive(false);
                }
                Some(Err(error)) => {
                    let _ = updates_tx
                        .send(UiUpdate::Status(crate::t!("noise-mode-send-failed", error)));
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                }
            }
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let noise_flag = widgets.noise_option_flag.clone();
        let noise_buttons = primary_buttons.clone();
        widgets
            .noise_option_row
            .connect_selected_notify(move |row| {
                if noise_flag.is_updating() {
                    return;
                }
                let Some(mode) = row.selected_id() else {
                    return;
                };
                row.set_sensitive(false);
                for (_, button) in &noise_buttons {
                    button.set_sensitive(false);
                }
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|sender| sender.send(WorkerCommand::SetNoiseMode(mode)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(error)) => {
                        let _ = updates_tx.send(UiUpdate::SettingError {
                            domain: SettingDomain::NoiseMode,
                            message: crate::t!("noise-mode-send-failed", error),
                        });
                    }
                    None => {
                        let _ = updates_tx.send(UiUpdate::SettingError {
                            domain: SettingDomain::NoiseMode,
                            message: crate::t!("channel-not-connected"),
                        });
                    }
                }
            });
    }

    let bass_debounce_source = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));

    let current_eq_preset = Rc::new(Cell::new(None::<u8>));
    let current_spatial_mode = Rc::new(Cell::new(None::<u8>));
    let current_game_mode = Rc::new(Cell::new(None::<bool>));
    let current_eq_presets = Rc::new(RefCell::new(Vec::<EqualizerPreset>::new()));
    let current_custom_eqs = Rc::new(RefCell::new(Vec::<CustomEqualizer>::new()));
    let current_supports_eq = Rc::new(Cell::new(false));
    let current_supports_custom_eq = Rc::new(Cell::new(false));
    let current_supports_spatial = Rc::new(Cell::new(false));
    let current_supports_game_mode = Rc::new(Cell::new(false));
    let current_supports_bass_engine = Rc::new(Cell::new(false));
    let current_supports_hi_res = Rc::new(Cell::new(false));
    let current_supports_audio_codec = Rc::new(Cell::new(false));
    let current_available_codecs = Rc::new(RefCell::new(Vec::<u8>::new()));
    let current_audio_codec = Rc::new(Cell::new(None::<u8>));
    let current_bass_engine_enabled = Rc::new(Cell::new(None::<bool>));
    let current_bass_engine_level = Rc::new(RefCell::new(None::<crate::protocol::BassEngineLevel>));
    let current_hi_res_enabled = Rc::new(Cell::new(None::<bool>));
    let current_dual_device = Rc::new(Cell::new(None::<bool>));

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let eq_flag = widgets.eq_flag.clone();
        let sound_widgets = sound_widgets.clone();
        let current_custom_eqs = Rc::clone(&current_custom_eqs);
        let current_eq_preset = Rc::clone(&current_eq_preset);

        widgets.eq_row.connect_selected_notify(move |row| {
            if eq_flag.is_updating() {
                return;
            }
            let Some(preset_id) = row.selected_id() else {
                return;
            };
            row.set_sensitive(false);

            let is_custom = current_custom_eqs
                .borrow()
                .iter()
                .any(|c| c.id == preset_id);

            let cmd = if is_custom {
                WorkerCommand::SelectCustomEqualizer(preset_id)
            } else {
                WorkerCommand::SetEqualizer(preset_id)
            };

            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(cmd));
            match command_result {
                Some(Ok(())) => {
                    sound_state::update_sound_page_equalizer(&sound_widgets, Some(preset_id));
                }
                Some(Err(error)) => {
                    let _ = updates_tx
                        .send(UiUpdate::Status(crate::t!("equalizer-send-failed", error)));
                    row.set_sensitive(true);
                    let _guard = eq_flag.enter();
                    row.set_selected_id(current_eq_preset.get());
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    row.set_sensitive(true);
                    let _guard = eq_flag.enter();
                    row.set_selected_id(current_eq_preset.get());
                }
            }
        });
    }

    for (preset_index, radio, _) in &sound_widgets.preset_radios {
        let preset_index = *preset_index;
        let radio = radio.clone();
        let sound_eq_flag = sound_widgets.eq_flag.clone();
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let widgets_eq_row = widgets.eq_row.clone();
        let widgets_eq_flag = widgets.eq_flag.clone();
        let current_eq_preset = Rc::clone(&current_eq_preset);
        let sound_widgets_for_revert = sound_widgets.clone();

        radio.connect_toggled(move |r| {
            if !r.is_active() || sound_eq_flag.is_updating() {
                return;
            }
            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(WorkerCommand::SetEqualizer(preset_index)));
            match command_result {
                Some(Ok(())) => {
                    let _guard = widgets_eq_flag.enter();
                    widgets_eq_row.set_selected_id(Some(preset_index));
                    widgets_eq_row.set_sensitive(false);
                }
                Some(Err(error)) => {
                    let _ = updates_tx
                        .send(UiUpdate::Status(crate::t!("equalizer-send-failed", error)));
                    sound_state::update_sound_page_equalizer(
                        &sound_widgets_for_revert,
                        current_eq_preset.get(),
                    );
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    sound_state::update_sound_page_equalizer(
                        &sound_widgets_for_revert,
                        current_eq_preset.get(),
                    );
                }
            }
        });
    }

    {
        let sound_widgets = sound_widgets.clone();
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let current_supports_custom_eq = Rc::clone(&current_supports_custom_eq);

        sound_widgets.add_custom_row.connect_activated(move |row| {
            if !current_supports_custom_eq.get() {
                return;
            }
            let existing = sound_widgets.custom_equalizers.borrow();
            if existing.len() >= 3 {
                return;
            }
            let mut max_num = 0;
            let localized_prefix = crate::t!("equalizer-custom-default-prefix");
            for p in existing.iter() {
                if let Some(rest) = p
                    .name
                    .strip_prefix(localized_prefix.as_str())
                    .or_else(|| p.name.strip_prefix("自定义"))
                    .or_else(|| p.name.strip_prefix("Custom"))
                {
                    if let Ok(n) = rest.trim().parse::<usize>() {
                        if n > max_num {
                            max_num = n;
                        }
                    }
                }
            }
            let default_name = crate::t!("equalizer-custom-default-name", max_num + 1);
            drop(existing);

            let worker_commands = Rc::clone(&worker_commands);
            let updates_tx = updates_tx.clone();
            crate::ui::dialogs::show_custom_eq_dialog(
                row,
                None,
                &default_name,
                move |name, bands| {
                    let cmd = WorkerCommand::AddCustomEqualizer { name, bands };
                    if let Some(sender) = worker_commands.borrow().as_ref() {
                        let _ = sender.send(cmd);
                    } else {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    }
                },
            );
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let spatial_flag = widgets.spatial_flag.clone();
        let sound_widgets = sound_widgets.clone();

        widgets.spatial_row.connect_selected_notify(move |row| {
            if spatial_flag.is_updating() {
                return;
            }
            let Some(mode) = row.selected_id() else {
                return;
            };

            row.set_sensitive(false);
            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(WorkerCommand::SetSpatialAudio(mode)));
            match command_result {
                Some(Ok(())) => {
                    sound_state::update_sound_page_spatial(&sound_widgets, Some(mode));
                }
                Some(Err(error)) => {
                    let _ = updates_tx.send(UiUpdate::SettingError {
                        domain: SettingDomain::SpatialAudio,
                        message: crate::t!("spatial-audio-send-failed", error),
                    });
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::SettingError {
                        domain: SettingDomain::SpatialAudio,
                        message: crate::t!("channel-not-connected"),
                    });
                }
            }
        });
    }

    for (mode, radio, _) in &sound_widgets.spatial_radios {
        let mode = *mode;
        let radio = radio.clone();
        let sound_spatial_flag = sound_widgets.spatial_flag.clone();
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let widgets_spatial_row = widgets.spatial_row.clone();
        let widgets_spatial_flag = widgets.spatial_flag.clone();
        let current_spatial_mode = Rc::clone(&current_spatial_mode);
        let sound_widgets_for_revert = sound_widgets.clone();

        radio.connect_toggled(move |r| {
            if !r.is_active() || sound_spatial_flag.is_updating() {
                return;
            }

            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(WorkerCommand::SetSpatialAudio(mode)));
            match command_result {
                Some(Ok(())) => {
                    let _guard = widgets_spatial_flag.enter();
                    widgets_spatial_row.set_selected_id(Some(mode));
                    widgets_spatial_row.set_sensitive(false);
                }
                Some(Err(error)) => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                        "spatial-audio-send-failed",
                        error
                    )));
                    sound_state::update_sound_page_spatial(
                        &sound_widgets_for_revert,
                        current_spatial_mode.get(),
                    );
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    sound_state::update_sound_page_spatial(
                        &sound_widgets_for_revert,
                        current_spatial_mode.get(),
                    );
                }
            }
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let game_mode_flag = widgets.game_mode_flag.clone();
        widgets
            .game_mode_switch
            .connect_state_set(move |switch, state| {
                if game_mode_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|sender| sender.send(WorkerCommand::SetGameMode(state)));
                match command_result {
                    Some(Ok(())) => {
                        switch.set_sensitive(false);
                    }
                    Some(Err(error)) => {
                        let _ = updates_tx.send(UiUpdate::SettingError {
                            domain: SettingDomain::GameMode,
                            message: crate::t!("game-mode-send-failed", error),
                        });
                    }
                    None => {
                        let _ = updates_tx.send(UiUpdate::SettingError {
                            domain: SettingDomain::GameMode,
                            message: crate::t!("channel-not-connected"),
                        });
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let bass_switch_flag = sound_widgets.bass_switch_flag.clone();
        let sound_widgets_revert = sound_widgets.clone();
        let current_bass_engine_enabled = Rc::clone(&current_bass_engine_enabled);

        sound_widgets
            .bass_switch
            .connect_state_set(move |switch, state| {
                if bass_switch_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|sender| sender.send(WorkerCommand::SetBassEngineEnabled(state)));
                match command_result {
                    Some(Ok(())) => {
                        switch.set_sensitive(false);
                    }
                    Some(Err(error)) => {
                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                            "bass-engine-send-failed",
                            error
                        )));
                        if let Some(enabled) = current_bass_engine_enabled.get() {
                            sound_state::update_sound_page_bass_enabled(
                                &sound_widgets_revert,
                                enabled,
                            );
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(enabled) = current_bass_engine_enabled.get() {
                            sound_state::update_sound_page_bass_enabled(
                                &sound_widgets_revert,
                                enabled,
                            );
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let bass_slider_flag = sound_widgets.bass_slider_flag.clone();
        let bass_debounce_source = Rc::clone(&bass_debounce_source);

        sound_widgets
            .bass_slider
            .connect_value_changed(move |slider| {
                let rounded = slider.value().round();
                if (slider.value() - rounded).abs() > 0.001 {
                    slider.set_value(rounded);
                    return;
                }
                let value = rounded as i8;
                if bass_slider_flag.is_updating() {
                    return;
                }
                if let Some(source_id) = bass_debounce_source.borrow_mut().take() {
                    source_id.remove();
                }

                let worker_commands = Rc::clone(&worker_commands);
                let updates_tx = updates_tx.clone();
                let bass_debounce_source_clone = Rc::clone(&bass_debounce_source);

                let source_id =
                    gtk::glib::timeout_add_local(Duration::from_millis(500), move || {
                        *bass_debounce_source_clone.borrow_mut() = None;
                        let command_result = worker_commands
                            .borrow()
                            .as_ref()
                            .map(|sender| sender.send(WorkerCommand::SetBassEngineLevel(value)));
                        match command_result {
                            Some(Ok(())) => {}
                            Some(Err(error)) => {
                                let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                    "bass-engine-send-failed",
                                    error
                                )));
                            }
                            None => {
                                let _ = updates_tx
                                    .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                            }
                        }
                        gtk::glib::ControlFlow::Break
                    });
                *bass_debounce_source.borrow_mut() = Some(source_id);
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let hires_switch_flag = sound_widgets.hires_switch_flag.clone();
        let sound_widgets_revert = sound_widgets.clone();
        let current_hi_res_enabled = Rc::clone(&current_hi_res_enabled);

        sound_widgets
            .hires_switch
            .connect_state_set(move |switch, state| {
                if hires_switch_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|sender| sender.send(WorkerCommand::SetHiRes(state)));
                match command_result {
                    Some(Ok(())) => {
                        switch.set_sensitive(false);
                    }
                    Some(Err(error)) => {
                        let _ = updates_tx
                            .send(UiUpdate::Status(crate::t!("hi-res-send-failed", error)));
                        if let Some(enabled) = current_hi_res_enabled.get() {
                            sound_state::update_sound_page_hi_res(&sound_widgets_revert, enabled);
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(enabled) = current_hi_res_enabled.get() {
                            sound_state::update_sound_page_hi_res(&sound_widgets_revert, enabled);
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let codec_flag = sound_widgets.codec_flag.clone();
        let current_codec = Rc::clone(&current_audio_codec);
        let current_dual = Rc::clone(&current_dual_device);

        sound_widgets.codec_row.connect_selected_notify(move |row| {
            if codec_flag.is_updating() {
                return;
            }
            let Some(target_codec) = crate::ui::components::ComboRowExtManual::selected_id(row)
            else {
                return;
            };
            if current_codec.get() == Some(target_codec) {
                return;
            }

            let is_ldac =
                target_codec == crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8();
            let dual_is_on = current_dual.get().unwrap_or(false);

            let (title, message) = if is_ldac && dual_is_on {
                (
                    crate::t!("dialog-reboot-required-title"),
                    crate::t!("dialog-ldac-conflict-message"),
                )
            } else {
                (
                    crate::t!("dialog-reboot-required-title"),
                    crate::t!("dialog-codec-reboot-message"),
                )
            };

            let row_for_dialog = row.clone();
            let row_for_action = row.clone();
            let current_codec_confirm = Rc::clone(&current_codec);
            let worker_commands = Rc::clone(&worker_commands);
            let updates_tx = updates_tx.clone();

            let row_revert = row.clone();
            let codec_flag_cancel = codec_flag.clone();
            let current_codec_cancel = Rc::clone(&current_codec);

            let dialog = adw::AlertDialog::builder()
                .heading(&title)
                .body(&message)
                .build();
            dialog.add_response("cancel", &crate::t!("common-cancel"));
            dialog.add_response("confirm", &crate::t!("common-confirm"));
            dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

            dialog.choose(
                Some(&row_for_dialog),
                None::<&gtk::gio::Cancellable>,
                move |response| {
                    if response == "confirm" {
                        row_for_action.set_sensitive(false);
                        current_codec_confirm.set(Some(target_codec));
                        let cmd_res = worker_commands
                            .borrow()
                            .as_ref()
                            .map(|s| s.send(WorkerCommand::SetAudioCodec(target_codec)));
                        if let Some(Err(e)) = cmd_res {
                            let _ = updates_tx.send(UiUpdate::SettingError {
                                domain: SettingDomain::AudioCodec,
                                message: e.to_string(),
                            });
                        }
                    } else {
                        let _guard = codec_flag_cancel.enter();
                        crate::ui::components::ComboRowExtManual::set_selected_id(
                            &row_revert,
                            current_codec_cancel.get(),
                        );
                    }
                },
            );
        });
    }

    let selected_ear = Rc::new(Cell::new(EarSide::Left));
    let current_key_functions = Rc::new(RefCell::new(Vec::<KeyFunction>::new()));
    let current_gesture_rows = Rc::new(RefCell::new(Vec::<GestureRow>::new()));
    let current_noise_cycle = Rc::new(RefCell::new(None::<NoiseCycleState>));
    let is_finding_earbuds = Rc::new(Cell::new(false));
    let fit_test_running = Rc::new(Cell::new(false));
    let current_prompt_volume = Rc::new(Cell::new(None::<u8>));
    let current_head_motion = Rc::new(Cell::new(None::<u8>));
    let current_in_ear = Rc::new(Cell::new(None::<bool>));
    let current_sleep = Rc::new(Cell::new(None::<bool>));
    let current_preferred_device = Rc::new(RefCell::new(None::<PairedDeviceAddress>));
    let cached_paired_devices = Rc::new(RefCell::new(Vec::<PairedDevice>::new()));
    let current_wear = Rc::new(RefCell::new(crate::protocol::WearState::default()));

    let prompt_volume_debounce_source = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));

    {
        let selected_ear = Rc::clone(&selected_ear);
        let controls_widgets_clone = controls_widgets.clone();
        let current_key_functions = Rc::clone(&current_key_functions);
        let current_gesture_rows = Rc::clone(&current_gesture_rows);
        let current_noise_cycle = Rc::clone(&current_noise_cycle);

        controls_widgets.ear_left_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                selected_ear.set(EarSide::Left);
                controls_state::update_key_functions(
                    &controls_widgets_clone,
                    &current_key_functions.borrow(),
                    EarSide::Left,
                    &current_gesture_rows.borrow(),
                );
                if let Some(cycle) = *current_noise_cycle.borrow() {
                    controls_state::update_noise_cycle(
                        &controls_widgets_clone,
                        cycle,
                        EarSide::Left,
                    );
                }
            }
        });
    }

    {
        let selected_ear = Rc::clone(&selected_ear);
        let controls_widgets_clone = controls_widgets.clone();
        let current_key_functions = Rc::clone(&current_key_functions);
        let current_gesture_rows = Rc::clone(&current_gesture_rows);
        let current_noise_cycle = Rc::clone(&current_noise_cycle);

        controls_widgets.ear_right_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                selected_ear.set(EarSide::Right);
                controls_state::update_key_functions(
                    &controls_widgets_clone,
                    &current_key_functions.borrow(),
                    EarSide::Right,
                    &current_gesture_rows.borrow(),
                );
                if let Some(cycle) = *current_noise_cycle.borrow() {
                    controls_state::update_noise_cycle(
                        &controls_widgets_clone,
                        cycle,
                        EarSide::Right,
                    );
                }
            }
        });
    }

    for (row_def, combo) in &controls_widgets.gesture_combos {
        let row_def = row_def.clone();
        let combo = combo.clone();
        let selected_ear = Rc::clone(&selected_ear);
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let gesture_combos_flag = controls_widgets.gesture_combos_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_key_functions = Rc::clone(&current_key_functions);
        let current_gesture_rows = Rc::clone(&current_gesture_rows);

        combo.connect_selected_notify(move |c| {
            if gesture_combos_flag.is_updating() {
                return;
            }
            if let Some(func_id) = c.selected_id() {
                let side = selected_ear.get();
                let button = row_def.button;
                let action = row_def.action;
                let function = func_id;

                c.set_sensitive(false);
                let command_result = worker_commands.borrow().as_ref().map(|s| {
                    s.send(WorkerCommand::SetGesture {
                        side,
                        button,
                        action,
                        function,
                    })
                });
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                            "controls-gesture-send-failed",
                            err
                        )));
                        controls_state::update_key_functions(
                            &controls_widgets_clone,
                            &current_key_functions.borrow(),
                            selected_ear.get(),
                            &current_gesture_rows.borrow(),
                        );
                        c.set_sensitive(true);
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        controls_state::update_key_functions(
                            &controls_widgets_clone,
                            &current_key_functions.borrow(),
                            selected_ear.get(),
                            &current_gesture_rows.borrow(),
                        );
                        c.set_sensitive(true);
                    }
                }
            }
        });
    }

    for check in [
        &controls_widgets.cycle_check_anc,
        &controls_widgets.cycle_check_transparency,
        &controls_widgets.cycle_check_off,
    ] {
        let controls_widgets_clone = controls_widgets.clone();
        let selected_ear = Rc::clone(&selected_ear);
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let noise_cycle_flag = controls_widgets.noise_cycle_flag.clone();
        let current_noise_cycle = Rc::clone(&current_noise_cycle);

        check.connect_toggled(move |_| {
            if noise_cycle_flag.is_updating() {
                return;
            }
            let anc = controls_widgets_clone.cycle_check_anc.is_active();
            let trans = controls_widgets_clone.cycle_check_transparency.is_active();
            let off = controls_widgets_clone.cycle_check_off.is_active();
            let count = (anc as u8) + (trans as u8) + (off as u8);
            if count < 2 {
                let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                    "controls-noise-cycle-min-warning"
                )));
                if let Some(cycle) = *current_noise_cycle.borrow() {
                    controls_state::update_noise_cycle(
                        &controls_widgets_clone,
                        cycle,
                        selected_ear.get(),
                    );
                }
                return;
            }
            let mut mask = 0u8;
            if off {
                mask |= 1 << 0;
            }
            if anc {
                mask |= 1 << 1;
            }
            if trans {
                mask |= 1 << 2;
            }
            let side = selected_ear.get();
            let command_result = worker_commands.borrow().as_ref().map(|s| {
                s.send(WorkerCommand::SetNoiseCycle {
                    side,
                    mask: mask as u32,
                    is_shared: false,
                })
            });
            match command_result {
                Some(Ok(())) => {
                    controls_widgets_clone.cycle_check_anc.set_sensitive(false);
                    controls_widgets_clone
                        .cycle_check_transparency
                        .set_sensitive(false);
                    controls_widgets_clone.cycle_check_off.set_sensitive(false);
                }
                Some(Err(err)) => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                        "controls-noise-cycle-send-failed",
                        err
                    )));
                    if let Some(cycle) = *current_noise_cycle.borrow() {
                        controls_state::update_noise_cycle(
                            &controls_widgets_clone,
                            cycle,
                            selected_ear.get(),
                        );
                    }
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    if let Some(cycle) = *current_noise_cycle.borrow() {
                        controls_state::update_noise_cycle(
                            &controls_widgets_clone,
                            cycle,
                            selected_ear.get(),
                        );
                    }
                }
            }
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let is_finding = Rc::clone(&is_finding_earbuds);
        let current_wear = Rc::clone(&current_wear);

        controls_widgets.find_btn.connect_clicked(move |_| {
            if is_finding.get() {
                controls_widgets_clone.find_btn.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|s| s.send(WorkerCommand::FindEarbuds(false)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                            "controls-find-send-failed",
                            err
                        )));
                        controls_widgets_clone.find_btn.set_sensitive(true);
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        controls_widgets_clone.find_btn.set_sensitive(true);
                    }
                }
            } else {
                let in_ear = current_wear.borrow().any_earbud_in_ear();
                let worker_commands_find = Rc::clone(&worker_commands);
                let updates_tx_find = updates_tx.clone();
                let find_btn_find = controls_widgets_clone.find_btn.clone();
                let do_play = move || {
                    find_btn_find.set_sensitive(false);
                    let command_result = worker_commands_find
                        .borrow()
                        .as_ref()
                        .map(|s| s.send(WorkerCommand::FindEarbuds(true)));
                    match command_result {
                        Some(Ok(())) => {}
                        Some(Err(err)) => {
                            let _ = updates_tx_find.send(UiUpdate::Status(crate::t!(
                                "controls-find-send-failed",
                                err
                            )));
                            find_btn_find.set_sensitive(true);
                        }
                        None => {
                            let _ = updates_tx_find
                                .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                            find_btn_find.set_sensitive(true);
                        }
                    }
                };

                if in_ear {
                    let alert = adw::AlertDialog::builder()
                        .heading(&crate::t!("controls-find-warning-title"))
                        .body(&crate::t!("controls-find-warning-body"))
                        .build();
                    alert.add_response("cancel", &crate::t!("common-cancel"));
                    alert.add_response("play", &crate::t!("controls-find-play"));
                    alert.set_response_appearance("play", adw::ResponseAppearance::Destructive);
                    alert.choose(
                        Some(&controls_widgets_clone.find_btn),
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "play" {
                                do_play();
                            }
                        },
                    );
                } else {
                    do_play();
                }
            }
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let controls_widgets_clone = controls_widgets.clone();
        controls_widgets.fit_btn.connect_clicked(move |_| {
            controls_state::update_fit_test_running(&controls_widgets_clone, true);
            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|s| s.send(WorkerCommand::RunFitTest));
            match command_result {
                Some(Ok(())) => {}
                Some(Err(err)) => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                        "controls-fit-test-send-failed",
                        err
                    )));
                    controls_state::update_fit_test_running(&controls_widgets_clone, false);
                }
                None => {
                    let _ = updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                    controls_state::update_fit_test_running(&controls_widgets_clone, false);
                }
            }
        });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let prompt_volume_flag = controls_widgets.prompt_volume_flag.clone();
        let prompt_volume_debounce_source = Rc::clone(&prompt_volume_debounce_source);
        let controls_widgets_clone = controls_widgets.clone();

        controls_widgets
            .prompt_volume_scale
            .connect_value_changed(move |slider| {
                let rounded = slider.value().round();
                if (slider.value() - rounded).abs() > 0.001 {
                    slider.set_value(rounded);
                    return;
                }
                let value = rounded as u8;
                controls_widgets_clone
                    .prompt_volume_label
                    .set_text(&value.to_string());
                if prompt_volume_flag.is_updating() {
                    return;
                }
                if let Some(source_id) = prompt_volume_debounce_source.borrow_mut().take() {
                    source_id.remove();
                }

                let worker_commands = Rc::clone(&worker_commands);
                let updates_tx = updates_tx.clone();
                let debounce_clone = Rc::clone(&prompt_volume_debounce_source);

                let source_id =
                    gtk::glib::timeout_add_local(Duration::from_millis(500), move || {
                        *debounce_clone.borrow_mut() = None;
                        let command_result = worker_commands
                            .borrow()
                            .as_ref()
                            .map(|s| s.send(WorkerCommand::SetPromptVolume(value)));
                        match command_result {
                            Some(Ok(())) => {}
                            Some(Err(err)) => {
                                let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                    "controls-prompt-volume-send-failed",
                                    err
                                )));
                            }
                            None => {
                                let _ = updates_tx
                                    .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                            }
                        }
                        gtk::glib::ControlFlow::Break
                    });
                *prompt_volume_debounce_source.borrow_mut() = Some(source_id);
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let head_motion_flag = controls_widgets.head_motion_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_head = Rc::clone(&current_head_motion);

        controls_widgets
            .head_switch
            .connect_state_set(move |switch, state| {
                if head_motion_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                let motion_type = if state {
                    let sel = controls_widgets_clone
                        .head_motion_row
                        .selected_id()
                        .unwrap_or(0);
                    if sel == 0 { 1 } else { sel }
                } else {
                    0
                };

                switch.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|s| s.send(WorkerCommand::SetHeadMotionType(motion_type)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                            "controls-head-motion-send-failed",
                            err
                        )));
                        if let Some(m) = current_head.get() {
                            controls_state::update_head_switch(&controls_widgets_clone, m > 0);
                            controls_state::update_head_motion(&controls_widgets_clone, m);
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(m) = current_head.get() {
                            controls_state::update_head_switch(&controls_widgets_clone, m > 0);
                            controls_state::update_head_motion(&controls_widgets_clone, m);
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let head_motion_flag = controls_widgets.head_motion_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_head = Rc::clone(&current_head_motion);

        controls_widgets
            .head_motion_row
            .connect_selected_notify(move |row| {
                if head_motion_flag.is_updating() {
                    return;
                }
                if let Some(motion_type) = row.selected_id() {
                    if controls_widgets_clone.head_switch.is_active() && motion_type > 0 {
                        row.set_sensitive(false);
                        let command_result = worker_commands
                            .borrow()
                            .as_ref()
                            .map(|s| s.send(WorkerCommand::SetHeadMotionType(motion_type)));
                        match command_result {
                            Some(Ok(())) => {}
                            Some(Err(err)) => {
                                let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                    "controls-head-motion-send-failed",
                                    err
                                )));
                                if let Some(m) = current_head.get() {
                                    controls_state::update_head_motion(&controls_widgets_clone, m);
                                }
                            }
                            None => {
                                let _ = updates_tx
                                    .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                                if let Some(m) = current_head.get() {
                                    controls_state::update_head_motion(&controls_widgets_clone, m);
                                }
                            }
                        }
                    }
                }
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let dual_device_flag = controls_widgets.dual_device_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_dual = Rc::clone(&current_dual_device);
        let current_codec = Rc::clone(&current_audio_codec);

        controls_widgets
            .dual_switch
            .connect_state_set(move |switch, state| {
                if dual_device_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }

                // If user enables dual device while LDAC is active on EarFun:
                let is_ldac = current_codec.get()
                    == Some(crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8());
                if state && is_ldac {
                    let switch_for_dialog = switch.clone();
                    let switch_for_action = switch.clone();
                    let updating_dual_confirm = dual_device_flag.clone();
                    let worker_commands = Rc::clone(&worker_commands);
                    let dialog = adw::AlertDialog::builder()
                        .heading(&crate::t!("dialog-reboot-required-title"))
                        .body(&crate::t!("dialog-dual-device-conflict-message"))
                        .build();
                    dialog.add_response("cancel", &crate::t!("common-cancel"));
                    dialog.add_response("confirm", &crate::t!("common-confirm"));
                    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

                    dialog.choose(
                        Some(&switch_for_dialog),
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "confirm" {
                                switch_for_action.set_sensitive(false);
                                let _ = worker_commands
                                    .borrow()
                                    .as_ref()
                                    .map(|s| s.send(WorkerCommand::SetDualDeviceConnection(true)));
                            } else {
                                let _guard = updating_dual_confirm.enter();
                                switch_for_action.set_state(false);
                                switch_for_action.set_active(false);
                            }
                        },
                    );

                    return gtk::glib::Propagation::Stop;
                }

                switch.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|s| s.send(WorkerCommand::SetDualDeviceConnection(state)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ = updates_tx
                            .send(UiUpdate::Status(crate::t!("dual-device-send-failed", err)));
                        if let Some(en) = current_dual.get() {
                            controls_state::update_dual_device(&controls_widgets_clone, en);
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(en) = current_dual.get() {
                            controls_state::update_dual_device(&controls_widgets_clone, en);
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let in_ear_flag = controls_widgets.in_ear_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_in_ear = Rc::clone(&current_in_ear);

        controls_widgets
            .in_ear_switch
            .connect_state_set(move |switch, state| {
                if in_ear_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                switch.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|s| s.send(WorkerCommand::SetInEarDetection(state)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("wear-send-failed", err)));
                        if let Some(en) = current_in_ear.get() {
                            controls_state::update_in_ear_detection(&controls_widgets_clone, en);
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(en) = current_in_ear.get() {
                            controls_state::update_in_ear_detection(&controls_widgets_clone, en);
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let sleep_flag = controls_widgets.sleep_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let current_sleep = Rc::clone(&current_sleep);

        controls_widgets
            .sleep_switch
            .connect_state_set(move |switch, state| {
                if sleep_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                switch.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|s| s.send(WorkerCommand::SetSleepDetection(state)));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(err)) => {
                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                            "sleep-detection-send-failed",
                            err
                        )));
                        if let Some(en) = current_sleep.get() {
                            controls_state::update_sleep_detection(&controls_widgets_clone, en);
                        }
                    }
                    None => {
                        let _ =
                            updates_tx.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        if let Some(en) = current_sleep.get() {
                            controls_state::update_sleep_detection(&controls_widgets_clone, en);
                        }
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let preferred_flag = controls_widgets.preferred_flag.clone();
        let controls_widgets_clone = controls_widgets.clone();
        let cached_devices = Rc::clone(&cached_paired_devices);
        let current_pref = Rc::clone(&current_preferred_device);

        controls_widgets
            .preferred_row
            .connect_selected_notify(move |row| {
                if preferred_flag.is_updating() {
                    return;
                }
                if let Some(id) = row.selected_id() {
                    let target_mac = if id == 0 {
                        None
                    } else {
                        let idx = (id - 1) as usize;
                        cached_devices.borrow().get(idx).map(|d| d.address)
                    };
                    row.set_sensitive(false);
                    let command_result = worker_commands
                        .borrow()
                        .as_ref()
                        .map(|s| s.send(WorkerCommand::SetPreferredDevice(target_mac)));
                    match command_result {
                        Some(Ok(())) => {}
                        Some(Err(err)) => {
                            let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                "controls-preferred-device-send-failed",
                                err
                            )));
                            controls_state::update_preferred_devices(
                                &controls_widgets_clone,
                                *current_pref.borrow(),
                                &cached_devices.borrow(),
                            );
                        }
                        None => {
                            let _ = updates_tx
                                .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                            controls_state::update_preferred_devices(
                                &controls_widgets_clone,
                                *current_pref.borrow(),
                                &cached_devices.borrow(),
                            );
                        }
                    }
                }
            });
    }

    let send_mode = {
        let worker_commands = Rc::clone(&worker_commands);
        let updates_tx = updates_tx.clone();
        let noise_buttons = primary_buttons.clone();
        let noise_row = widgets.noise_option_row.clone();
        let session_state = Rc::clone(&session_state);
        Rc::new(move |protocol_index: u8| -> Result<(), String> {
            {
                let st = session_state.borrow();
                if !st.connected {
                    return Err(crate::t!("channel-not-connected"));
                }
                let worn = !st.supports_wear_detection || st.wear.any_earbud_in_ear();
                if !worn {
                    return Err(crate::t!("wear-earbuds-not-in-ear"));
                }
                if !st.noise.enabled {
                    return Err(crate::t!("noise-mode-blocked"));
                }
            }
            let command_result = worker_commands
                .borrow()
                .as_ref()
                .map(|sender| sender.send(WorkerCommand::SetNoiseMode(protocol_index)));
            match command_result {
                Some(Ok(())) => {
                    for (_, button) in &noise_buttons {
                        button.set_sensitive(false);
                    }
                    noise_row.set_sensitive(false);
                    Ok(())
                }
                Some(Err(error)) => {
                    let msg = crate::t!("noise-mode-send-failed", error);
                    let _ = updates_tx.send(UiUpdate::Status(msg.clone()));
                    Err(msg)
                }
                None => {
                    let msg = crate::t!("channel-not-connected");
                    let _ = updates_tx.send(UiUpdate::Status(msg.clone()));
                    Err(msg)
                }
            }
        })
    };

    if let Some(ref svc) = quick_settings {
        let send_mode_for_set = Rc::clone(&send_mode);
        let send_mode_for_toggle = Rc::clone(&send_mode);
        let session_state_for_toggle = Rc::clone(&session_state);
        let present_window = Rc::clone(&present_window);

        svc.set_handlers(
            move |mode_idx| send_mode_for_set(mode_idx),
            move || {
                let target = {
                    let st = session_state_for_toggle.borrow();
                    resolve_toggle_target(
                        st.noise.current_mode,
                        st.noise.last_non_off_mode,
                        &st.noise.supported_indices,
                        &st.noise.modes,
                    )
                };
                match target {
                    Some(idx) => send_mode_for_toggle(idx),
                    None => Err(crate::t!("noise-mode-blocked")),
                }
            },
            move || present_window(None),
        );
    }

    if let Some(ref tray_service) = tray {
        let send_mode_for_tray = Rc::clone(&send_mode);
        let present_window = Rc::clone(&present_window);
        let quit_app = Rc::clone(&quit_app);
        tray_service.set_handlers(
            move |activation_token| present_window(activation_token),
            move || quit_app(),
            move |mode_idx| send_mode_for_tray(mode_idx),
        );
    }

    let widgets = widgets_ref(&widgets);
    let mut battery = BatteryState::default();
    let mut wear = crate::protocol::WearState::default();
    let mut case_battery_is_stale = false;
    let mut connected = false;
    let mut supports_wear_detection = true;
    let mut noise_controls_enabled = false;
    let mut noise_mode_indices = Vec::new();
    let mut noise_modes = Vec::new();
    let mut noise_mode_support_note = crate::t!("noise-mode-read-support-after-connecting");
    let mut current_noise_mode = None;
    let mut paired_devices = Err(crate::t!("dual-device-read-after-connecting"));
    let mut paired_device_controls_enabled = Err(crate::t!("dual-device-reading-status"));
    let scanning_for_updates = Rc::clone(&scanning);
    let auto_connect_pending_for_updates = Rc::clone(&auto_connect_pending);
    let device_choices_for_updates = Rc::clone(&device_choices);
    let connected_device_for_updates = Rc::clone(&connected_device);
    let pending_connect_for_updates = Rc::clone(&pending_connect);
    let worker_commands_for_updates = Rc::clone(&worker_commands);
    let paired_action_pending_for_updates = Rc::clone(&paired_action_pending);
    let updates_tx_for_updates = updates_tx.clone();
    let refresh_button_for_updates = widgets.refresh_devices.clone();
    let session_state_for_updates = Rc::clone(&session_state);
    let current_artwork_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    {
        let current_artwork_path = Rc::clone(&current_artwork_path);
        let model_picture = widgets.model_picture.clone();
        let active_turntable = Rc::clone(&widgets.active_turntable);
        let scene_view = widgets.scene_view.clone();
        adw::StyleManager::default().connect_dark_notify(move |manager| {
            if active_turntable.borrow().is_none() && !scene_view.has_scene() {
                if let Some(path) = current_artwork_path.borrow().as_deref() {
                    if crate::resources::is_generic_artwork(path) {
                        let new_path = crate::resources::generic_artwork_path(manager.is_dark());
                        model_picture.set_filename(Some(&new_path));
                    }
                }
            }
        });
    }
    start_device_discovery(
        updates_tx.clone(),
        widgets.refresh_devices.clone(),
        Rc::clone(&scanning),
        false,
    );
    {
        let updates_tx = updates_tx.clone();
        let refresh_button = widgets.refresh_devices.clone();
        let scanning = Rc::clone(&scanning);
        gtk::glib::timeout_add_local(Duration::from_secs(15), move || {
            start_device_discovery(
                updates_tx.clone(),
                refresh_button.clone(),
                Rc::clone(&scanning),
                false,
            );
            gtk::glib::ControlFlow::Continue
        });
    }

    gtk::glib::spawn_future_local(async move {
        while let Some(update) = async_rx.recv().await {
            match update {
                UiUpdate::Status(message) => {
                    device_widgets
                        .connection
                        .set_text(&crate::t!("common-status", message));
                }
                UiUpdate::SettingError { domain, message } => {
                    device_widgets
                        .connection
                        .set_text(&crate::t!("common-status", message));
                    match domain {
                        SettingDomain::NoiseMode => {
                            update_noise_controls(
                                &widgets,
                                &NoiseControlsViewState {
                                    supported_indices: &noise_mode_indices,
                                    support_note: &noise_mode_support_note,
                                    capability_enabled: noise_controls_enabled && connected,
                                    supports_wear_detection,
                                    wear: &wear,
                                    current_mode: current_noise_mode,
                                    modes: &noise_modes,
                                },
                            );
                        }
                        SettingDomain::Equalizer => {
                            update_equalizer(&widgets, current_eq_preset.get());
                            widgets.eq_row.set_sensitive(current_supports_eq.get());
                            sound_state::update_sound_page_equalizer(
                                &sound_widgets,
                                current_eq_preset.get(),
                            );
                        }
                        SettingDomain::SpatialAudio => {
                            update_spatial_audio(&widgets, current_spatial_mode.get());
                            widgets
                                .spatial_row
                                .set_sensitive(current_supports_spatial.get());
                            sound_state::update_sound_page_spatial(
                                &sound_widgets,
                                current_spatial_mode.get(),
                            );
                        }
                        SettingDomain::GameMode => {
                            update_game_mode(&widgets, current_game_mode.get());
                            widgets
                                .game_mode_switch
                                .set_sensitive(current_supports_game_mode.get());
                        }
                        SettingDomain::BassEngine => {
                            if let Some(enabled) = current_bass_engine_enabled.get() {
                                sound_state::update_sound_page_bass_enabled(
                                    &sound_widgets,
                                    enabled,
                                );
                            }
                            if let Some(level) = current_bass_engine_level.borrow().as_ref() {
                                sound_state::update_sound_page_bass_level(&sound_widgets, *level);
                            }
                            sound_widgets
                                .bass_switch
                                .set_sensitive(current_supports_bass_engine.get());
                            sound_widgets
                                .bass_switch_row
                                .set_sensitive(current_supports_bass_engine.get());
                        }
                        SettingDomain::HiRes => {
                            if let Some(enabled) = current_hi_res_enabled.get() {
                                sound_state::update_sound_page_hi_res(&sound_widgets, enabled);
                            }
                            sound_widgets
                                .hires_switch
                                .set_sensitive(current_supports_hi_res.get());
                            sound_widgets
                                .hires_switch_row
                                .set_sensitive(current_supports_hi_res.get());
                        }
                        SettingDomain::AudioCodec => {
                            if let Some(codec) = current_audio_codec.get() {
                                sound_state::update_sound_page_audio_codec(&sound_widgets, codec);
                            }
                            sound_widgets
                                .codec_row
                                .set_sensitive(current_supports_audio_codec.get());
                        }
                        SettingDomain::Controls => {}
                    }

                    for (_, combo) in &controls_widgets.gesture_combos {
                        combo.set_sensitive(true);
                    }
                    controls_widgets.cycle_check_anc.set_sensitive(true);
                    controls_widgets
                        .cycle_check_transparency
                        .set_sensitive(true);
                    controls_widgets.cycle_check_off.set_sensitive(true);
                    controls_widgets.find_btn.set_sensitive(true);
                    controls_widgets.fit_btn.set_sensitive(true);
                    controls_widgets.dual_switch.set_sensitive(true);
                    controls_widgets.dual_row.set_sensitive(true);
                    controls_widgets.in_ear_switch.set_sensitive(true);
                    controls_widgets.in_ear_row.set_sensitive(true);
                    controls_widgets.sleep_switch.set_sensitive(true);
                    controls_widgets.sleep_row.set_sensitive(true);
                    controls_widgets.head_switch.set_sensitive(true);
                    controls_widgets.head_switch_row.set_sensitive(true);
                    controls_widgets.head_motion_row.set_sensitive(true);
                    controls_widgets.preferred_row.set_sensitive(true);
                    controls_widgets.prompt_volume_scale.set_sensitive(true);
                }
                UiUpdate::Devices(result) => {
                    scanning_for_updates.set(false);
                    refresh_button_for_updates.set_sensitive(true);
                    refresh_button_for_updates
                        .set_tooltip_text(Some(&crate::t!("device-refresh-list")));
                    match result {
                        Ok(mut devices) => {
                            if let Some(active_device) =
                                connected_device_for_updates.borrow().as_ref()
                                && !devices
                                    .iter()
                                    .any(|device| device.is_same_device(active_device))
                            {
                                devices.push(active_device.clone());
                            }
                            devices.sort_by(|left, right| {
                                left.brand
                                    .label()
                                    .cmp(right.brand.label())
                                    .then_with(|| left.name.cmp(&right.name))
                            });
                            *device_choices_for_updates.borrow_mut() = devices.clone();

                            render_candidate_devices(
                                &CandidateDeviceListWidgets {
                                    list: &widgets.device_list,
                                    empty_label: &widgets.device_list_empty,
                                    popover: &widgets.device_popover,
                                    connection_tag: &widgets.connection_tag,
                                    device_name: &widgets.device_name,
                                },
                                &devices,
                                connected_device_for_updates.borrow().as_ref(),
                                &CandidateDeviceContext {
                                    updates_tx: &updates_tx_for_updates,
                                    worker_commands: &worker_commands_for_updates,
                                    pending_connect: &pending_connect_for_updates,
                                    active_connected_device: &connected_device_for_updates,
                                    auto_connect_pending: &auto_connect_pending_for_updates,
                                },
                            );

                            if connected_device_for_updates.borrow().is_none() {
                                if let Some(first_device) = devices.first() {
                                    widgets.device_name.set_text(&first_device.name);
                                } else {
                                    widgets
                                        .device_name
                                        .set_text(&crate::t!("device-no-supported-connected"));
                                }
                            }
                            if devices.is_empty() && !connected {
                                device_widgets
                                    .connection
                                    .set_text(&crate::t!("device-status-no-connected"));
                            }
                            // Auto-connect once per app launch: pick the first discovered
                            // headset that exposes the control service. If none is found yet,
                            // keep waiting for a later periodic/manual scan. A manual selection
                            // from the device list cancels auto-connect.
                            let idle = worker_commands_for_updates.borrow().is_none()
                                && pending_connect_for_updates.borrow().is_none();
                            let auto_connect_target =
                                if auto_connect_pending_for_updates.get() && idle {
                                    devices
                                        .iter()
                                        .find(|device| device.has_control_service)
                                        .cloned()
                                } else {
                                    None
                                };
                            if let Some(target) = auto_connect_target {
                                auto_connect_pending_for_updates.set(false);
                                widgets.device_name.set_text(&target.name);
                                widgets
                                    .connection_tag
                                    .set_text(&crate::t!("common-connecting"));
                                widgets
                                    .connection_tag
                                    .remove_css_class("connection-tag-connected");
                                widgets
                                    .connection_tag
                                    .add_css_class("connection-tag-disconnected");
                                start_monitor_for_device(
                                    target,
                                    updates_tx_for_updates.clone(),
                                    Rc::clone(&worker_commands_for_updates),
                                );
                            }
                        }
                        Err(error) => {
                            if !connected && worker_commands_for_updates.borrow().is_none() {
                                device_widgets
                                    .connection
                                    .set_text(&crate::t!("device-status-discovery-failed", error));
                            }
                        }
                    }
                }
                UiUpdate::Connecting { device } => {
                    if !connected {
                        connection_state.show_connecting(&device.name);
                    }
                }
                UiUpdate::Connected { device } => {
                    connected = true;
                    connection_state.show_content();
                    widgets.clear_turntable();
                    *current_artwork_path.borrow_mut() = None;
                    widgets.scene_view.clear();
                    widgets.model_stack.set_visible_child_name("picture");
                    widgets
                        .model_picture
                        .set_paintable(None::<&gtk::gdk::Paintable>);
                    *connected_device_for_updates.borrow_mut() = Some(device.clone());
                    battery = BatteryState::default();
                    case_battery_is_stale = false;
                    wear = crate::protocol::WearState::default();
                    current_noise_mode = None;
                    noise_mode_indices.clear();
                    noise_modes.clear();
                    noise_controls_enabled = false;
                    noise_mode_support_note = crate::t!("noise-mode-reading-and-wear");
                    device_widgets
                        .wear
                        .set_text(&crate::t!("wear-status-waiting"));
                    supports_wear_detection = true;
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.reset_on_disconnect();
                        st.connected = true;
                        st.device_name = device.name.clone();
                        st.supports_wear_detection = true;
                    }
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: false,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: None,
                            modes: &noise_modes,
                        },
                    );
                    current_eq_preset.set(None);
                    current_spatial_mode.set(None);
                    current_game_mode.set(None);
                    current_eq_presets.borrow_mut().clear();
                    current_custom_eqs.borrow_mut().clear();
                    current_supports_eq.set(false);
                    current_supports_spatial.set(false);
                    current_supports_game_mode.set(false);
                    current_supports_bass_engine.set(false);
                    current_supports_hi_res.set(false);
                    current_supports_audio_codec.set(false);
                    current_available_codecs.borrow_mut().clear();
                    current_audio_codec.set(None);
                    current_bass_engine_enabled.set(None);
                    *current_bass_engine_level.borrow_mut() = None;
                    current_hi_res_enabled.set(None);
                    reset_sound_controls(&widgets);
                    sound_state::reset_sound_page(&sound_widgets);
                    widgets.device_name.set_text(&device.name);
                    widgets
                        .connection_tag
                        .set_text(&crate::t!("common-connected"));
                    widgets
                        .connection_tag
                        .remove_css_class("connection-tag-disconnected");
                    widgets
                        .connection_tag
                        .add_css_class("connection-tag-connected");
                    paired_devices = Err(crate::t!("dual-device-reading-list"));
                    paired_device_controls_enabled = Err(crate::t!("dual-device-reading-status"));
                    paired_action_pending_for_updates.set(false);
                    render_paired_devices(
                        &widgets.paired_devices,
                        paired_devices.clone(),
                        &paired_device_controls_enabled,
                        &paired_action_pending_for_updates,
                        &worker_commands_for_updates,
                        &updates_tx_for_updates,
                    );
                    widgets
                        .device_menu_button
                        .set_tooltip_text(Some(&device.name));
                    render_candidate_devices(
                        &CandidateDeviceListWidgets {
                            list: &widgets.device_list,
                            empty_label: &widgets.device_list_empty,
                            popover: &widgets.device_popover,
                            connection_tag: &widgets.connection_tag,
                            device_name: &widgets.device_name,
                        },
                        &device_choices_for_updates.borrow(),
                        Some(&device),
                        &CandidateDeviceContext {
                            updates_tx: &updates_tx_for_updates,
                            worker_commands: &worker_commands_for_updates,
                            pending_connect: &pending_connect_for_updates,
                            active_connected_device: &connected_device_for_updates,
                            auto_connect_pending: &auto_connect_pending_for_updates,
                        },
                    );
                    device_widgets
                        .connection
                        .set_text(&crate::t!("channel-status-connected"));
                    device_widgets
                        .name
                        .set_text(&crate::t!("device-name", device.name));
                    for indicator in [
                        &widgets.left_battery,
                        &widgets.right_battery,
                        &widgets.case_battery,
                    ] {
                        set_battery(indicator, None, false);
                    }
                    widgets.model_firmware.set_visible(false);
                    widgets.model_firmware.set_text("");
                }
                UiUpdate::ProductId {
                    product_id,
                    color_id,
                } => {
                    let profile = crate::compatibility::profile_for_product_id(product_id);
                    supports_wear_detection =
                        profile.is_none_or(|profile| profile.supports_wear_detection());
                    if !supports_wear_detection {
                        device_widgets
                            .wear
                            .set_text(&crate::t!("wear-status-not-supported"));
                    }
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.product_id = Some(product_id);
                        st.supports_wear_detection = supports_wear_detection;
                    }
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                    let supports_dual_device =
                        profile.is_some_and(|profile| profile.supports_dual_device());
                    widgets
                        .paired_devices_section
                        .set_visible(supports_dual_device);
                    let supports_noise =
                        profile.is_some_and(|profile| profile.supports_noise_control());
                    widgets.mode_group.set_visible(supports_noise);

                    let supports_eq = profile.is_some_and(|p| p.supports_equalizer());
                    let supports_custom_eq = profile.is_some_and(|p| p.supports_custom_equalizer());
                    let supports_spatial = profile.is_some_and(|p| p.supports_spatial_audio());
                    let supports_game = profile.is_some_and(|p| p.supports_game_mode());
                    let supports_head_track = profile.is_some_and(|p| p.supports_head_tracking());
                    let supports_bass = profile.is_some_and(|p| p.supports_bass_engine());
                    let supports_hires = profile.is_some_and(|p| p.supports_hi_res());
                    let supports_codec = profile.is_some_and(|p| p.supports_audio_codec());
                    let available_codecs =
                        crate::compatibility::supported_codecs_for_product(product_id).to_vec();

                    current_supports_eq.set(supports_eq);
                    current_supports_custom_eq.set(supports_custom_eq);
                    current_supports_spatial.set(supports_spatial);
                    current_supports_game_mode.set(supports_game);
                    current_supports_bass_engine.set(supports_bass);
                    current_supports_hi_res.set(supports_hires);
                    current_supports_audio_codec.set(supports_codec);
                    *current_available_codecs.borrow_mut() = available_codecs.clone();
                    let presets =
                        crate::compatibility::equalizer_presets_for_product(product_id).to_vec();
                    *current_eq_presets.borrow_mut() = presets.clone();

                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.sound.supports_eq = supports_eq;
                        st.sound.supports_custom_eq = supports_custom_eq;
                        st.sound.supports_spatial = supports_spatial;
                        st.sound.supports_head_tracking = supports_head_track;
                        st.sound.supports_game_mode = supports_game;
                        st.sound.supports_bass_engine = supports_bass;
                        st.sound.supports_hi_res = supports_hires;
                        st.sound.supports_audio_codec = supports_codec;
                        st.sound.available_codecs = available_codecs.clone();
                        st.sound.presets = presets.clone();
                        st.dual_device.supports_dual_device = supports_dual_device;
                    }

                    update_sound_features(
                        &widgets,
                        &HomeSoundFeaturesViewState {
                            supports_equalizer: supports_eq,
                            supports_spatial_audio: supports_spatial,
                            supports_head_tracking: supports_head_track,
                            supports_game_mode: supports_game,
                            presets: &presets,
                            custom_equalizers: &current_custom_eqs.borrow(),
                        },
                    );
                    sound_state::update_sound_page_features(
                        &sound_widgets,
                        &SoundPageFeaturesViewState {
                            supports_equalizer: supports_eq,
                            supports_custom_equalizer: supports_custom_eq,
                            supports_spatial_audio: supports_spatial,
                            supports_head_tracking: supports_head_track,
                            supports_bass_engine: supports_bass,
                            supports_hi_res: supports_hires,
                            supports_audio_codec: supports_codec,
                            available_codecs: &available_codecs,
                            presets: &presets,
                        },
                    );
                    device_widgets
                        .product_id
                        .set_text(&crate::t!("device-product-id", format!("{product_id:06X}")));
                    crate::resources::spawn_artwork_fetch(
                        product_id,
                        color_id,
                        updates_tx_for_updates.clone(),
                    );
                }
                UiUpdate::DeviceArtwork(path) => {
                    session_state_for_updates.borrow_mut().artwork_path = Some(path.clone());
                    *current_artwork_path.borrow_mut() = Some(path.clone());
                    if widgets.active_turntable.borrow().is_none()
                        && !widgets.scene_view.has_scene()
                    {
                        let display_path = if crate::resources::is_generic_artwork(&path) {
                            let is_dark = adw::StyleManager::default().is_dark();
                            crate::resources::generic_artwork_path(is_dark)
                        } else {
                            path
                        };
                        widgets.model_picture.set_filename(Some(&display_path));
                        widgets.model_stack.set_visible_child_name("picture");
                    }
                }
                UiUpdate::DeviceModel(path) => {
                    widgets.scene_view.clear();
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Some(webp) = crate::resources::turntable::AnimatedWebp::parse(bytes)
                        {
                            let turntable = crate::resources::turntable::TurntableModel::new(webp);
                            widgets.set_turntable(turntable);
                        }
                    }
                }
                UiUpdate::DeviceScene(path) => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        match crate::resources::vfxms::VfxScene::parse(&bytes) {
                            Ok(scene) => match widgets.scene_view.load_scene(&scene) {
                                Ok(()) => {
                                    widgets.clear_turntable();
                                    widgets.model_stack.set_visible_child_name("scene3d");
                                    widgets.model_placeholder.set_visible(false);
                                }
                                Err(e) => {
                                    eprintln!("Failed to load 3D scene: {e}");
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed to parse VFXMS: {e}");
                            }
                        }
                    }
                }
                UiUpdate::Firmware(firmware) => {
                    let not_reported = crate::t!("common-not-reported");
                    let fw_text = match (&firmware.left, &firmware.right, &firmware.case) {
                        (Some(l), Some(r), Some(c)) => {
                            crate::t!("firmware-version-full", l, r, c)
                        }
                        (Some(l), Some(r), None) => {
                            crate::t!("firmware-version-left-right", l, r)
                        }
                        (Some(v), None, None) | (None, Some(v), None) => {
                            crate::t!("firmware-version-single", v)
                        }
                        _ => crate::t!(
                            "firmware-version-full",
                            firmware.left.as_deref().unwrap_or(&not_reported),
                            firmware.right.as_deref().unwrap_or(&not_reported),
                            firmware.case.as_deref().unwrap_or(&not_reported),
                        ),
                    };
                    device_widgets.firmware.set_text(&fw_text);
                    widgets.model_firmware.set_text(&fw_text);
                    widgets.model_firmware.set_visible(true);
                }
                UiUpdate::Wear(update) => {
                    merge_wear(&mut wear, update);
                    *current_wear.borrow_mut() = wear.clone();
                    if !supports_wear_detection {
                        device_widgets
                            .wear
                            .set_text(&crate::t!("wear-status-not-supported"));
                    } else {
                        let case_status = wear
                            .case_open
                            .map(|open| {
                                if open {
                                    crate::t!("wear-case-open")
                                } else {
                                    crate::t!("wear-case-closed")
                                }
                            })
                            .unwrap_or_else(|| crate::t!("wear-case-status-unknown"));
                        let wear_left =
                            wear_part_name(&crate::t!("battery-left-earbud"), wear.left);
                        let wear_right =
                            wear_part_name(&crate::t!("battery-right-earbud"), wear.right);
                        device_widgets.wear.set_text(&crate::t!(
                            "wear-status",
                            format!("{wear_left}; {wear_right}; {case_status}")
                        ));
                    }
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: noise_controls_enabled && connected,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: current_noise_mode,
                            modes: &noise_modes,
                        },
                    );
                    session_state_for_updates.borrow_mut().wear = wear.clone();
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                }
                UiUpdate::Battery(update) => {
                    if update.case.is_some() {
                        case_battery_is_stale = false;
                    } else if battery.case.is_some() {
                        case_battery_is_stale = true;
                    }
                    merge_battery(&mut battery, update);
                    set_battery(&widgets.left_battery, battery.left, false);
                    set_battery(&widgets.right_battery, battery.right, false);
                    set_battery(&widgets.case_battery, battery.case, case_battery_is_stale);
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.battery = battery.clone();
                        st.case_battery_is_stale = case_battery_is_stale;
                    }
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                }
                UiUpdate::PairedDevices(result) => {
                    let mut sorted_result = result.clone();
                    if let Ok(devs) = &mut sorted_result {
                        devs.sort_by_key(|d| (!d.connected, !d.is_current_host));
                        *cached_paired_devices.borrow_mut() = devs.clone();
                        controls_state::update_preferred_devices(
                            &controls_widgets,
                            *current_preferred_device.borrow(),
                            devs,
                        );
                    }
                    session_state_for_updates
                        .borrow_mut()
                        .dual_device
                        .paired_devices = sorted_result.clone();
                    paired_devices = sorted_result.clone();
                    render_paired_devices(
                        &widgets.paired_devices,
                        paired_devices.clone(),
                        &paired_device_controls_enabled,
                        &paired_action_pending_for_updates,
                        &worker_commands_for_updates,
                        &updates_tx_for_updates,
                    );
                }
                UiUpdate::PairedDeviceControlsEnabled(result) => {
                    if let Err(error) = &result {
                        let _ = updates_tx_for_updates.send(UiUpdate::Status(crate::t!(
                            "dual-device-confirm-status-failed",
                            error
                        )));
                    }
                    session_state_for_updates
                        .borrow_mut()
                        .dual_device
                        .controls_enabled = result.clone();
                    paired_device_controls_enabled = result;
                    render_paired_devices(
                        &widgets.paired_devices,
                        paired_devices.clone(),
                        &paired_device_controls_enabled,
                        &paired_action_pending_for_updates,
                        &worker_commands_for_updates,
                        &updates_tx_for_updates,
                    );
                }
                UiUpdate::PairedDeviceActionFinished => {
                    paired_action_pending_for_updates.set(false);
                    render_paired_devices(
                        &widgets.paired_devices,
                        paired_devices.clone(),
                        &paired_device_controls_enabled,
                        &paired_action_pending_for_updates,
                        &worker_commands_for_updates,
                        &updates_tx_for_updates,
                    );
                }
                UiUpdate::NoiseMode(index) => {
                    current_noise_mode = Some(index);
                    if !noise_mode_indices.contains(&index) {
                        noise_mode_indices.push(index);
                    }
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: noise_controls_enabled && connected,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: current_noise_mode,
                            modes: &noise_modes,
                        },
                    );
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.noise.current_mode = Some(index);
                        st.noise.supported_indices = noise_mode_indices.clone();
                        if is_non_off_writable_mode(index, &st.noise.modes) {
                            st.noise.last_non_off_mode = Some(index);
                        }
                    }
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                }
                UiUpdate::NoiseControlsEnabled(enabled) => {
                    noise_controls_enabled = enabled;
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: noise_controls_enabled && connected,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: current_noise_mode,
                            modes: &noise_modes,
                        },
                    );
                    session_state_for_updates.borrow_mut().noise.enabled = enabled && connected;
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                }
                UiUpdate::NoiseModeOptions {
                    indices,
                    note,
                    modes,
                } => {
                    noise_mode_indices = indices;
                    noise_mode_support_note = note;
                    noise_modes = modes;
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: noise_controls_enabled && connected,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: current_noise_mode,
                            modes: &noise_modes,
                        },
                    );
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.noise.supported_indices = noise_mode_indices.clone();
                        st.noise.modes = noise_modes.clone();
                        st.noise.support_note = noise_mode_support_note.clone();
                        if let Some(idx) = st.noise.current_mode
                            && is_non_off_writable_mode(idx, &st.noise.modes)
                        {
                            st.noise.last_non_off_mode = Some(idx);
                        }
                    }
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
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
                        let mut st = session_state_for_updates.borrow_mut();
                        st.sound.supports_eq = supports_equalizer;
                        st.sound.supports_custom_eq = supports_custom_equalizer;
                        st.sound.supports_spatial = supports_spatial_audio;
                        st.sound.supports_head_tracking = supports_head_tracking;
                        st.sound.supports_game_mode = supports_game_mode;
                        st.sound.supports_bass_engine = supports_bass_engine;
                        st.sound.supports_hi_res = supports_hi_res;
                        st.sound.supports_audio_codec = supports_audio_codec;
                        st.sound.available_codecs = available_codecs.clone();
                        st.sound.presets = presets.clone();
                    }
                    current_supports_eq.set(supports_equalizer);
                    current_supports_custom_eq.set(supports_custom_equalizer);
                    current_supports_spatial.set(supports_spatial_audio);
                    current_supports_game_mode.set(supports_game_mode);
                    current_supports_bass_engine.set(supports_bass_engine);
                    current_supports_hi_res.set(supports_hi_res);
                    current_supports_audio_codec.set(supports_audio_codec);
                    *current_available_codecs.borrow_mut() = available_codecs.clone();
                    *current_eq_presets.borrow_mut() = presets.clone();
                    update_sound_features(
                        &widgets,
                        &HomeSoundFeaturesViewState {
                            supports_equalizer,
                            supports_spatial_audio,
                            supports_head_tracking,
                            supports_game_mode,
                            presets: &presets,
                            custom_equalizers: &current_custom_eqs.borrow(),
                        },
                    );
                    sound_state::update_sound_page_features(
                        &sound_widgets,
                        &SoundPageFeaturesViewState {
                            supports_equalizer,
                            supports_custom_equalizer,
                            supports_spatial_audio,
                            supports_head_tracking,
                            supports_bass_engine,
                            supports_hi_res,
                            supports_audio_codec,
                            available_codecs: &available_codecs,
                            presets: &presets,
                        },
                    );
                }
                UiUpdate::Equalizer(preset) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .current_eq_preset = Some(preset);
                    current_eq_preset.set(Some(preset));
                    update_equalizer(&widgets, current_eq_preset.get());
                    widgets.eq_row.set_sensitive(current_supports_eq.get());
                    sound_state::update_sound_page_equalizer(
                        &sound_widgets,
                        current_eq_preset.get(),
                    );
                }
                UiUpdate::CustomEqualizers(entries) => {
                    let worker_commands_select = Rc::clone(&worker_commands);
                    let updates_tx_select = updates_tx.clone();
                    let widgets_eq_row_select = widgets.eq_row.clone();
                    let widgets_eq_flag_select = widgets.eq_flag.clone();

                    let on_select = move |id: u8| {
                        let command_result = worker_commands_select
                            .borrow()
                            .as_ref()
                            .map(|sender| sender.send(WorkerCommand::SelectCustomEqualizer(id)));
                        match command_result {
                            Some(Ok(())) => {
                                let _guard = widgets_eq_flag_select.enter();
                                widgets_eq_row_select.set_selected_id(Some(id));
                                widgets_eq_row_select.set_sensitive(false);
                            }
                            Some(Err(error)) => {
                                let _ = updates_tx_select.send(UiUpdate::Status(crate::t!(
                                    "equalizer-send-failed",
                                    error
                                )));
                            }
                            None => {
                                let _ = updates_tx_select
                                    .send(UiUpdate::Status(crate::t!("channel-not-connected")));
                            }
                        }
                    };

                    let worker_commands_edit = Rc::clone(&worker_commands);
                    let updates_tx_edit = updates_tx.clone();
                    let on_edit =
                        move |preset: &crate::protocol::CustomEqualizer,
                              row_widget: &gtk::Widget| {
                            let worker_commands = Rc::clone(&worker_commands_edit);
                            let updates_tx = updates_tx_edit.clone();
                            let id = preset.id;
                            crate::ui::dialogs::show_custom_eq_dialog(
                                row_widget,
                                Some(preset),
                                &preset.name,
                                move |name, bands| {
                                    let cmd =
                                        WorkerCommand::SaveCustomEqualizer { id, name, bands };
                                    if let Some(sender) = worker_commands.borrow().as_ref() {
                                        let _ = sender.send(cmd);
                                    } else {
                                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                            "channel-not-connected"
                                        )));
                                    }
                                },
                            );
                        };

                    let worker_commands_del = Rc::clone(&worker_commands);
                    let updates_tx_del = updates_tx.clone();
                    let on_delete = move |id: u8, row_widget: &gtk::Widget| {
                        let worker_commands = Rc::clone(&worker_commands_del);
                        let updates_tx = updates_tx_del.clone();
                        let alert = adw::AlertDialog::builder()
                            .heading(&crate::t!("equalizer-custom-delete-confirm"))
                            .build();
                        alert.add_response("cancel", &crate::t!("common-cancel"));
                        alert.add_response("delete", &crate::t!("common-delete"));
                        alert.set_response_appearance(
                            "delete",
                            adw::ResponseAppearance::Destructive,
                        );
                        alert.choose(
                            Some(row_widget),
                            None::<&gtk::gio::Cancellable>,
                            move |response| {
                                if response == "delete" {
                                    let cmd = WorkerCommand::DeleteCustomEqualizer(id);
                                    if let Some(sender) = worker_commands.borrow().as_ref() {
                                        let _ = sender.send(cmd);
                                    } else {
                                        let _ = updates_tx.send(UiUpdate::Status(crate::t!(
                                            "channel-not-connected"
                                        )));
                                    }
                                }
                            },
                        );
                    };

                    *current_custom_eqs.borrow_mut() = entries.clone();
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .custom_equalizers = entries.clone();
                    if let Some(selected_custom) = entries.iter().find(|e| e.is_selected) {
                        current_eq_preset.set(Some(selected_custom.id));
                        session_state_for_updates
                            .borrow_mut()
                            .sound
                            .current_eq_preset = Some(selected_custom.id);
                    }
                    update_equalizer_items(
                        &widgets,
                        current_supports_eq.get(),
                        &current_eq_presets.borrow(),
                        &entries,
                        current_eq_preset.get(),
                    );

                    sound_state::update_sound_page_custom_equalizers(
                        &sound_widgets,
                        &entries,
                        current_supports_custom_eq.get(),
                        on_select,
                        on_edit,
                        on_delete,
                    );
                }
                UiUpdate::SpatialAudio(mode) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .current_spatial_mode = Some(mode);
                    current_spatial_mode.set(Some(mode));
                    update_spatial_audio(&widgets, current_spatial_mode.get());
                    widgets
                        .spatial_row
                        .set_sensitive(current_supports_spatial.get());
                    sound_state::update_sound_page_spatial(
                        &sound_widgets,
                        current_spatial_mode.get(),
                    );
                }
                UiUpdate::GameMode(enabled) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .current_game_mode = Some(enabled);
                    current_game_mode.set(Some(enabled));
                    update_game_mode(&widgets, current_game_mode.get());
                }
                UiUpdate::BassEngineEnabled(enabled) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .bass_engine_enabled = Some(enabled);
                    current_bass_engine_enabled.set(Some(enabled));
                    sound_state::update_sound_page_bass_enabled(&sound_widgets, enabled);
                    sound_widgets
                        .bass_switch
                        .set_sensitive(current_supports_bass_engine.get());
                    sound_widgets
                        .bass_switch_row
                        .set_sensitive(current_supports_bass_engine.get());
                }
                UiUpdate::BassEngineLevel(level) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .bass_engine_level = Some(level);
                    *current_bass_engine_level.borrow_mut() = Some(level);
                    sound_state::update_sound_page_bass_level(&sound_widgets, level);
                }
                UiUpdate::HiRes(enabled) => {
                    session_state_for_updates.borrow_mut().sound.hi_res_enabled = Some(enabled);
                    current_hi_res_enabled.set(Some(enabled));
                    sound_state::update_sound_page_hi_res(&sound_widgets, enabled);
                    sound_widgets
                        .hires_switch
                        .set_sensitive(current_supports_hi_res.get());
                    sound_widgets
                        .hires_switch_row
                        .set_sensitive(current_supports_hi_res.get());
                }
                UiUpdate::AudioCodec(codec) => {
                    session_state_for_updates
                        .borrow_mut()
                        .sound
                        .current_audio_codec = Some(codec);
                    current_audio_codec.set(Some(codec));
                    sound_state::update_sound_page_audio_codec(&sound_widgets, codec);
                    sound_widgets
                        .codec_row
                        .set_sensitive(current_supports_audio_codec.get());
                }
                UiUpdate::DeviceRebooting => {
                    device_widgets
                        .connection
                        .set_text(&crate::t!("device-rebooting"));
                    widgets
                        .connection_tag
                        .set_text(&crate::t!("device-rebooting"));
                }
                UiUpdate::ControlsFeatures {
                    supports_touch_customization,
                    supports_dual_device,
                    supports_wear_detection,
                    supports_sleep_detection,
                    supports_head_motion,
                    supports_find_device,
                    supports_fit_detection,
                    supports_prompt_volume,
                    gesture_rows,
                } => {
                    {
                        let mut st = session_state_for_updates.borrow_mut();
                        st.controls.supports_touch_customization = supports_touch_customization;
                        st.dual_device.supports_dual_device = supports_dual_device;
                        st.supports_wear_detection = supports_wear_detection;
                        st.controls.supports_sleep_detection = supports_sleep_detection;
                        st.controls.supports_head_motion = supports_head_motion;
                        st.controls.supports_find_device = supports_find_device;
                        st.controls.supports_fit_detection = supports_fit_detection;
                        st.controls.supports_prompt_volume = supports_prompt_volume;
                        st.controls.gesture_rows = gesture_rows.clone();
                    }
                    *current_gesture_rows.borrow_mut() = gesture_rows.clone();
                    controls_state::update_controls_features(
                        &controls_widgets,
                        &ControlsFeaturesViewState {
                            supports_touch: supports_touch_customization,
                            supports_dual: supports_dual_device,
                            supports_wear: supports_wear_detection,
                            supports_sleep: supports_sleep_detection,
                            supports_head: supports_head_motion,
                            supports_find: supports_find_device,
                            supports_fit: supports_fit_detection,
                            supports_volume: supports_prompt_volume,
                            gesture_rows: &gesture_rows,
                        },
                    );
                }
                UiUpdate::KeyFunctions(functions) => {
                    session_state_for_updates
                        .borrow_mut()
                        .controls
                        .key_functions = functions.clone();
                    *current_key_functions.borrow_mut() = functions.clone();
                    controls_state::update_key_functions(
                        &controls_widgets,
                        &functions,
                        selected_ear.get(),
                        &current_gesture_rows.borrow(),
                    );
                    for (_, combo) in &controls_widgets.gesture_combos {
                        combo.set_sensitive(true);
                    }
                }
                UiUpdate::NoiseCycle(cycle) => {
                    session_state_for_updates.borrow_mut().controls.noise_cycle = Some(cycle);
                    *current_noise_cycle.borrow_mut() = Some(cycle);
                    controls_state::update_noise_cycle(
                        &controls_widgets,
                        cycle,
                        selected_ear.get(),
                    );
                    controls_widgets.cycle_check_anc.set_sensitive(true);
                    controls_widgets
                        .cycle_check_transparency
                        .set_sensitive(true);
                    controls_widgets.cycle_check_off.set_sensitive(true);
                }
                UiUpdate::IsFindingEarbuds(finding) => {
                    is_finding_earbuds.set(finding);
                    controls_state::update_finding_earbuds(&controls_widgets, finding);
                }
                UiUpdate::FitTestRunning(running) => {
                    fit_test_running.set(running);
                    controls_state::update_fit_test_running(&controls_widgets, running);
                }
                UiUpdate::FitTestResult(result) => {
                    fit_test_running.set(false);
                    controls_state::update_fit_test_running(&controls_widgets, false);
                    controls_state::update_fit_test_result(&controls_widgets, result);
                }
                UiUpdate::PromptVolume(volume) => {
                    session_state_for_updates
                        .borrow_mut()
                        .controls
                        .prompt_volume = Some(volume);
                    current_prompt_volume.set(Some(volume));
                    controls_state::update_prompt_volume(&controls_widgets, volume);
                }
                UiUpdate::HeadMotionType(motion_type) => {
                    session_state_for_updates.borrow_mut().controls.head_motion = Some(motion_type);
                    current_head_motion.set(Some(motion_type));
                    controls_state::update_head_switch(&controls_widgets, motion_type > 0);
                    controls_state::update_head_motion(&controls_widgets, motion_type);
                }
                UiUpdate::DualDeviceConnectionEnabled(enabled) => {
                    session_state_for_updates
                        .borrow_mut()
                        .dual_device
                        .current_dual_device = Some(enabled);
                    current_dual_device.set(Some(enabled));
                    controls_state::update_dual_device(&controls_widgets, enabled);
                }
                UiUpdate::InEarDetectionEnabled(enabled) => {
                    session_state_for_updates
                        .borrow_mut()
                        .controls
                        .in_ear_detection = Some(enabled);
                    current_in_ear.set(Some(enabled));
                    controls_state::update_in_ear_detection(&controls_widgets, enabled);
                }
                UiUpdate::SleepDetectionEnabled(enabled) => {
                    session_state_for_updates
                        .borrow_mut()
                        .controls
                        .sleep_detection = Some(enabled);
                    current_sleep.set(Some(enabled));
                    controls_state::update_sleep_detection(&controls_widgets, enabled);
                }
                UiUpdate::PreferredDevice(address) => {
                    session_state_for_updates
                        .borrow_mut()
                        .controls
                        .preferred_device = address;
                    *current_preferred_device.borrow_mut() = address;
                    controls_state::update_preferred_devices(
                        &controls_widgets,
                        address,
                        &cached_paired_devices.borrow(),
                    );
                }
                UiUpdate::Finished => {
                    connection_state.show_disconnected();
                    widgets.clear_turntable();
                    *current_artwork_path.borrow_mut() = None;
                    widgets.scene_view.clear();
                    widgets
                        .model_picture
                        .set_paintable(None::<&gtk::gdk::Paintable>);
                    widgets.model_stack.set_visible_child_name("picture");
                    widgets.model_placeholder.set_visible(true);
                    widgets.model_firmware.set_visible(false);
                    widgets.model_firmware.set_text("");
                    let was_connected = connected;
                    if connected {
                        device_widgets
                            .connection
                            .set_text(&crate::t!("channel-status-disconnected"));
                        connected = false;
                        *connected_device_for_updates.borrow_mut() = None;
                    }
                    session_state_for_updates.borrow_mut().reset_on_disconnect();
                    sync_desktop_services(
                        &quick_settings,
                        &tray,
                        &session_state_for_updates.borrow(),
                    );
                    widgets
                        .connection_tag
                        .set_text(&crate::t!("common-disconnected"));
                    widgets
                        .connection_tag
                        .remove_css_class("connection-tag-connected");
                    widgets
                        .connection_tag
                        .add_css_class("connection-tag-disconnected");
                    widgets.paired_devices_section.set_visible(true);
                    paired_devices = Err(crate::t!("dual-device-read-after-connecting"));
                    paired_device_controls_enabled = Err(crate::t!("dual-device-reading-status"));
                    paired_action_pending_for_updates.set(false);
                    render_paired_devices(
                        &widgets.paired_devices,
                        paired_devices.clone(),
                        &paired_device_controls_enabled,
                        &paired_action_pending_for_updates,
                        &worker_commands_for_updates,
                        &updates_tx_for_updates,
                    );
                    for indicator in [
                        &widgets.left_battery,
                        &widgets.right_battery,
                        &widgets.case_battery,
                    ] {
                        set_battery(indicator, None, false);
                    }
                    noise_controls_enabled = false;
                    supports_wear_detection = true;
                    update_noise_controls(
                        &widgets,
                        &NoiseControlsViewState {
                            supported_indices: &noise_mode_indices,
                            support_note: &noise_mode_support_note,
                            capability_enabled: false,
                            supports_wear_detection,
                            wear: &wear,
                            current_mode: current_noise_mode,
                            modes: &noise_modes,
                        },
                    );
                    current_eq_preset.set(None);
                    current_spatial_mode.set(None);
                    current_game_mode.set(None);
                    current_eq_presets.borrow_mut().clear();
                    current_supports_eq.set(false);
                    current_supports_spatial.set(false);
                    current_supports_game_mode.set(false);
                    current_supports_bass_engine.set(false);
                    current_supports_hi_res.set(false);
                    current_supports_audio_codec.set(false);
                    current_available_codecs.borrow_mut().clear();
                    current_audio_codec.set(None);
                    current_bass_engine_enabled.set(None);
                    *current_bass_engine_level.borrow_mut() = None;
                    current_hi_res_enabled.set(None);
                    reset_sound_controls(&widgets);
                    sound_state::reset_sound_page(&sound_widgets);
                    if let Some(source_id) = bass_debounce_source.borrow_mut().take() {
                        source_id.remove();
                    }
                    controls_state::reset_controls_page(&controls_widgets);
                    if let Some(source_id) = prompt_volume_debounce_source.borrow_mut().take() {
                        source_id.remove();
                    }
                    selected_ear.set(EarSide::Left);
                    controls_widgets.ear_left_btn.set_active(true);
                    controls_widgets.ear_right_btn.set_active(false);
                    current_key_functions.borrow_mut().clear();
                    current_gesture_rows.borrow_mut().clear();
                    *current_noise_cycle.borrow_mut() = None;
                    is_finding_earbuds.set(false);
                    fit_test_running.set(false);
                    current_prompt_volume.set(None);
                    current_head_motion.set(None);
                    current_dual_device.set(None);
                    current_in_ear.set(None);
                    current_sleep.set(None);
                    *current_preferred_device.borrow_mut() = None;
                    cached_paired_devices.borrow_mut().clear();
                    *worker_commands.borrow_mut() = None;
                    if let Some(device) = pending_connect_for_updates.borrow_mut().take() {
                        widgets
                            .connection_tag
                            .set_text(&crate::t!("common-connecting"));
                        widgets
                            .connection_tag
                            .remove_css_class("connection-tag-connected");
                        widgets
                            .connection_tag
                            .add_css_class("connection-tag-disconnected");
                        start_monitor_for_device(
                            device,
                            updates_tx_for_updates.clone(),
                            Rc::clone(&worker_commands_for_updates),
                        );
                    } else {
                        if was_connected {
                            auto_connect_pending_for_updates.set(true);
                        }
                        if let Some(first_device) = device_choices_for_updates.borrow().first() {
                            widgets.device_name.set_text(&first_device.name);
                        }
                    }
                    widgets
                        .device_menu_button
                        .set_tooltip_text(Some(&crate::t!("device-title")));
                    render_candidate_devices(
                        &CandidateDeviceListWidgets {
                            list: &widgets.device_list,
                            empty_label: &widgets.device_list_empty,
                            popover: &widgets.device_popover,
                            connection_tag: &widgets.connection_tag,
                            device_name: &widgets.device_name,
                        },
                        &device_choices_for_updates.borrow(),
                        None,
                        &CandidateDeviceContext {
                            updates_tx: &updates_tx_for_updates,
                            worker_commands: &worker_commands_for_updates,
                            pending_connect: &pending_connect_for_updates,
                            active_connected_device: &connected_device_for_updates,
                            auto_connect_pending: &auto_connect_pending_for_updates,
                        },
                    );
                }
            }
        }
    });
}

struct CandidateDeviceListWidgets<'a> {
    list: &'a gtk::ListBox,
    empty_label: &'a Label,
    popover: &'a gtk::Popover,
    connection_tag: &'a Label,
    device_name: &'a Label,
}

struct CandidateDeviceContext<'a> {
    updates_tx: &'a mpsc::Sender<UiUpdate>,
    worker_commands: &'a Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
    pending_connect: &'a Rc<RefCell<Option<bluetooth::DeviceChoice>>>,
    active_connected_device: &'a Rc<RefCell<Option<bluetooth::DeviceChoice>>>,
    auto_connect_pending: &'a Rc<Cell<bool>>,
}

fn render_candidate_devices(
    widgets: &CandidateDeviceListWidgets,
    devices: &[bluetooth::DeviceChoice],
    connected_device: Option<&bluetooth::DeviceChoice>,
    ctx: &CandidateDeviceContext,
) {
    while let Some(child) = widgets.list.first_child() {
        widgets.list.remove(&child);
    }

    if devices.is_empty() {
        widgets.empty_label.set_visible(true);
        widgets.list.set_visible(false);
        return;
    }

    widgets.empty_label.set_visible(false);
    widgets.list.set_visible(true);

    for (index, device) in devices.iter().enumerate() {
        let duplicate_name = devices
            .iter()
            .filter(|candidate| candidate.name == device.name)
            .count()
            > 1;
        let display_title = if duplicate_name {
            format!(
                "{} · {}",
                crate::t!("device-numbered", device.brand.label(), index + 1),
                device.name
            )
        } else {
            format!("{} · {}", device.brand.label(), device.name)
        };

        let is_connected = connected_device.is_some_and(|c| c.is_same_device(device));

        let row = adw::ActionRow::builder()
            .title(&display_title)
            .activatable(true)
            .build();

        let icon = gtk::Image::builder()
            .icon_name("audio-headphones-symbolic")
            .build();
        row.add_prefix(&icon);

        if is_connected {
            let check_icon = gtk::Image::builder()
                .icon_name("object-select-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["accent"])
                .build();
            row.add_suffix(&check_icon);
        }

        let dev = device.clone();
        let pop = widgets.popover.clone();
        let tx = ctx.updates_tx.clone();
        let cmds = Rc::clone(ctx.worker_commands);
        let pending = Rc::clone(ctx.pending_connect);
        let tag = widgets.connection_tag.clone();
        let name_lbl = widgets.device_name.clone();
        let active_conn = Rc::clone(ctx.active_connected_device);
        let auto_pending = Rc::clone(ctx.auto_connect_pending);

        row.connect_activated(move |_| {
            pop.popdown();
            auto_pending.set(false);

            if let Some(active) = active_conn.borrow().as_ref() {
                if dev.is_same_device(active) {
                    return;
                }
            }

            name_lbl.set_text(&dev.name);

            if let Some(sender) = cmds.borrow().as_ref() {
                *pending.borrow_mut() = Some(dev.clone());
                tag.set_text(&crate::t!("common-switching"));
                tag.remove_css_class("connection-tag-connected");
                tag.add_css_class("connection-tag-disconnected");
                let _ = tx.send(UiUpdate::Status(crate::t!("device-switching")));
                if let Err(error) = sender.send(WorkerCommand::Disconnect) {
                    pending.borrow_mut().take();
                    let _ = tx.send(UiUpdate::Status(crate::t!(
                        "device-stop-connection-failed",
                        error
                    )));
                }
            } else {
                tag.set_text(&crate::t!("common-connecting"));
                tag.remove_css_class("connection-tag-connected");
                tag.add_css_class("connection-tag-disconnected");
                start_monitor_for_device(dev.clone(), tx.clone(), Rc::clone(&cmds));
            }
        });

        widgets.list.append(&row);
    }
}

fn render_paired_devices(
    container: &gtk::ListBox,
    result: Result<Vec<crate::protocol::PairedDevice>, String>,
    controls_enabled: &Result<bool, String>,
    action_pending: &Rc<Cell<bool>>,
    worker_commands: &Rc<RefCell<Option<std::sync::mpsc::Sender<WorkerCommand>>>>,
    updates: &std::sync::mpsc::Sender<UiUpdate>,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    match result {
        Ok(devices) if devices.is_empty() => {
            let empty_row = adw::ActionRow::builder()
                .title(&crate::t!("dual-device-no-devices-reported"))
                .css_classes(["dim-label"])
                .build();
            container.append(&empty_row);
        }
        Ok(mut devices) => {
            devices.sort_by_key(|d| (!d.connected, !d.is_current_host));
            for device in devices {
                let row = adw::ActionRow::builder()
                    .title(if device.is_current_host {
                        crate::t!("dual-device-this-host-label", &device.name)
                    } else {
                        device.name.clone()
                    })
                    .activatable(true)
                    .build();

                let icon = gtk::Image::builder()
                    .icon_name(paired_device_icon_name(device.device_type))
                    .build();
                row.add_prefix(&icon);

                let status_text = if device.connected {
                    crate::t!("common-connected")
                } else {
                    crate::t!("common-disconnected")
                };
                let status_label = Label::builder()
                    .label(&status_text)
                    .valign(gtk::Align::Center)
                    .css_classes(["dim-label"])
                    .build();
                row.add_suffix(&status_label);

                let chevron = gtk::Image::builder()
                    .icon_name("go-next-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(["dim-label"])
                    .build();
                row.add_suffix(&chevron);

                let controls_enabled = controls_enabled.clone();
                let action_pending = Rc::clone(action_pending);
                let worker_commands = Rc::clone(worker_commands);
                let updates = updates.clone();

                row.connect_activated(move |row| {
                    show_paired_device_dialog(
                        row,
                        &device,
                        &controls_enabled,
                        &action_pending,
                        &worker_commands,
                        &updates,
                    );
                });

                container.append(&row);
            }
        }
        Err(error) => {
            let error_row = adw::ActionRow::builder()
                .title(&error)
                .css_classes(["dim-label"])
                .build();
            container.append(&error_row);
        }
    }
}
