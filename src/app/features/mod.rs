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

pub(crate) mod controls;
pub(crate) mod device;
pub(crate) mod noise;
pub(crate) mod sound;

use std::{cell::RefCell, rc::Rc, sync::mpsc};

use super::{
    events::{SettingDomain, UiUpdate, WorkerCommand},
    state::HeadsetSessionState,
};

/// Shared execution context passed to feature controllers.
#[derive(Clone)]
pub(crate) struct FeatureContext {
    pub worker_commands: Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
    pub updates_tx: mpsc::Sender<UiUpdate>,
    pub session_state: Rc<RefCell<HeadsetSessionState>>,
}

impl FeatureContext {
    pub fn new(
        worker_commands: Rc<RefCell<Option<mpsc::Sender<WorkerCommand>>>>,
        updates_tx: mpsc::Sender<UiUpdate>,
        session_state: Rc<RefCell<HeadsetSessionState>>,
    ) -> Self {
        Self {
            worker_commands,
            updates_tx,
            session_state,
        }
    }

    /// Dispatches a command to the background Bluetooth worker.
    pub fn send_command(&self, cmd: WorkerCommand) -> Result<(), String> {
        match self.worker_commands.borrow().as_ref() {
            Some(sender) => sender.send(cmd).map_err(|e| e.to_string()),
            None => Err(crate::t!("channel-not-connected")),
        }
    }

    /// Emits a status update to the UI.
    pub fn send_status(&self, message: String) {
        let _ = self.updates_tx.send(UiUpdate::Status(message));
    }

    /// Emits a setting failure error to the UI.
    pub fn send_setting_error(&self, domain: SettingDomain, message: String) {
        let _ = self
            .updates_tx
            .send(UiUpdate::SettingError { domain, message });
    }

    /// Emits an arbitrary UI update.
    #[allow(dead_code)]
    pub fn send_update(&self, update: UiUpdate) {
        let _ = self.updates_tx.send(update);
    }
}
