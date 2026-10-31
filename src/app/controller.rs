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

use std::{cell::RefCell, rc::Rc, sync::mpsc};

pub(crate) use super::desktop::DesktopServices;
use super::{
    events::UiUpdate,
    features::{
        FeatureContext,
        controls::{ControlsComponent, ControlsController},
        device::{DeviceComponent, DeviceController},
        noise::{NoiseComponent, NoiseController},
        sound::{SoundComponent, SoundController},
    },
    state::HeadsetSessionState,
};
use crate::ui::{
    ConnectionStateWidgets, ControlsWidgets, DeviceWidgets, HomeWidgets, SoundWidgets,
};

pub(crate) struct AppUiWidgets {
    pub home: HomeWidgets,
    pub sound: SoundWidgets,
    pub controls: ControlsWidgets,
    pub device: DeviceWidgets,
    pub connection_state: ConnectionStateWidgets,
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
                home,
                sound,
                controls,
                device,
                connection_state,
            },
        services,
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
    let worker_commands = Rc::new(RefCell::new(None));
    let context = FeatureContext::new(
        Rc::clone(&worker_commands),
        updates_tx,
        Rc::clone(&session_state),
    );

    let cached_paired_devices = Rc::new(RefCell::new(Vec::new()));

    let noise_component = NoiseComponent::new(home.noise);
    let noise_controller = NoiseController::new(noise_component, context.clone());

    let sound_component = SoundComponent::new(home.sound, sound);
    let sound_controller = SoundController::new(sound_component, context.clone());

    let controls_component = ControlsComponent::new(controls);
    let controls_controller = ControlsController::new(
        controls_component,
        context.clone(),
        Rc::clone(&cached_paired_devices),
    );

    let device_component = DeviceComponent::new(
        home.header,
        home.battery,
        home.hero,
        home.paired,
        device,
        connection_state,
    );
    let device_controller =
        DeviceController::new(device_component, context.clone(), cached_paired_devices);

    // Wire desktop services (quick settings & tray)
    {
        let noise_for_desktop = noise_controller.clone();
        services.wire_handlers(
            Rc::clone(&session_state),
            move |mode_idx| noise_for_desktop.send_mode(mode_idx),
            present_window,
            quit_app,
        );
    }

    // Bind event handlers for all vertical slice controllers
    noise_controller.bind_events();
    sound_controller.bind_events();
    controls_controller.bind_events();
    device_controller.bind_events();

    // Event loop for UI updates
    gtk::glib::spawn_future_local(async move {
        while let Some(update) = async_rx.recv().await {
            if let UiUpdate::InitialSession(snapshot) = &update {
                session_state.borrow_mut().apply_initial_snapshot(snapshot);
            }

            let _ = device_controller.on_update(&update);
            let _ = noise_controller.on_update(&update);
            let _ = sound_controller.on_update(&update);
            let _ = controls_controller.on_update(&update);

            match &update {
                UiUpdate::InitialSession(..) => {
                    noise_controller.render(&session_state.borrow());
                }
                UiUpdate::Connected { .. } | UiUpdate::Finished => {
                    noise_controller.render(&session_state.borrow());
                    sound_controller.reset();
                    controls_controller.reset();
                }
                UiUpdate::Wear(..) | UiUpdate::ProductId { .. } => {
                    noise_controller.render(&session_state.borrow());
                }
                _ => {}
            }

            services.sync(&session_state.borrow());
        }
    });
}
