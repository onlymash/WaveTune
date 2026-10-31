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

//! Generic session runner for Bluetooth headset monitor threads.
//!
//! Standardizes thread lifecycle, RFCOMM connection management, atomic initial
//! state delivery, UI command dispatching, socket polling, and cleanup across
//! all headset brands.

use std::{
    io::{Read, Write},
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, driver::HeadsetDriver, transport::is_socket_timeout},
};

pub struct SessionRunner;

impl SessionRunner {
    /// Spawns a background thread running the session worker with the given driver.
    pub fn spawn<D: HeadsetDriver>(
        device: DeviceChoice,
        driver: D,
        updates: Sender<UiUpdate>,
        commands: Receiver<WorkerCommand>,
    ) -> std::io::Result<thread::JoinHandle<()>> {
        let brand_name = driver.brand_name();
        thread::Builder::new()
            .name(format!("wavetune-session-{brand_name}"))
            .spawn(move || {
                let mut driver = driver;
                let result = Self::run(&device, &mut driver, &updates, &commands);
                if let Err(error) = result {
                    let _ = updates.send(UiUpdate::Status(error));
                }
                let _ = updates.send(UiUpdate::Finished);
            })
    }

    /// Connects to the device's RFCOMM socket and runs the session loop.
    pub fn run<D: HeadsetDriver>(
        device: &DeviceChoice,
        driver: &mut D,
        updates: &Sender<UiUpdate>,
        commands: &Receiver<WorkerCommand>,
    ) -> Result<(), String> {
        let mut stream = driver
            .open_rfcomm(device)
            .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
        Self::run_with_stream(&mut stream, device, driver, updates, commands)
    }

    /// Runs the session loop over any `Read + Write` stream.
    ///
    /// This separation allows unit testing the full session lifecycle using mock streams.
    pub fn run_with_stream<T: Read + Write, D: HeadsetDriver>(
        stream: &mut T,
        device: &DeviceChoice,
        driver: &mut D,
        updates: &Sender<UiUpdate>,
        commands: &Receiver<WorkerCommand>,
    ) -> Result<(), String> {
        // 1. Initial handshake & capability probing
        let snapshot = driver.initialize(stream, device, Some(updates))?;

        // 2. Initial state delivery to UI
        let _ = updates.send(UiUpdate::Connected {
            device: device.clone(),
        });
        let _ = updates.send(UiUpdate::InitialSession(Box::new(snapshot.clone())));

        let list_separator = || crate::t!("common-list-separator");
        let clause_separator = || crate::t!("common-clause-separator");

        let status_message = if snapshot.identity.read_only {
            let detected = if snapshot.diagnostics.detected_features.is_empty() {
                crate::t!("probe-no-queries-confirmed")
            } else {
                crate::t!(
                    "probe-detected-read-only",
                    snapshot
                        .diagnostics
                        .detected_features
                        .join(&list_separator())
                )
            };
            if snapshot.diagnostics.probe_errors.is_empty() {
                crate::t!("channel-connected-read-only", detected)
            } else {
                crate::t!(
                    "channel-connected-read-only-with-errors",
                    detected,
                    snapshot.diagnostics.probe_errors.join(&clause_separator())
                )
            }
        } else {
            crate::t!("channel-connected")
        };
        let _ = updates.send(UiUpdate::Status(status_message));

        let poll_interval = driver.poll_interval();
        let mut last_poll = Instant::now();

        // 3. Main event loop
        loop {
            // Check for commands from the UI
            match commands.try_recv() {
                Ok(WorkerCommand::Disconnect) => {
                    let _ = updates.send(UiUpdate::Status(crate::t!("channel-disconnected")));
                    return Ok(());
                }
                Ok(command) => {
                    if let Err(error) = driver.handle_command(stream, command, updates) {
                        let _ = updates.send(UiUpdate::Status(error));
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Ok(());
                }
            }

            // Periodic background tick
            if let Some(interval) = poll_interval {
                if last_poll.elapsed() >= interval {
                    last_poll = Instant::now();
                    let _ = driver.periodic_tick(stream, updates);
                }
            }

            // Read incoming bytes from the socket
            let mut buffer = [0u8; 1024];
            match stream.read(&mut buffer) {
                Ok(0) => {
                    return Err(crate::t!("channel-headset-closed"));
                }
                Ok(n) => {
                    if let Err(error) = driver.handle_incoming_bytes(stream, &buffer[..n], updates)
                    {
                        let _ = updates.send(UiUpdate::Status(error));
                    }
                }
                Err(e) if is_socket_timeout(&e) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(crate::t!("protocol-response-read-failed", e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bluetooth::{DeviceBrand, DeviceSnapshot, snapshot::DeviceIdentitySnapshot};
    use bluer::Address;
    use std::{
        collections::VecDeque,
        io::{Cursor, ErrorKind},
        sync::mpsc,
    };

    struct MockStream {
        read_data: VecDeque<u8>,
        written_data: Vec<u8>,
    }

    impl MockStream {
        fn new(initial_read: Vec<u8>) -> Self {
            Self {
                read_data: VecDeque::from(initial_read),
                written_data: Vec::new(),
            }
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.read_data.is_empty() {
                return Err(std::io::Error::from(ErrorKind::WouldBlock));
            }
            let mut count = 0;
            for slot in buf.iter_mut() {
                if let Some(byte) = self.read_data.pop_front() {
                    *slot = byte;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(count)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written_data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct MockDriver {
        brand: &'static str,
        read_only: bool,
        commands_received: Vec<WorkerCommand>,
        bytes_received: Vec<Vec<u8>>,
        tick_count: usize,
        poll_interval: Option<Duration>,
    }

    impl MockDriver {
        fn new(brand: &'static str, read_only: bool) -> Self {
            Self {
                brand,
                read_only,
                commands_received: Vec::new(),
                bytes_received: Vec::new(),
                tick_count: 0,
                poll_interval: None,
            }
        }
    }

    impl HeadsetDriver for MockDriver {
        fn brand_name(&self) -> &'static str {
            self.brand
        }

        fn open_rfcomm(&mut self, _device: &DeviceChoice) -> std::io::Result<std::fs::File> {
            Err(std::io::Error::other("mock open_rfcomm"))
        }

        fn initialize<T: Read + Write>(
            &mut self,
            _stream: &mut T,
            device: &DeviceChoice,
            _updates: Option<&Sender<UiUpdate>>,
        ) -> Result<DeviceSnapshot, String> {
            let mut snapshot = DeviceSnapshot::default();
            snapshot.identity = DeviceIdentitySnapshot {
                name: device.name.clone(),
                product_id: 0x123456,
                color_id: None,
                model: "Mock Headset".to_string(),
                is_known_model: true,
                read_only: self.read_only,
                firmware: Default::default(),
            };
            if self.read_only {
                snapshot
                    .diagnostics
                    .detected_features
                    .push("battery-status".to_string());
            }
            Ok(snapshot)
        }

        fn handle_command<T: Read + Write>(
            &mut self,
            _stream: &mut T,
            command: WorkerCommand,
            _updates: &Sender<UiUpdate>,
        ) -> Result<(), String> {
            self.commands_received.push(command);
            Ok(())
        }

        fn handle_incoming_bytes<T: Read + Write>(
            &mut self,
            _stream: &mut T,
            bytes: &[u8],
            _updates: &Sender<UiUpdate>,
        ) -> Result<(), String> {
            self.bytes_received.push(bytes.to_vec());
            Ok(())
        }

        fn poll_interval(&self) -> Option<Duration> {
            self.poll_interval
        }

        fn periodic_tick<T: Read + Write>(
            &mut self,
            _stream: &mut T,
            _updates: &Sender<UiUpdate>,
        ) -> Result<(), String> {
            self.tick_count += 1;
            Ok(())
        }
    }

    fn mock_device() -> DeviceChoice {
        DeviceChoice {
            name: "Test Buds Pro".to_string(),
            address: Address::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]),
            brand: DeviceBrand::Oppo,
            product_id: None,
            has_control_service: true,
            control_service_uuids: vec![],
        }
    }

    #[test]
    fn test_session_runner_lifecycle_initialization_and_disconnect() {
        let (updates_tx, updates_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let mut stream = MockStream::new(vec![]);
        let mut driver = MockDriver::new("oppo", false);
        let device = mock_device();

        // Queue a Disconnect command so the runner immediately exits cleanly after init
        commands_tx.send(WorkerCommand::Disconnect).unwrap();

        let result = SessionRunner::run_with_stream(
            &mut stream,
            &device,
            &mut driver,
            &updates_tx,
            &commands_rx,
        );
        assert!(result.is_ok());

        // Verify updates received
        let mut events = Vec::new();
        while let Ok(evt) = updates_rx.try_recv() {
            events.push(evt);
        }

        // Expected: Connected, InitialSession, Status("channel-connected"), Status("channel-disconnected")
        assert!(events.len() >= 4);
        assert!(matches!(events[0], UiUpdate::Connected { .. }));
        assert!(matches!(events[1], UiUpdate::InitialSession(..)));
        assert!(matches!(events[2], UiUpdate::Status(..)));
        assert!(matches!(events[3], UiUpdate::Status(..)));
    }

    #[test]
    fn test_session_runner_read_only_status() {
        let (updates_tx, updates_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let mut stream = MockStream::new(vec![]);
        let mut driver = MockDriver::new("oppo", true);
        let device = mock_device();

        commands_tx.send(WorkerCommand::Disconnect).unwrap();

        let result = SessionRunner::run_with_stream(
            &mut stream,
            &device,
            &mut driver,
            &updates_tx,
            &commands_rx,
        );
        assert!(result.is_ok());

        let mut status_strings = Vec::new();
        while let Ok(evt) = updates_rx.try_recv() {
            if let UiUpdate::Status(msg) = evt {
                status_strings.push(msg);
            }
        }
        // Should contain read-only indication
        assert!(!status_strings.is_empty());
    }

    #[test]
    fn test_session_runner_dispatches_command_and_handles_bytes() {
        let (updates_tx, _updates_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let mut stream = MockStream::new(vec![0xAA, 0xBB, 0xCC]);
        let mut driver = MockDriver::new("earfun", false);
        let device = mock_device();

        // Send a setting command followed by disconnect
        commands_tx.send(WorkerCommand::SetGameMode(true)).unwrap();
        commands_tx.send(WorkerCommand::Disconnect).unwrap();

        let result = SessionRunner::run_with_stream(
            &mut stream,
            &device,
            &mut driver,
            &updates_tx,
            &commands_rx,
        );
        assert!(result.is_ok());

        // Verify driver received the command
        assert_eq!(driver.commands_received.len(), 1);
        assert!(matches!(
            driver.commands_received[0],
            WorkerCommand::SetGameMode(true)
        ));

        // Verify driver received the bytes from the stream
        assert_eq!(driver.bytes_received.len(), 1);
        assert_eq!(driver.bytes_received[0], vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_session_runner_detects_headset_closed() {
        let (updates_tx, _updates_rx) = mpsc::channel();
        let (_commands_tx, commands_rx) = mpsc::channel();
        // A cursor with 0 bytes will return Ok(0) on read
        let mut stream = Cursor::new(Vec::new());
        let mut driver = MockDriver::new("oppo", false);
        let device = mock_device();

        let result = SessionRunner::run_with_stream(
            &mut stream,
            &device,
            &mut driver,
            &updates_tx,
            &commands_rx,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::t!("channel-headset-closed"));
    }
}
