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

//! OPPO/OnePlus/realme device identity, battery, firmware, and paired devices codecs.

use super::oppo_types::OppoProtocolError;
use crate::protocol::core::types::{
    BatteryPart, BatteryState, FirmwareVersions, PairedDevice, PairedDeviceAddress,
};

pub fn parse_oppo_color_response(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

pub use parse_oppo_color_response as parse_color_response;

pub fn parse_oppo_battery_response(payload: &[u8]) -> Result<BatteryState, OppoProtocolError> {
    if payload.first() != Some(&0) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_battery_components(&payload[1..])
}

pub use parse_oppo_battery_response as parse_battery_response;

pub fn parse_oppo_battery_event(payload: &[u8]) -> Result<BatteryState, OppoProtocolError> {
    if payload.first() != Some(&1) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_battery_components(&payload[1..])
}

pub use parse_oppo_battery_event as parse_battery_event;

fn parse_battery_components(payload: &[u8]) -> Result<BatteryState, OppoProtocolError> {
    let Some((&count, records)) = payload.split_first() else {
        return Err(OppoProtocolError::InvalidPacketLength);
    };
    if records.len() != count as usize * 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }

    let mut state = BatteryState::default();
    for record in records.chunks_exact(2) {
        let part = BatteryPart {
            percentage: record[1] & 0x7f,
            charging: record[1] & 0x80 != 0,
        };
        if part.percentage > 100 {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        match record[0] {
            1 => state.left = Some(part),
            2 => state.right = Some(part),
            // Many OPPO/realme firmwares send [0x03, 0x00] when case battery cannot be read
            // (e.g., both earbuds removed from case or case lacks fuel-gauge telemetry).
            // A depleted case on the charger sets bit 7 (0x80) and is still preserved.
            3 if record[1] != 0 => state.case = Some(part),
            3 => {}
            _ => {}
        }
    }
    Ok(state)
}

pub fn parse_oppo_firmware_response(payload: &[u8]) -> Result<FirmwareVersions, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let count = payload[1] as usize;
    let csv =
        std::str::from_utf8(&payload[2..]).map_err(|_| OppoProtocolError::InvalidPacketLength)?;
    let fields: Vec<&str> = csv.split(',').collect();
    if fields.len() != count * 3 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }

    let mut versions = FirmwareVersions::default();
    for record in fields.chunks_exact(3) {
        let component = record[0]
            .parse::<u8>()
            .map_err(|_| OppoProtocolError::InvalidPacketLength)?;
        let _version_type = record[1]
            .parse::<u8>()
            .map_err(|_| OppoProtocolError::InvalidPacketLength)?;
        if record[2].is_empty() || record[2].len() > 32 {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        match component {
            1 => versions.left = Some(record[2].to_string()),
            2 => versions.right = Some(record[2].to_string()),
            3 => versions.case = Some(record[2].to_string()),
            _ => {}
        }
    }
    Ok(versions)
}

pub use parse_oppo_firmware_response as parse_firmware_response;

pub fn parse_oppo_paired_devices_response(
    payload: &[u8],
) -> Result<Vec<PairedDevice>, OppoProtocolError> {
    if payload.first() != Some(&0) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_paired_devices(&payload[1..])
}

pub use parse_oppo_paired_devices_response as parse_paired_devices_response;

pub fn parse_oppo_paired_devices_event(
    payload: &[u8],
) -> Result<Vec<PairedDevice>, OppoProtocolError> {
    if payload.first() != Some(&6) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_paired_devices(&payload[1..])
}

pub use parse_oppo_paired_devices_event as parse_paired_devices_event;

fn parse_paired_devices(payload: &[u8]) -> Result<Vec<PairedDevice>, OppoProtocolError> {
    let Some(&count) = payload.first() else {
        return Err(OppoProtocolError::InvalidPacketLength);
    };

    let mut offset = 1usize;
    let mut devices = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let header_end = offset
            .checked_add(10)
            .filter(|end| *end <= payload.len())
            .ok_or(OppoProtocolError::InvalidPacketLength)?;
        let entry_length = payload[offset + 6] as usize;
        let state = payload[offset + 7];
        let flags = payload[offset + 8];
        let name_length = payload[offset + 9] as usize;
        let entry_end = offset
            .checked_add(7 + entry_length)
            .filter(|end| *end <= payload.len())
            .ok_or(OppoProtocolError::InvalidPacketLength)?;
        let name_end = header_end
            .checked_add(name_length)
            .filter(|end| *end <= entry_end)
            .ok_or(OppoProtocolError::InvalidPacketLength)?;
        if entry_length < 3 + name_length {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        let name = std::str::from_utf8(&payload[header_end..name_end])
            .map_err(|_| OppoProtocolError::InvalidPacketLength)?
            .to_string();
        let address = std::array::from_fn(|index| payload[offset + 5 - index]);
        devices.push(PairedDevice {
            name,
            connected: state != 0,
            is_current_host: flags & 0x01 != 0,
            device_type: (flags >> 3) & 0x07,
            address: PairedDeviceAddress::from_display_bytes(address),
        });
        offset = entry_end;
    }

    if offset != payload.len() {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(devices)
}

pub fn parse_oppo_priority_device_response(
    payload: &[u8],
) -> Result<Option<PairedDeviceAddress>, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 3 || payload[1] != 0x02 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    if payload[2] == 0 || payload.len() < 9 {
        return Ok(None);
    }
    // Addresses in 0x0132 are reverse ordered
    let mac = [
        payload[8], payload[7], payload[6], payload[5], payload[4], payload[3],
    ];
    Ok(Some(PairedDeviceAddress::from_display_bytes(mac)))
}

pub use parse_oppo_priority_device_response as parse_priority_device_response;

pub fn encode_oppo_set_priority_device(address: Option<PairedDeviceAddress>) -> Vec<u8> {
    if let Some(addr) = address {
        let mac = addr.command_bytes();
        // 0x0429 format for priority device: [0x04, 0x01, reversed mac]
        vec![0x04, 0x01, mac[5], mac[4], mac[3], mac[2], mac[1], mac[0]]
    } else {
        vec![0x04, 0x00]
    }
}

pub use encode_oppo_set_priority_device as encode_set_priority_device;
