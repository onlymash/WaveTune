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

//! Abstract headset driver interface.
//!
//! Each supported headset brand (OPPO family, EarFun, future brands like Huawei, Xiaomi)
//! implements `HeadsetDriver` to provide brand-specific handshake, capability detection,
//! command execution, and unsolicited packet handling.

use std::{
    fs::File,
    io::{Read, Write},
    sync::mpsc::Sender,
    time::Duration,
};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, DeviceSnapshot},
};

/// Common driver trait for brand-specific Bluetooth headset protocols.
pub trait HeadsetDriver: Send + 'static {
    /// Returns the brand name or identifier for thread naming and logging.
    fn brand_name(&self) -> &'static str;

    /// Opens the Linux RFCOMM connection for the given device.
    fn open_rfcomm(&mut self, device: &DeviceChoice) -> std::io::Result<File>;

    /// Executes initial handshake, model identification, and capability detection.
    ///
    /// Returns the initial vertical-slice domain snapshot.
    fn initialize<T: Read + Write>(
        &mut self,
        stream: &mut T,
        device: &DeviceChoice,
        updates: Option<&Sender<UiUpdate>>,
    ) -> Result<DeviceSnapshot, String>;

    /// Handles a user-initiated setting write command from the UI.
    fn handle_command<T: Read + Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String>;

    /// Handles incoming bytes read from the RFCOMM socket stream during idle polling.
    fn handle_incoming_bytes<T: Read + Write>(
        &mut self,
        stream: &mut T,
        bytes: &[u8],
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String>;

    /// Optional periodic background polling interval (e.g. for polling battery if events are unsupported).
    fn poll_interval(&self) -> Option<Duration> {
        None
    }

    /// Periodic background tick called when `poll_interval` elapses.
    fn periodic_tick<T: Read + Write>(
        &mut self,
        _stream: &mut T,
        _updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        Ok(())
    }
}

