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
    io::{Read, Write},
    sync::mpsc::Sender,
};

use crate::app::events::{SettingDomain, UiUpdate, WorkerCommand};
use crate::bluetooth::brands::oppo::features::{OppoDualDeviceState, OppoFeatureContext};
use crate::bluetooth::brands::oppo::transport::request;
use crate::protocol::{
    FrameDecoder, PairedDevice, PairedDeviceAction, PairedDeviceAddress,
    encode_set_priority_device, parse_feature_flags_response, parse_paired_devices_response,
    parse_priority_device_response,
};

pub fn set_paired_device<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    controls_enabled: &Result<bool, String>,
    paired_devices: &Result<Vec<PairedDevice>, String>,
    address: PairedDeviceAddress,
    action: PairedDeviceAction,
    updates: &Sender<UiUpdate>,
) -> Result<(Vec<PairedDevice>, bool), String> {
    match controls_enabled {
        Ok(true) => {}
        Ok(false) => return Err(crate::t!("dual-device-not-enabled")),
        Err(error) => return Err(crate::t!("dual-device-confirm-status-failed", error)),
    }

    let paired_devices = paired_devices
        .as_ref()
        .map_err(|error| crate::t!("dual-device-read-list-failed", error))?;
    let target = paired_devices
        .iter()
        .find(|device| device.address == address)
        .ok_or_else(|| crate::t!("dual-device-not-in-list"))?;
    if target.is_current_host {
        return Err(match action {
            PairedDeviceAction::Remove => crate::t!("dual-device-this-host-cannot-remove"),
            _ => crate::t!("dual-device-this-host-cannot-disconnect"),
        });
    }
    match action {
        PairedDeviceAction::Connect | PairedDeviceAction::Disconnect => {
            let expected = action
                .expected_connected()
                .expect("Connect/Disconnect must have expected state");
            if target.connected == expected {
                return Ok((paired_devices.clone(), false));
            }

            let updated = issue_paired_device_action(stream, decoder, address, action, updates)?;
            let updated_target = updated
                .iter()
                .find(|device| device.address == address)
                .ok_or_else(|| crate::t!("dual-device-target-missing"))?;
            if updated_target.connected != expected {
                return Err(crate::t!("dual-device-readback-mismatch"));
            }
            Ok((updated, true))
        }
        PairedDeviceAction::Remove => {
            if target.connected {
                return Err(crate::t!("dual-device-cannot-delete-connected"));
            }

            let updated = issue_paired_device_action(stream, decoder, address, action, updates)?;
            if updated.iter().any(|device| device.address == address) {
                return Err(crate::t!("dual-device-deletion-unconfirmed"));
            }
            Ok((updated, true))
        }
    }
}

pub fn issue_paired_device_action<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    address: PairedDeviceAddress,
    action: PairedDeviceAction,
    updates: &Sender<UiUpdate>,
) -> Result<Vec<PairedDevice>, String> {
    request(
        stream,
        decoder,
        0x0429,
        0,
        action.payload(address),
        Some(updates),
    )?;
    read_paired_devices(stream, decoder, Some(updates))
        .map_err(|error| crate::t!("dual-device-readback-list-failed", error))
}

pub fn read_paired_devices<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<PairedDevice>, String> {
    let response = request(stream, decoder, 0x0112, 0, vec![], updates)?;
    parse_paired_devices_response(&response.payload)
        .map_err(|error| crate::t!("dual-device-list-invalid", error))
}

pub fn handle_dual_device_command<T: Read + Write>(
    ctx: &mut OppoFeatureContext<'_, T>,
    command: &WorkerCommand,
    state: &mut OppoDualDeviceState,
) -> bool {
    match command {
        WorkerCommand::SetPairedDevice { address, action } => {
            if ctx.read_only || !ctx.supports_dual_device() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("dual-device-management-blocked"),
                });
                let _ = ctx.updates.send(UiUpdate::PairedDeviceActionFinished);
                return true;
            }
            let result = set_paired_device(
                ctx.stream,
                ctx.decoder,
                &state.controls_enabled,
                &state.paired_devices,
                *address,
                *action,
                ctx.updates,
            );
            match result {
                Ok((devices, changed)) => {
                    state.paired_devices = Ok(devices.clone());
                    let _ = ctx.updates.send(UiUpdate::PairedDevices(Ok(devices)));
                    let message = if changed {
                        match action {
                            PairedDeviceAction::Remove => crate::t!("dual-device-removed"),
                            _ => crate::t!("dual-device-status-confirmed"),
                        }
                    } else {
                        crate::t!("dual-device-already-in-state")
                    };
                    let _ = ctx.updates.send(UiUpdate::Status(message));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: error,
                    });
                    state.paired_devices =
                        read_paired_devices(ctx.stream, ctx.decoder, Some(ctx.updates));
                    let _ = ctx
                        .updates
                        .send(UiUpdate::PairedDevices(state.paired_devices.clone()));
                }
            }
            let _ = ctx.updates.send(UiUpdate::PairedDeviceActionFinished);
            true
        }
        WorkerCommand::SetDualDeviceConnection(enabled) => {
            if ctx.read_only || !ctx.supports_dual_device() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("dual-device-toggle-blocked"),
                });
                return true;
            }
            let payload = vec![17, if *enabled { 1 } else { 0 }];
            match request(
                ctx.stream,
                ctx.decoder,
                0x0403,
                0,
                payload,
                Some(ctx.updates),
            ) {
                Ok(_) => {
                    if let Ok(res) = request(
                        ctx.stream,
                        ctx.decoder,
                        0x010d,
                        0,
                        vec![1, 17],
                        Some(ctx.updates),
                    ) {
                        if let Ok(actual) = parse_feature_flags_response(&res.payload, 17) {
                            state.dual_device_enabled = Some(actual);
                            state.controls_enabled = Ok(actual);
                            let _ = ctx
                                .updates
                                .send(UiUpdate::DualDeviceConnectionEnabled(actual));
                            let _ = ctx
                                .updates
                                .send(UiUpdate::PairedDeviceControlsEnabled(Ok(actual)));
                        }
                    }
                    let _ = ctx
                        .updates
                        .send(UiUpdate::Status(crate::t!("dual-device-toggle-updated")));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("dual-device-toggle-failed", error),
                    });
                    if let Some(d) = state.dual_device_enabled {
                        let _ = ctx.updates.send(UiUpdate::DualDeviceConnectionEnabled(d));
                    }
                }
            }
            true
        }
        WorkerCommand::SetPreferredDevice(addr) => {
            if ctx.read_only || !ctx.supports_dual_device() {
                let _ = ctx.updates.send(UiUpdate::SettingError {
                    domain: SettingDomain::Controls,
                    message: crate::t!("controls-preferred-device-blocked"),
                });
                return true;
            }
            let payload = encode_set_priority_device(*addr);
            match request(
                ctx.stream,
                ctx.decoder,
                0x0429,
                0,
                payload,
                Some(ctx.updates),
            ) {
                Ok(_) => {
                    if let Ok(res) = request(
                        ctx.stream,
                        ctx.decoder,
                        0x0132,
                        0,
                        vec![2],
                        Some(ctx.updates),
                    ) {
                        if let Ok(actual) = parse_priority_device_response(&res.payload) {
                            state.preferred_device = actual;
                            let _ = ctx.updates.send(UiUpdate::PreferredDevice(actual));
                        }
                    }
                    let _ = ctx.updates.send(UiUpdate::Status(crate::t!(
                        "controls-preferred-device-updated"
                    )));
                }
                Err(error) => {
                    let _ = ctx.updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-preferred-device-update-failed", error),
                    });
                    let _ = ctx
                        .updates
                        .send(UiUpdate::PreferredDevice(state.preferred_device));
                }
            }
            true
        }
        _ => false,
    }
}
