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

use std::{cell::Cell, cell::RefCell, path::Path, rc::Rc, sync::mpsc};

use adw::prelude::*;

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth,
    protocol::{BatteryState, FirmwareVersions, PairedDevice, WearState},
    ui::{
        ConnectionStateWidgets, DeviceWidgets, HomeWidgetRefs,
        dialogs::{paired_device_icon_name, show_paired_device_dialog},
        home_state::{set_battery, wear_part_name},
    },
};

#[derive(Clone)]
pub(crate) struct DeviceComponent {
    pub home: HomeWidgetRefs,
    pub device: DeviceWidgets,
    pub connection_state: ConnectionStateWidgets,
}

impl DeviceComponent {
    pub fn new(
        home: HomeWidgetRefs,
        device: DeviceWidgets,
        connection_state: ConnectionStateWidgets,
    ) -> Self {
        Self {
            home,
            device,
            connection_state,
        }
    }

    pub fn set_device_name(&self, name: &str) {
        self.home.device_name.set_text(name);
    }

    pub fn set_tag_connected(&self) {
        self.home
            .connection_tag
            .set_text(&crate::t!("common-connected"));
        self.home
            .connection_tag
            .remove_css_class("connection-tag-disconnected");
        self.home
            .connection_tag
            .add_css_class("connection-tag-connected");
    }

    pub fn set_tag_connecting(&self) {
        self.home
            .connection_tag
            .set_text(&crate::t!("common-connecting"));
        self.home
            .connection_tag
            .remove_css_class("connection-tag-connected");
        self.home
            .connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_switching(&self) {
        self.home
            .connection_tag
            .set_text(&crate::t!("common-switching"));
        self.home
            .connection_tag
            .remove_css_class("connection-tag-connected");
        self.home
            .connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_disconnected(&self) {
        self.home
            .connection_tag
            .set_text(&crate::t!("common-disconnected"));
        self.home
            .connection_tag
            .remove_css_class("connection-tag-connected");
        self.home
            .connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_rebooting(&self) {
        self.home
            .connection_tag
            .set_text(&crate::t!("device-rebooting"));
    }

    pub fn render_connected(&self, device_name: &str) {
        self.connection_state.show_content();
        self.home.clear_turntable();
        self.home.scene_view.clear();
        self.home.model_stack.set_visible_child_name("picture");
        self.home
            .model_picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
        self.set_device_name(device_name);
        self.set_tag_connected();
        self.home
            .device_menu_button
            .set_tooltip_text(Some(device_name));
        self.device
            .connection
            .set_text(&crate::t!("channel-status-connected"));
        self.device
            .name
            .set_text(&crate::t!("device-name", device_name));
        self.device.wear.set_text(&crate::t!("wear-status-waiting"));
        self.reset_batteries();
        self.home.model_firmware.set_visible(false);
        self.home.model_firmware.set_text("");
    }

    pub fn render_disconnected(&self) {
        self.connection_state.show_disconnected();
        self.home.clear_turntable();
        self.home.scene_view.clear();
        self.home
            .model_picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
        self.home.model_stack.set_visible_child_name("picture");
        self.home.model_placeholder.set_visible(true);
        self.home.model_firmware.set_visible(false);
        self.home.model_firmware.set_text("");
        self.set_tag_disconnected();
        self.home.paired_devices_section.set_visible(true);
        self.home
            .device_menu_button
            .set_tooltip_text(Some(&crate::t!("device-title")));
        self.reset_batteries();
    }

    pub fn render_battery(&self, battery: &BatteryState, case_battery_is_stale: bool) {
        set_battery(&self.home.left_battery, battery.left, false);
        set_battery(&self.home.right_battery, battery.right, false);
        set_battery(&self.home.case_battery, battery.case, case_battery_is_stale);
    }

    pub fn reset_batteries(&self) {
        for indicator in [
            &self.home.left_battery,
            &self.home.right_battery,
            &self.home.case_battery,
        ] {
            set_battery(indicator, None, false);
        }
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
                .unwrap_or_else(|| crate::t!("wear-case-status-unknown"));
            let wear_left = wear_part_name(&crate::t!("battery-left-earbud"), wear.left);
            let wear_right = wear_part_name(&crate::t!("battery-right-earbud"), wear.right);
            self.device.wear.set_text(&crate::t!(
                "wear-status",
                format!("{wear_left}; {wear_right}; {case_status}")
            ));
        }
    }

    pub fn render_firmware(&self, firmware: &FirmwareVersions) {
        let not_reported = crate::t!("common-not-reported");
        let fw_text = match (&firmware.left, &firmware.right, &firmware.case) {
            (Some(l), Some(r), Some(c)) => crate::t!("firmware-version-full", l, r, c),
            (Some(l), Some(r), None) => crate::t!("firmware-version-left-right", l, r),
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
        self.device.firmware.set_text(&fw_text);
        self.home.model_firmware.set_text(&fw_text);
        self.home.model_firmware.set_visible(true);
    }

    pub fn render_artwork(&self, path: &Path) {
        if self.home.active_turntable.borrow().is_none() && !self.home.scene_view.has_scene() {
            let display_path = if crate::resources::is_generic_artwork(path) {
                let is_dark = adw::StyleManager::default().is_dark();
                crate::resources::generic_artwork_path(is_dark)
            } else {
                path.to_path_buf()
            };
            self.home.model_picture.set_filename(Some(&display_path));
            self.home.model_stack.set_visible_child_name("picture");
        }
    }

    pub fn render_model(&self, path: &Path) {
        self.home.scene_view.clear();
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(webp) = crate::resources::turntable::AnimatedWebp::parse(bytes) {
                let turntable = crate::resources::turntable::TurntableModel::new(webp);
                self.home.set_turntable(turntable);
            }
        }
    }

    pub fn render_scene(&self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            match crate::resources::vfxms::VfxScene::parse(&bytes) {
                Ok(scene) => match self.home.scene_view.load_scene(&scene) {
                    Ok(()) => {
                        self.home.clear_turntable();
                        self.home.model_stack.set_visible_child_name("scene3d");
                        self.home.model_placeholder.set_visible(false);
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
        while let Some(child) = self.home.device_list.first_child() {
            self.home.device_list.remove(&child);
        }

        if devices.is_empty() {
            self.home.device_list_empty.set_visible(true);
            self.home.device_list.set_visible(false);
            return;
        }

        self.home.device_list_empty.set_visible(false);
        self.home.device_list.set_visible(true);

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
            let pop = self.home.device_popover.clone();
            let on_select = on_select.clone();

            row.connect_activated(move |_| {
                pop.popdown();
                on_select(dev.clone());
            });

            self.home.device_list.append(&row);
        }
    }

    pub fn render_paired_devices(
        &self,
        result: &Result<Vec<PairedDevice>, String>,
        controls_enabled: &Result<bool, String>,
        action_pending: &Rc<Cell<bool>>,
        worker_commands: &Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
        updates: &mpsc::Sender<UiUpdate>,
    ) {
        while let Some(child) = self.home.paired_devices.first_child() {
            self.home.paired_devices.remove(&child);
        }

        match result {
            Ok(devices) if devices.is_empty() => {
                let empty_row = adw::ActionRow::builder()
                    .title(&crate::t!("dual-device-no-devices-reported"))
                    .css_classes(["dim-label"])
                    .build();
                self.home.paired_devices.append(&empty_row);
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

                    self.home.paired_devices.append(&row);
                }
            }
            Err(error) => {
                let error_row = adw::ActionRow::builder()
                    .title(error)
                    .css_classes(["dim-label"])
                    .build();
                self.home.paired_devices.append(&error_row);
            }
        }
    }
}
