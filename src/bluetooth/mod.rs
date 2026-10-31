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
pub mod driver;
mod probe;
pub mod snapshot;
#[cfg(test)]
mod tests;
pub(super) mod transport;
pub mod worker;

pub use crate::app::events::{UiUpdate, WorkerCommand};
#[allow(unused_imports)]
pub use brands::oppo as session;
#[allow(unused_imports)]
pub use brands::oppo::{initialize_oppo_session, initialize_session, spawn_oppo_monitor};
pub use discovery::spawn_discovery;
#[allow(unused_imports)]
pub use driver::HeadsetDriver;
pub use probe::probe;
pub use snapshot::*;
#[allow(unused_imports)]
pub use worker::SessionRunner;

/// Unified driver enum dispatched across supported headset brands.
pub enum AnyDriver {
    EarFun(brands::earfun::EarFunDriver),
    Oppo(brands::oppo::OppoDriver),
}

impl HeadsetDriver for AnyDriver {
    fn brand_name(&self) -> &'static str {
        match self {
            AnyDriver::EarFun(d) => d.brand_name(),
            AnyDriver::Oppo(d) => d.brand_name(),
        }
    }

    fn open_rfcomm(&mut self, device: &DeviceChoice) -> std::io::Result<std::fs::File> {
        match self {
            AnyDriver::EarFun(d) => d.open_rfcomm(device),
            AnyDriver::Oppo(d) => d.open_rfcomm(device),
        }
    }

    fn initialize<T: std::io::Read + std::io::Write>(
        &mut self,
        stream: &mut T,
        device: &DeviceChoice,
        updates: Option<&std::sync::mpsc::Sender<UiUpdate>>,
    ) -> Result<DeviceSnapshot, String> {
        match self {
            AnyDriver::EarFun(d) => d.initialize(stream, device, updates),
            AnyDriver::Oppo(d) => d.initialize(stream, device, updates),
        }
    }

    fn handle_command<T: std::io::Read + std::io::Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &std::sync::mpsc::Sender<UiUpdate>,
    ) -> Result<(), String> {
        match self {
            AnyDriver::EarFun(d) => d.handle_command(stream, command, updates),
            AnyDriver::Oppo(d) => d.handle_command(stream, command, updates),
        }
    }

    fn handle_incoming_bytes<T: std::io::Read + std::io::Write>(
        &mut self,
        stream: &mut T,
        bytes: &[u8],
        updates: &std::sync::mpsc::Sender<UiUpdate>,
    ) -> Result<(), String> {
        match self {
            AnyDriver::EarFun(d) => d.handle_incoming_bytes(stream, bytes, updates),
            AnyDriver::Oppo(d) => d.handle_incoming_bytes(stream, bytes, updates),
        }
    }

    fn poll_interval(&self) -> Option<std::time::Duration> {
        match self {
            AnyDriver::EarFun(d) => d.poll_interval(),
            AnyDriver::Oppo(d) => d.poll_interval(),
        }
    }

    fn periodic_tick<T: std::io::Read + std::io::Write>(
        &mut self,
        stream: &mut T,
        updates: &std::sync::mpsc::Sender<UiUpdate>,
    ) -> Result<(), String> {
        match self {
            AnyDriver::EarFun(d) => d.periodic_tick(stream, updates),
            AnyDriver::Oppo(d) => d.periodic_tick(stream, updates),
        }
    }
}

/// Factory function to create brand-specific headset drivers.
pub fn create_driver(brand: DeviceBrand) -> AnyDriver {
    match brand {
        DeviceBrand::EarFun => AnyDriver::EarFun(brands::earfun::EarFunDriver::new()),
        DeviceBrand::OnePlus | DeviceBrand::Oppo | DeviceBrand::Realme => {
            AnyDriver::Oppo(brands::oppo::OppoDriver::new())
        }
    }
}

pub fn spawn_monitor(
    device: DeviceChoice,
    updates: std::sync::mpsc::Sender<UiUpdate>,
    commands: std::sync::mpsc::Receiver<WorkerCommand>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let driver = create_driver(device.brand);
    SessionRunner::spawn(device, driver, updates, commands)
}

#[derive(Clone)]
pub struct DeviceChoice {
    pub(crate) address: Address,
    pub name: String,
    pub brand: DeviceBrand,
    pub product_id: Option<u32>,
    /// Whether BlueZ reports the headset control service UUID for this device.
    pub has_control_service: bool,
    /// Control service UUIDs reported by BlueZ for this device, ordered by catalog priority.
    pub(crate) control_service_uuids: Vec<bluer::Uuid>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceBrand {
    OnePlus,
    Oppo,
    Realme,
    EarFun,
}

impl DeviceBrand {
    pub fn label(self) -> &'static str {
        match self {
            Self::OnePlus => "OnePlus",
            Self::Oppo => "OPPO",
            Self::Realme => "realme",
            Self::EarFun => "EarFun",
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
