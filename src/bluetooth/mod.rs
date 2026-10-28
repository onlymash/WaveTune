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

use bluer::Address;

pub mod brands;
pub(super) mod discovery;
mod probe;
#[cfg(test)]
mod tests;
pub(super) mod transport;

pub use crate::app::events::{UiUpdate, WorkerCommand};
#[allow(unused_imports)]
pub use brands::oppo as session;
#[allow(unused_imports)]
pub use brands::oppo::{
    DeviceSnapshot, OppoDeviceSnapshot, initialize_oppo_session, initialize_session, spawn_monitor,
    spawn_oppo_monitor,
};
pub use discovery::spawn_discovery;
pub use probe::probe;

#[derive(Clone)]
pub struct DeviceChoice {
    address: Address,
    pub name: String,
    pub brand: DeviceBrand,
    /// Whether BlueZ reports the headset control service UUID for this device.
    pub has_control_service: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceBrand {
    OnePlus,
    Oppo,
    Realme,
}

impl DeviceBrand {
    pub fn label(self) -> &'static str {
        match self {
            Self::OnePlus => "OnePlus",
            Self::Oppo => "OPPO",
            Self::Realme => "realme",
        }
    }
}

impl std::fmt::Debug for DeviceChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeviceChoice")
            .field("name", &self.name)
            .field("brand", &self.brand)
            .field("has_control_service", &self.has_control_service)
            .finish_non_exhaustive()
    }
}

impl DeviceChoice {
    pub fn is_same_device(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
