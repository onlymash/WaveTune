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
    time::Duration,
};

use adw::prelude::*;

use super::component::DeviceComponent;
use crate::{
    app::{
        device_manager::{start_device_discovery, start_monitor_for_device},
        events::{UiUpdate, WorkerCommand},
        features::FeatureContext,
    },
    bluetooth,
    protocol::PairedDevice,
};

#[derive(Clone)]
pub(crate) struct DeviceController {
    component: DeviceComponent,
    context: FeatureContext,
    device_choices: Rc<RefCell<Vec<bluetooth::DeviceChoice>>>,
    connected_device: Rc<RefCell<Option<bluetooth::DeviceChoice>>>,
    pending_connect: Rc<RefCell<Option<bluetooth::DeviceChoice>>>,
    paired_action_pending: Rc<Cell<bool>>,
    scanning: Rc<Cell<bool>>,
    auto_connect_pending: Rc<Cell<bool>>,
    current_artwork_path: Rc<RefCell<Option<PathBuf>>>,
    cached_paired_devices: Rc<RefCell<Vec<PairedDevice>>>,
    paired_devices: Rc<RefCell<Result<Vec<PairedDevice>, String>>>,
    paired_device_controls_enabled: Rc<RefCell<Result<bool, String>>>,
}

impl DeviceController {
    pub fn new(
        component: DeviceComponent,
        context: FeatureContext,
        cached_paired_devices: Rc<RefCell<Vec<PairedDevice>>>,
    ) -> Self {
        Self {
            component,
            context,
            device_choices: Rc::new(RefCell::new(Vec::new())),
            connected_device: Rc::new(RefCell::new(None)),
            pending_connect: Rc::new(RefCell::new(None)),
            paired_action_pending: Rc::new(Cell::new(false)),
            scanning: Rc::new(Cell::new(false)),
            auto_connect_pending: Rc::new(Cell::new(true)),
            current_artwork_path: Rc::new(RefCell::new(None)),
            cached_paired_devices,
            paired_devices: Rc::new(RefCell::new(Err(crate::t!(
                "dual-device-read-after-connecting"
            )))),
            paired_device_controls_enabled: Rc::new(RefCell::new(Err(crate::t!(
                "dual-device-reading-status"
            )))),
        }
    }

    pub fn bind_events(&self) {
        let refresh_button = self.component.header.refresh_button().clone();
        let updates_tx = self.context.updates_tx.clone();
        let scanning = Rc::clone(&self.scanning);
        self.component.header.connect_refresh_clicked(move || {
            start_device_discovery(
                updates_tx.clone(),
                refresh_button.clone(),
                Rc::clone(&scanning),
                true,
            );
        });

        let component = self.component.clone();
        let current_artwork_path = Rc::clone(&self.current_artwork_path);
        adw::StyleManager::default().connect_dark_notify(move |manager| {
            if !component.hero.has_turntable() && !component.hero.has_scene() {
                if let Some(path) = current_artwork_path.borrow().as_deref() {
                    if crate::resources::is_generic_artwork(path) {
                        let new_path = crate::resources::generic_artwork_path(manager.is_dark());
                        component.hero.set_picture_filename(Some(&new_path));
                    }
                }
            }
        });

        start_device_discovery(
            self.context.updates_tx.clone(),
            self.component.header.refresh_button().clone(),
            Rc::clone(&self.scanning),
            false,
        );

        {
            let updates_tx = self.context.updates_tx.clone();
            let refresh_button = self.component.header.refresh_button().clone();
            let scanning = Rc::clone(&self.scanning);
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
    }

    pub fn connect_to_device(&self, dev: bluetooth::DeviceChoice) {
        self.auto_connect_pending.set(false);
        if let Some(active) = self.connected_device.borrow().as_ref() {
            if dev.is_same_device(active) {
                return;
            }
        }
        self.component.set_device_name(&dev.name);

        if let Some(sender) = self.context.worker_commands.borrow().as_ref() {
            *self.pending_connect.borrow_mut() = Some(dev.clone());
            self.component.set_tag_switching();
            self.context.send_status(crate::t!("device-switching"));
            if let Err(error) = sender.send(WorkerCommand::Disconnect) {
                self.pending_connect.borrow_mut().take();
                self.context
                    .send_status(crate::t!("device-stop-connection-failed", error));
            }
        } else {
            self.component.set_tag_connecting();
            start_monitor_for_device(
                dev,
                self.context.updates_tx.clone(),
                Rc::clone(&self.context.worker_commands),
            );
        }
    }

    pub fn render_candidates(&self) {
        let this = self.clone();
        self.component.render_candidate_devices(
            &self.device_choices.borrow(),
            self.connected_device.borrow().as_ref(),
            move |dev| {
                this.connect_to_device(dev);
            },
        );
    }

    pub fn render_paired(&self) {
        self.component.render_paired_devices(
            &self.paired_devices.borrow(),
            &self.paired_device_controls_enabled,
            &self.paired_action_pending,
            &self.context.worker_commands,
            &self.context.updates_tx,
        );
    }

    pub fn on_update(&self, update: &UiUpdate) -> bool {
        match update {
            UiUpdate::Status(message) => {
                self.component
                    .device
                    .connection
                    .set_text(&crate::t!("common-status", message));
                true
            }
            UiUpdate::SettingError { message, .. } => {
                self.component
                    .device
                    .connection
                    .set_text(&crate::t!("common-error-status", message));
                true
            }
            UiUpdate::Devices(result) => {
                self.scanning.set(false);
                self.component.header.set_refresh_sensitive(true);
                self.component
                    .header
                    .refresh_button()
                    .set_tooltip_text(Some(&crate::t!("device-refresh-list")));
                match result {
                    Ok(devices) => {
                        let mut devices = devices.clone();
                        if let Some(active_device) = self.connected_device.borrow().as_ref()
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
                        *self.device_choices.borrow_mut() = devices.clone();

                        self.render_candidates();

                        if self.connected_device.borrow().is_none() {
                            if let Some(first_device) = devices.first() {
                                self.component.set_device_name(&first_device.name);
                            } else {
                                self.component
                                    .set_device_name(&crate::t!("device-no-supported-connected"));
                            }
                        }
                        if devices.is_empty() && !self.context.session_state.borrow().connected {
                            self.component
                                .device
                                .connection
                                .set_text(&crate::t!("device-status-no-connected"));
                        }
                        let idle = self.context.worker_commands.borrow().is_none()
                            && self.pending_connect.borrow().is_none();
                        let auto_connect_target = if self.auto_connect_pending.get() && idle {
                            devices
                                .iter()
                                .find(|device| device.has_control_service)
                                .cloned()
                        } else {
                            None
                        };
                        if let Some(target) = auto_connect_target {
                            self.auto_connect_pending.set(false);
                            self.component.set_device_name(&target.name);
                            self.component.set_tag_connecting();
                            start_monitor_for_device(
                                target,
                                self.context.updates_tx.clone(),
                                Rc::clone(&self.context.worker_commands),
                            );
                        }
                    }
                    Err(error) => {
                        if !self.context.session_state.borrow().connected
                            && self.context.worker_commands.borrow().is_none()
                        {
                            self.component
                                .device
                                .connection
                                .set_text(&crate::t!("device-status-discovery-failed", error));
                        }
                    }
                }
                true
            }
            UiUpdate::Connecting { device } => {
                if !self.context.session_state.borrow().connected {
                    self.component
                        .connection_state
                        .show_connecting(&device.name);
                }
                true
            }
            UiUpdate::Connected { device } => {
                *self.connected_device.borrow_mut() = Some(device.clone());
                *self.current_artwork_path.borrow_mut() = None;
                self.component.render_connected(&device.name);
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.reset_on_disconnect();
                    st.connected = true;
                    st.device_name = device.name.clone();
                    st.supports_wear_detection = true;
                }
                *self.paired_devices.borrow_mut() = Err(crate::t!("dual-device-reading-list"));
                *self.paired_device_controls_enabled.borrow_mut() =
                    Err(crate::t!("dual-device-reading-status"));
                self.paired_action_pending.set(false);
                self.render_paired();
                self.render_candidates();
                true
            }
            UiUpdate::Battery(update) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    if update.case.is_some() {
                        st.case_battery_is_stale = false;
                    } else if st.battery.case.is_some() {
                        st.case_battery_is_stale = true;
                    }
                    crate::ui::home_state::merge_battery(&mut st.battery, update.clone());
                }
                let st = self.context.session_state.borrow();
                self.component
                    .render_battery(&st.battery, st.case_battery_is_stale);
                true
            }
            UiUpdate::Wear(update) => {
                let supports_wear = self.context.session_state.borrow().supports_wear_detection;
                {
                    let mut st = self.context.session_state.borrow_mut();
                    crate::ui::home_state::merge_wear(&mut st.wear, update.clone());
                }
                let st = self.context.session_state.borrow();
                self.component.render_wear(&st.wear, supports_wear);
                true
            }
            UiUpdate::ProductId {
                product_id,
                color_id,
            } => {
                let profile = crate::compatibility::profile_for_product_id(*product_id);
                let supports_wear = profile.is_none_or(|p| p.supports_wear_detection());
                let supports_dual = profile.is_some_and(|p| p.supports_dual_device());

                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.product_id = Some(*product_id);
                    st.supports_wear_detection = supports_wear;
                }
                if !supports_wear {
                    self.component
                        .device
                        .wear
                        .set_text(&crate::t!("wear-status-not-supported"));
                }
                self.component.paired.set_visible(supports_dual);
                self.component.device.product_id.set_text(&crate::t!(
                    "device-product-id",
                    format!("{:06X}", product_id)
                ));
                crate::resources::spawn_artwork_fetch(
                    *product_id,
                    *color_id,
                    self.context.updates_tx.clone(),
                );
                true
            }
            UiUpdate::DeviceArtwork(path) => {
                self.context.session_state.borrow_mut().artwork_path = Some(path.clone());
                *self.current_artwork_path.borrow_mut() = Some(path.clone());
                self.component.render_artwork(path);
                true
            }
            UiUpdate::DeviceModel(path) => {
                self.component.render_model(path);
                true
            }
            UiUpdate::DeviceScene(path) => {
                self.component.render_scene(path);
                true
            }
            UiUpdate::Firmware(firmware) => {
                self.component.render_firmware(firmware);
                true
            }
            UiUpdate::PairedDevices(result) => {
                let mut sorted = result.clone();
                if let Ok(devs) = &mut sorted {
                    devs.sort_by_key(|d| (!d.connected, !d.is_current_host));
                    *self.cached_paired_devices.borrow_mut() = devs.clone();
                }
                self.context
                    .session_state
                    .borrow_mut()
                    .dual_device
                    .paired_devices = sorted.clone();
                *self.paired_devices.borrow_mut() = sorted;
                self.render_paired();
                true
            }
            UiUpdate::PairedDeviceControlsEnabled(result) => {
                if let Err(error) = result {
                    self.context
                        .send_status(crate::t!("dual-device-confirm-status-failed", error));
                }
                self.context
                    .session_state
                    .borrow_mut()
                    .dual_device
                    .controls_enabled = result.clone();
                *self.paired_device_controls_enabled.borrow_mut() = result.clone();
                self.render_paired();
                true
            }
            UiUpdate::DualDeviceConnectionEnabled(enabled) => {
                let status = Ok(*enabled);
                self.context
                    .session_state
                    .borrow_mut()
                    .dual_device
                    .controls_enabled = status.clone();
                *self.paired_device_controls_enabled.borrow_mut() = status;
                self.render_paired();
                true
            }
            UiUpdate::PairedDeviceActionFinished => {
                self.paired_action_pending.set(false);
                self.render_paired();
                true
            }
            UiUpdate::DeviceRebooting => {
                self.component.set_tag_rebooting();
                true
            }
            UiUpdate::Finished => {
                let was_connected = self.context.session_state.borrow().connected;
                if was_connected {
                    self.component
                        .device
                        .connection
                        .set_text(&crate::t!("channel-status-disconnected"));
                    *self.connected_device.borrow_mut() = None;
                }
                *self.current_artwork_path.borrow_mut() = None;
                self.component.render_disconnected();
                self.context
                    .session_state
                    .borrow_mut()
                    .reset_on_disconnect();
                *self.paired_devices.borrow_mut() =
                    Err(crate::t!("dual-device-read-after-connecting"));
                *self.paired_device_controls_enabled.borrow_mut() =
                    Err(crate::t!("dual-device-reading-status"));
                self.paired_action_pending.set(false);
                self.render_paired();
                self.cached_paired_devices.borrow_mut().clear();
                *self.context.worker_commands.borrow_mut() = None;

                if let Some(device) = self.pending_connect.borrow_mut().take() {
                    self.component.set_tag_connecting();
                    start_monitor_for_device(
                        device,
                        self.context.updates_tx.clone(),
                        Rc::clone(&self.context.worker_commands),
                    );
                } else {
                    if was_connected {
                        self.auto_connect_pending.set(true);
                    }
                    if let Some(first_device) = self.device_choices.borrow().first() {
                        self.component.set_device_name(&first_device.name);
                    }
                }
                self.render_candidates();
                true
            }
            _ => false,
        }
    }
}
