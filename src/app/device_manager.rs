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

use std::{cell::Cell, cell::RefCell, rc::Rc, sync::mpsc};

use gtk::Button;
use gtk::prelude::*;

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth,
};

pub(super) fn start_device_discovery(
    updates: mpsc::Sender<UiUpdate>,
    refresh_button: Button,
    scanning: Rc<Cell<bool>>,
    announce: bool,
) {
    if scanning.replace(true) {
        return;
    }
    refresh_button.set_sensitive(false);
    refresh_button.set_tooltip_text(Some(&crate::t!("device-refreshing-list")));
    if announce {
        let _ = updates.send(UiUpdate::Status(crate::t!("device-refreshing-list")));
    }
    if let Err(error) = bluetooth::spawn_discovery(updates.clone()) {
        scanning.set(false);
        refresh_button.set_sensitive(true);
        refresh_button.set_tooltip_text(Some(&crate::t!("device-refresh-list")));
        let _ = updates.send(UiUpdate::Status(crate::t!(
            "device-start-discovery-failed",
            error
        )));
    }
}

pub(super) fn start_monitor_for_device(
    device: bluetooth::DeviceChoice,
    updates: mpsc::Sender<UiUpdate>,
    worker_commands: Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
) {
    let _ = updates.send(UiUpdate::Status(crate::t!(
        "channel-connecting-to",
        device.name
    )));
    let _ = updates.send(UiUpdate::Connecting {
        device: device.clone(),
    });
    let (commands_tx, commands_rx) = mpsc::channel();
    *worker_commands.borrow_mut() = Some(commands_tx);
    if let Err(error) = bluetooth::spawn_monitor(device, updates.clone(), commands_rx) {
        *worker_commands.borrow_mut() = None;
        let _ = updates.send(UiUpdate::Status(crate::t!(
            "channel-start-task-failed",
            error
        )));
        let _ = updates.send(UiUpdate::Finished);
    }
}
