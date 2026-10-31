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
use gtk::{Align, Box as GtkBox, Button, Label, Orientation};

use crate::{bluetooth, ui::components::configure_label_wrapping};

#[derive(Clone)]
pub(crate) struct DeviceHeaderWidgets {
    device_name: Label,
    connection_tag: Label,
    device_menu_button: gtk::MenuButton,
    device_popover: gtk::Popover,
    device_list: gtk::ListBox,
    device_list_empty: Label,
    refresh_devices: Button,
}

impl DeviceHeaderWidgets {
    pub fn set_device_name(&self, name: &str) {
        self.device_name.set_text(name);
    }

    pub fn set_tag_connected(&self) {
        self.connection_tag.set_text(&crate::t!("common-connected"));
        self.connection_tag
            .remove_css_class("connection-tag-disconnected");
        self.connection_tag
            .add_css_class("connection-tag-connected");
    }

    pub fn set_tag_connecting(&self) {
        self.connection_tag
            .set_text(&crate::t!("common-connecting"));
        self.connection_tag
            .remove_css_class("connection-tag-connected");
        self.connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_switching(&self) {
        self.connection_tag.set_text(&crate::t!("common-switching"));
        self.connection_tag
            .remove_css_class("connection-tag-connected");
        self.connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_disconnected(&self) {
        self.connection_tag
            .set_text(&crate::t!("common-disconnected"));
        self.connection_tag
            .remove_css_class("connection-tag-connected");
        self.connection_tag
            .add_css_class("connection-tag-disconnected");
    }

    pub fn set_tag_rebooting(&self) {
        self.connection_tag.set_text(&crate::t!("device-rebooting"));
    }

    pub fn set_menu_tooltip(&self, text: &str) {
        self.device_menu_button.set_tooltip_text(Some(text));
    }

    pub fn set_refresh_sensitive(&self, sensitive: bool) {
        self.refresh_devices.set_sensitive(sensitive);
    }

    pub fn menu_button(&self) -> &gtk::MenuButton {
        &self.device_menu_button
    }

    pub fn refresh_button(&self) -> &Button {
        &self.refresh_devices
    }

    pub fn connect_refresh_clicked<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.refresh_devices.connect_clicked(move |_| {
            callback();
        });
    }

    pub fn render_candidate_devices(
        &self,
        devices: &[bluetooth::DeviceChoice],
        connected_device: Option<&bluetooth::DeviceChoice>,
        on_select: impl Fn(bluetooth::DeviceChoice) + Clone + 'static,
    ) {
        while let Some(child) = self.device_list.first_child() {
            self.device_list.remove(&child);
        }

        if devices.is_empty() {
            self.device_list_empty.set_visible(true);
            self.device_list.set_visible(false);
            return;
        }

        self.device_list_empty.set_visible(false);
        self.device_list.set_visible(true);

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
            let pop = self.device_popover.clone();
            let on_select = on_select.clone();

            row.connect_activated(move |_| {
                pop.popdown();
                on_select(dev.clone());
            });

            self.device_list.append(&row);
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl DeviceHeaderWidgets {
    pub fn popover(&self) -> &gtk::Popover {
        &self.device_popover
    }

    pub fn reset(&self) {
        self.set_device_name(&crate::t!("device-no-supported-connected"));
        self.set_tag_disconnected();
        self.set_menu_tooltip(&crate::t!("device-title"));
    }
}

pub(crate) fn build_device_header(battery_box: &GtkBox) -> (GtkBox, DeviceHeaderWidgets) {
    let refresh_devices = Button::from_icon_name("view-refresh-symbolic");
    refresh_devices.set_tooltip_text(Some(&crate::t!("device-refresh-list")));
    refresh_devices.set_valign(Align::Center);
    refresh_devices.add_css_class("flat");

    let device_popover = gtk::Popover::new();
    let popover_box = GtkBox::new(Orientation::Vertical, 8);
    popover_box.set_margin_top(8);
    popover_box.set_margin_bottom(8);
    popover_box.set_margin_start(8);
    popover_box.set_margin_end(8);
    popover_box.set_size_request(280, -1);

    let popover_header = GtkBox::new(Orientation::Horizontal, 8);
    let popover_title = Label::builder()
        .label(&crate::t!("device-title"))
        .css_classes(["heading"])
        .halign(Align::Start)
        .hexpand(true)
        .build();
    popover_header.append(&popover_title);
    popover_header.append(&refresh_devices);
    popover_box.append(&popover_header);

    let device_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    popover_box.append(&device_list);

    let device_list_empty = Label::builder()
        .label(&crate::t!("device-no-supported-connected"))
        .css_classes(["dim-label"])
        .wrap(true)
        .xalign(0.5)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    popover_box.append(&device_list_empty);

    device_popover.set_child(Some(&popover_box));

    let device_menu_button = gtk::MenuButton::builder()
        .icon_name("audio-headphones-symbolic")
        .tooltip_text(&crate::t!("device-title"))
        .popover(&device_popover)
        .valign(Align::Center)
        .build();

    let device_row = GtkBox::new(Orientation::Horizontal, 16);
    device_row.set_hexpand(true);
    device_row.set_halign(Align::Fill);
    device_row.set_valign(Align::Center);
    device_row.set_margin_top(12);
    device_row.set_margin_bottom(12);
    device_row.set_margin_start(16);
    device_row.set_margin_end(16);

    let device_info = GtkBox::new(Orientation::Vertical, 6);
    device_info.set_valign(Align::Center);
    device_info.set_hexpand(true);

    let device_name = Label::builder()
        .label(&crate::t!("device-no-supported-connected"))
        .xalign(0.0)
        .css_classes(["device-name"])
        .build();
    configure_label_wrapping(&device_name);

    let connection_tag = Label::builder()
        .label(&crate::t!("common-disconnected"))
        .css_classes(["connection-tag", "connection-tag-disconnected"])
        .halign(Align::Start)
        .valign(Align::Center)
        .build();

    device_info.append(&device_name);
    device_info.append(&connection_tag);
    device_info.append(battery_box);
    device_row.append(&device_info);

    let widgets = DeviceHeaderWidgets {
        device_name,
        connection_tag,
        device_menu_button,
        device_popover,
        device_list,
        device_list_empty,
        refresh_devices,
    };

    (device_row, widgets)
}
