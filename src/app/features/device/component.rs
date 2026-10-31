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
    path::Path,
    rc::Rc,
    sync::mpsc,
};

use adw::prelude::*;

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth,
    protocol::{BatteryState, FirmwareVersions, PairedDevice, WearState},
    ui::{
        ConnectionStateWidgets, DeviceWidgets,
        dialogs::{paired_device_icon_name, show_paired_device_dialog},
        home_state::wear_part_name,
        pages::home::{
            BatteryBannerWidgets, DeviceHeaderWidgets, DeviceModelHeroWidgets,
            PairedDevicesCardWidgets,
        },
    },
};

#[derive(Clone)]
pub(crate) struct DeviceComponent {
    pub header: DeviceHeaderWidgets,
    pub battery: BatteryBannerWidgets,
    pub hero: DeviceModelHeroWidgets,
    pub paired: PairedDevicesCardWidgets,
    pub device: DeviceWidgets,
    pub connection_state: ConnectionStateWidgets,
}

impl DeviceComponent {
    pub fn new(
        header: DeviceHeaderWidgets,
        battery: BatteryBannerWidgets,
        hero: DeviceModelHeroWidgets,
        paired: PairedDevicesCardWidgets,
        device: DeviceWidgets,
        connection_state: ConnectionStateWidgets,
    ) -> Self {
        Self {
            header,
            battery,
            hero,
            paired,
            device,
            connection_state,
        }
    }

    pub fn set_device_name(&self, name: &str) {
        self.header.set_device_name(name);
    }

    pub fn set_tag_connected(&self) {
        self.header.set_tag_connected();
    }

    pub fn set_tag_connecting(&self) {
        self.header.set_tag_connecting();
    }

    pub fn set_tag_switching(&self) {
        self.header.set_tag_switching();
    }

    pub fn set_tag_disconnected(&self) {
        self.header.set_tag_disconnected();
    }

    pub fn set_tag_rebooting(&self) {
        self.header.set_tag_rebooting();
    }

    pub fn render_connected(&self, device_name: &str) {
        self.connection_state.show_content();
        self.hero.reset();
        self.set_device_name(device_name);
        self.set_tag_connected();
        self.header.set_menu_tooltip(device_name);
        self.device
            .connection
            .set_text(&crate::t!("channel-status-connected"));
        self.device
            .name
            .set_text(&crate::t!("device-name", device_name));
        self.device.wear.set_text(&crate::t!("wear-status-waiting"));
        self.reset_batteries();
    }

    pub fn render_disconnected(&self) {
        self.connection_state.show_disconnected();
        self.hero.reset();
        self.set_tag_disconnected();
        self.paired.set_visible(true);
        self.header.set_menu_tooltip(&crate::t!("device-title"));
        self.reset_batteries();
    }

    pub fn render_battery(&self, battery: &BatteryState, case_battery_is_stale: bool) {
        self.battery.render_battery(battery, case_battery_is_stale);
    }

    pub fn reset_batteries(&self) {
        self.battery.reset();
    }

    pub fn render_wear(&self, wear: &WearState, supports_wear_detection: bool) {
        if !supports_wear_detection {
            self.device
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
                .unwrap_or_else(|| crate::t!("wear-status-unknown", crate::t!("wear-part-case")));
            let left_status = wear_part_name(&crate::t!("wear-part-left"), wear.left);
            let right_status = wear_part_name(&crate::t!("wear-part-right"), wear.right);
            self.device
                .wear
                .set_text(&format!("{case_status} · {left_status} · {right_status}"));
        }
    }

    pub fn render_firmware(&self, versions: &FirmwareVersions) {
        let fw_text =
            if versions.left.is_some() || versions.right.is_some() || versions.case.is_some() {
                let mut parts = Vec::new();
                if let Some(ref left) = versions.left {
                    parts.push(format!("L: {left}"));
                }
                if let Some(ref right) = versions.right {
                    parts.push(format!("R: {right}"));
                }
                if let Some(ref case) = versions.case {
                    parts.push(format!("Case: {case}"));
                }
                format!("{} ({})", crate::t!("device-firmware"), parts.join(" · "))
            } else {
                crate::t!("device-firmware-unknown")
            };
        self.device.firmware.set_text(&fw_text);
        self.hero.set_firmware(&fw_text);
    }

    pub fn render_artwork(&self, path: &Path) {
        if !self.hero.has_turntable() && !self.hero.has_scene() {
            let display_path = if crate::resources::is_generic_artwork(path) {
                let is_dark = adw::StyleManager::default().is_dark();
                crate::resources::generic_artwork_path(is_dark)
            } else {
                path.to_path_buf()
            };
            self.hero.set_picture_filename(Some(&display_path));
            self.hero.show_picture();
        }
    }

    pub fn render_model(&self, path: &Path) {
        self.hero.clear_scene();
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(webp) = crate::resources::turntable::AnimatedWebp::parse(bytes) {
                let turntable = crate::resources::turntable::TurntableModel::new(webp);
                self.hero.set_turntable(turntable);
            }
        }
    }

    pub fn render_scene(&self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            match crate::resources::vfxms::VfxScene::parse(&bytes) {
                Ok(scene) => match self.hero.load_scene(&scene) {
                    Ok(()) => {
                        self.hero.clear_turntable();
                        self.hero.show_scene3d();
                        self.hero.set_placeholder_visible(false);
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

    pub fn render_candidate_devices(
        &self,
        devices: &[bluetooth::DeviceChoice],
        connected_device: Option<&bluetooth::DeviceChoice>,
        on_select: impl Fn(bluetooth::DeviceChoice) + Clone + 'static,
    ) {
        self.header
            .render_candidate_devices(devices, connected_device, on_select);
    }

    pub fn render_paired_devices(
        &self,
        result: &Result<Vec<PairedDevice>, String>,
        controls_enabled: &Rc<RefCell<Result<bool, String>>>,
        action_pending: &Rc<Cell<bool>>,
        worker_commands: &Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
        updates: &mpsc::Sender<UiUpdate>,
    ) {
        self.paired.clear_items();

        match result {
            Ok(devices) if devices.is_empty() => {
                let empty_row = adw::ActionRow::builder()
                    .title(&crate::t!("dual-device-no-devices-reported"))
                    .css_classes(["dim-label"])
                    .build();
                self.paired.append_row(&empty_row);
            }
            Ok(devices) => {
                let mut devices = devices.clone();
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
                    let status_label = gtk::Label::builder()
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

                    let controls_enabled = Rc::clone(controls_enabled);
                    let action_pending = Rc::clone(action_pending);
                    let worker_commands = Rc::clone(worker_commands);
                    let updates = updates.clone();

                    row.connect_activated(move |row| {
                        show_paired_device_dialog(
                            row,
                            &device,
                            &controls_enabled.borrow(),
                            &action_pending,
                            &worker_commands,
                            &updates,
                        );
                    });

                    self.paired.append_row(&row);
                }
            }
            Err(error) => {
                let error_row = adw::ActionRow::builder()
                    .title(error)
                    .css_classes(["dim-label"])
                    .build();
                self.paired.append_row(&error_row);
            }
        }
    }
}
