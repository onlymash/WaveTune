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
    rc::Rc,
    sync::mpsc::Sender,
};

use adw::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    protocol::{PairedDevice, PairedDeviceAction},
};

pub fn paired_device_icon_name(device_type: u8) -> &'static str {
    match device_type {
        1 => "phone-symbolic",
        2 => "computer-symbolic",
        3 => "tablet-symbolic",
        _ => "bluetooth-active-symbolic",
    }
}

pub fn paired_device_type_name(device_type: u8) -> String {
    match device_type {
        1 => crate::t!("device-type-phone"),
        2 => crate::t!("device-type-computer"),
        3 => crate::t!("device-type-tablet"),
        _ => crate::t!("device-type-other"),
    }
}

pub fn show_paired_device_dialog(
    parent: &impl IsA<gtk::Widget>,
    device: &PairedDevice,
    controls_enabled: &Result<bool, String>,
    action_pending: &Rc<Cell<bool>>,
    worker_commands: &Rc<RefCell<Option<Sender<WorkerCommand>>>>,
    updates: &Sender<UiUpdate>,
) {
    let dialog = adw::Dialog::builder()
        .content_width(400)
        .title(&device.name)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    let header_bar = adw::HeaderBar::builder().show_title(false).build();
    toolbar_view.add_top_bar(&header_bar);

    let page = adw::PreferencesPage::new();

    let header_group = adw::PreferencesGroup::new();
    let header_box = GtkBox::new(Orientation::Vertical, 12);
    header_box.set_halign(gtk::Align::Center);
    header_box.set_margin_top(12);
    header_box.set_margin_bottom(12);

    let icon = gtk::Image::builder()
        .icon_name(paired_device_icon_name(device.device_type))
        .pixel_size(72)
        .css_classes(["icon-dropshadow"])
        .build();

    let name_label = Label::builder()
        .label(&device.name)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .justify(gtk::Justification::Center)
        .css_classes(["title-1"])
        .build();

    header_box.append(&icon);
    header_box.append(&name_label);

    if device.is_current_host {
        let host_badge = Label::builder()
            .label(&crate::t!("dual-device-this-host"))
            .css_classes(["dim-label"])
            .build();
        header_box.append(&host_badge);
    }

    header_group.add(&header_box);
    page.add(&header_group);

    let props_group = adw::PreferencesGroup::new();

    let type_row = adw::ActionRow::builder()
        .title(&crate::t!("device-type"))
        .subtitle(&paired_device_type_name(device.device_type))
        .css_classes(["property"])
        .build();
    props_group.add(&type_row);

    let mac_row = adw::ActionRow::builder()
        .title(&crate::t!("device-mac-address"))
        .subtitle(&device.address.format_mac())
        .subtitle_selectable(true)
        .css_classes(["property"])
        .build();
    props_group.add(&mac_row);

    let conn_status_text = if device.connected {
        crate::t!("common-connected")
    } else {
        crate::t!("common-disconnected")
    };
    let conn_row = adw::ActionRow::builder()
        .title(&crate::t!("channel-bluetooth-connection"))
        .subtitle(&conn_status_text)
        .css_classes(["property"])
        .build();
    props_group.add(&conn_row);

    page.add(&props_group);

    if device.is_current_host {
        let host_info_group = adw::PreferencesGroup::new();
        let host_row = adw::ActionRow::builder()
            .title(&crate::t!("dual-device-this-host"))
            .subtitle(&crate::t!("dual-device-this-host-cannot-disconnect"))
            .build();
        let host_icon = gtk::Image::builder()
            .icon_name("dialog-information-symbolic")
            .build();
        host_row.add_prefix(&host_icon);
        host_info_group.add(&host_row);
        page.add(&host_info_group);
    } else {
        let actions_group = adw::PreferencesGroup::builder().separate_rows(true).build();

        let can_control = matches!(controls_enabled, Ok(true));

        if !can_control {
            if let Err(error) = controls_enabled {
                actions_group.set_description(Some(error));
            } else {
                actions_group.set_description(Some(&crate::t!("dual-device-please-enable")));
            }
        } else if device.connected {
            actions_group.set_description(Some(&crate::t!("dual-device-cannot-delete-connected")));
        }

        let connect_title = if device.connected {
            crate::t!("dual-device-disconnect")
        } else {
            crate::t!("dual-device-connect")
        };
        let connect_row = adw::ButtonRow::builder().title(&connect_title).build();

        let can_toggle_connect = can_control && !action_pending.get();
        connect_row.set_sensitive(can_toggle_connect);

        if let Err(error) = controls_enabled {
            connect_row.set_tooltip_text(Some(&crate::t!("dual-device-state-unavailable", error)));
        } else if !can_control {
            connect_row.set_tooltip_text(Some(&crate::t!("dual-device-please-enable")));
        }

        if can_toggle_connect {
            let address = device.address;
            let action = if device.connected {
                PairedDeviceAction::Disconnect
            } else {
                PairedDeviceAction::Connect
            };
            let action_pending = Rc::clone(action_pending);
            let worker_commands = Rc::clone(worker_commands);
            let updates = updates.clone();
            let dialog_weak = dialog.downgrade();

            connect_row.connect_activated(move |row| {
                if action_pending.replace(true) {
                    return;
                }
                row.set_sensitive(false);
                let command_result = worker_commands
                    .borrow()
                    .as_ref()
                    .map(|sender| sender.send(WorkerCommand::SetPairedDevice { address, action }));
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(_)) => {
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "dual-device-send-connect-failed"
                        )));
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                    None => {
                        let _ = updates.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                }
                if let Some(dialog) = dialog_weak.upgrade() {
                    dialog.close();
                }
            });
        }
        actions_group.add(&connect_row);

        let delete_row = adw::ButtonRow::builder()
            .title(&crate::t!("dual-device-forget"))
            .build();
        delete_row.add_css_class("destructive-action");

        let can_delete = can_control && !action_pending.get() && !device.connected;
        delete_row.set_sensitive(can_delete);

        if device.connected {
            delete_row.set_tooltip_text(Some(&crate::t!("dual-device-cannot-delete-connected")));
        } else if let Err(error) = controls_enabled {
            delete_row.set_tooltip_text(Some(&crate::t!("dual-device-state-unavailable", error)));
        } else if !can_control {
            delete_row.set_tooltip_text(Some(&crate::t!("dual-device-please-enable")));
        } else {
            delete_row.set_tooltip_text(Some(&crate::t!("dual-device-remove-tooltip")));
        }

        if can_delete {
            let address = device.address;
            let action_pending = Rc::clone(action_pending);
            let worker_commands = Rc::clone(worker_commands);
            let updates = updates.clone();
            let dialog_weak = dialog.downgrade();

            delete_row.connect_activated(move |row| {
                if action_pending.replace(true) {
                    return;
                }
                row.set_sensitive(false);
                let command_result = worker_commands.borrow().as_ref().map(|sender| {
                    sender.send(WorkerCommand::SetPairedDevice {
                        address,
                        action: PairedDeviceAction::Remove,
                    })
                });
                match command_result {
                    Some(Ok(())) => {}
                    Some(Err(_)) => {
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "dual-device-send-delete-failed"
                        )));
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                    None => {
                        let _ = updates.send(UiUpdate::Status(crate::t!("channel-not-connected")));
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                }
                if let Some(dialog) = dialog_weak.upgrade() {
                    dialog.close();
                }
            });
        }
        actions_group.add(&delete_row);

        page.add(&actions_group);
    }
    toolbar_view.set_content(Some(&page));
    dialog.set_child(Some(&toolbar_view));
    dialog.present(Some(parent));
}
