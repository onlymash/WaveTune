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

//! Parsers for OPPO/OnePlus/realme response and event payloads.

use super::oppo_types::OppoProtocolError;
use crate::protocol::core::types::{
    BassEngineLevel, BatteryPart, BatteryState, CustomEqualizer, EarSide, FirmwareVersions,
    FitQuality, FitTestResult, GestureAction, KeyFunction, PairedDevice, PairedDeviceAddress,
    WearPart, WearState,
};

pub const FEATURE_HI_RES: u8 = 24;
pub const FEATURE_BASS_ENGINE: u8 = 29;

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

pub fn parse_oppo_wear_response(payload: &[u8]) -> Result<WearState, OppoProtocolError> {
    if payload.first() != Some(&0) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_wear_components(&payload[1..])
}

pub use parse_oppo_wear_response as parse_wear_response;

pub fn parse_oppo_wear_event(payload: &[u8]) -> Result<WearState, OppoProtocolError> {
    if payload.first() != Some(&2) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    parse_wear_components(&payload[1..])
}

pub use parse_oppo_wear_event as parse_wear_event;

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

pub fn parse_oppo_feature_flags_response(
    payload: &[u8],
    feature_id: u8,
) -> Result<bool, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let count = payload[1] as usize;
    let records = &payload[2..];
    if records.len() != count * 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(records
        .chunks_exact(2)
        .find(|record| record[0] == feature_id)
        .is_some_and(|record| record[1] != 0))
}

pub use parse_oppo_feature_flags_response as parse_feature_flags_response;

pub fn parse_oppo_equalizer_response(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

pub use parse_oppo_equalizer_response as parse_equalizer_response;

pub fn parse_oppo_equalizer_event(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    let Some(&preset) = payload.first() else {
        return Err(OppoProtocolError::InvalidPacketLength);
    };
    Ok(preset)
}

pub use parse_oppo_equalizer_event as parse_equalizer_event;

pub fn parse_oppo_spatial_audio_response(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 || payload[1] > 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

pub use parse_oppo_spatial_audio_response as parse_spatial_audio_response;

pub fn parse_oppo_bass_engine_response(
    payload: &[u8],
) -> Result<BassEngineLevel, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 4 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let min = payload[1] as i8;
    let max = payload[2] as i8;
    let current = payload[3] as i8;
    if min > max {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(BassEngineLevel {
        min,
        max,
        current: current.clamp(min, max),
    })
}

pub use parse_oppo_bass_engine_response as parse_bass_engine_response;

pub fn parse_oppo_bass_engine_event(payload: &[u8]) -> Result<BassEngineLevel, OppoProtocolError> {
    let (min, max, current) = if payload.len() >= 4 && payload[0] == 0 {
        (payload[1] as i8, payload[2] as i8, payload[3] as i8)
    } else if payload.len() >= 3 {
        (payload[0] as i8, payload[1] as i8, payload[2] as i8)
    } else {
        return Err(OppoProtocolError::InvalidPacketLength);
    };
    if min > max {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(BassEngineLevel {
        min,
        max,
        current: current.clamp(min, max),
    })
}

pub use parse_oppo_bass_engine_event as parse_bass_engine_event;

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

fn parse_wear_components(payload: &[u8]) -> Result<WearState, OppoProtocolError> {
    let Some((&count, records)) = payload.split_first() else {
        return Err(OppoProtocolError::InvalidPacketLength);
    };
    if records.len() != count as usize * 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }

    let mut state = WearState::default();
    for record in records.chunks_exact(2) {
        let component = record[0];
        let flags = record[1];
        let part = WearPart {
            out_of_case: flags & 0x01 != 0,
            in_ear: flags & 0x02 != 0,
            raw_flags: flags,
        };
        match component {
            1 => state.left = Some(part),
            2 => state.right = Some(part),
            3 => state.case_open = Some(flags & 0x04 != 0),
            _ => {}
        }
    }
    Ok(state)
}

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

pub fn parse_oppo_custom_equalizer_list(
    payload: &[u8],
    has_status: bool,
) -> Result<Vec<CustomEqualizer>, OppoProtocolError> {
    let mut offset = 0;
    if has_status {
        if payload.is_empty() {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        if payload[0] != 0 {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        offset += 1;
    }
    if offset >= payload.len() {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let count = payload[offset] as usize;
    offset += 1;

    let mut list = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 5 > payload.len() {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        let selected = payload[offset] != 0;
        let min_gain = payload[offset + 1] as i8;
        let max_gain = payload[offset + 2] as i8;
        let id = payload[offset + 3];
        let name_len = payload[offset + 4] as usize;
        offset += 5;

        if offset + name_len > payload.len() {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        let name = match std::str::from_utf8(&payload[offset..offset + name_len]) {
            Ok(s) => s.to_string(),
            Err(_) => return Err(OppoProtocolError::InvalidPacketLength),
        };
        offset += name_len;

        if offset >= payload.len() {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        let band_count = payload[offset] as usize;
        offset += 1;

        if offset + band_count * 3 > payload.len() {
            return Err(OppoProtocolError::InvalidPacketLength);
        }
        let mut bands = Vec::with_capacity(band_count);
        for _ in 0..band_count {
            let freq = u16::from_le_bytes([payload[offset], payload[offset + 1]]);
            let gain = payload[offset + 2] as i8;
            offset += 3;
            bands.push((freq, gain));
        }
        list.push(CustomEqualizer {
            id,
            name,
            min_gain,
            max_gain,
            is_selected: selected,
            bands,
        });
    }
    Ok(list)
}

pub use parse_oppo_custom_equalizer_list as parse_custom_equalizer_list;

pub fn encode_oppo_custom_equalizer_command(
    action: u8,
    id: u8,
    name: &str,
    min_gain: i8,
    max_gain: i8,
    bands: &[(u16, i8)],
) -> Vec<u8> {
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(255);
    let mut payload = Vec::with_capacity(6 + name_len + bands.len() * 3);
    payload.push(action);
    payload.push(min_gain as u8);
    payload.push(max_gain as u8);
    payload.push(id);
    payload.push(name_len as u8);
    payload.extend_from_slice(&name_bytes[..name_len]);
    payload.push(bands.len().min(255) as u8);
    for &(freq, gain) in bands {
        payload.extend_from_slice(&freq.to_le_bytes());
        payload.push(gain.clamp(min_gain, max_gain) as u8);
    }
    payload
}

pub use encode_oppo_custom_equalizer_command as encode_custom_equalizer_command;

pub fn parse_oppo_touch_controls_response(
    payload: &[u8],
) -> Result<Vec<KeyFunction>, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let count = payload[1] as usize;
    if payload.len() < 2 + count * 4 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let mut list = Vec::with_capacity(count);
    for i in 0..count {
        let offset = 2 + i * 4;
        let side = EarSide::from_u8(payload[offset]);
        let button = payload[offset + 1];
        let action = GestureAction::from_u8(payload[offset + 2]);
        let function = payload[offset + 3];
        if let (Some(side), Some(action)) = (side, action) {
            list.push(KeyFunction {
                side,
                button,
                action,
                function,
            });
        }
    }
    Ok(list)
}

pub use parse_oppo_touch_controls_response as parse_touch_controls_response;

pub fn parse_oppo_fit_test_event(payload: &[u8]) -> Result<FitTestResult, OppoProtocolError> {
    if payload.first() != Some(&4) || payload.len() < 5 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let mut left = FitQuality::Unknown;
    let mut right = FitQuality::Unknown;
    let mut i = 1;
    while i + 1 < payload.len() && i <= 3 {
        let quality = match payload[i + 1] {
            1 => FitQuality::Good,
            0 => FitQuality::Average,
            6 => FitQuality::Poor,
            _ => FitQuality::Failed,
        };
        if payload[i] == 1 {
            left = quality;
        } else if payload[i] == 2 {
            right = quality;
        }
        i += 2;
    }
    Ok(FitTestResult { left, right })
}

pub use parse_oppo_fit_test_event as parse_fit_test_event;

pub fn parse_oppo_prompt_volume_response(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let volume = payload[1];
    if !(1..=10).contains(&volume) {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(volume)
}

pub use parse_oppo_prompt_volume_response as parse_prompt_volume_response;

pub fn parse_oppo_head_motion_type_response(payload: &[u8]) -> Result<u8, OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 2 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let motion_type = payload[1];
    if motion_type > 1 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    Ok(motion_type)
}

pub use parse_oppo_head_motion_type_response as parse_head_motion_type_response;

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

pub fn parse_oppo_noise_cycle_response(payload: &[u8]) -> Result<(u8, u32), OppoProtocolError> {
    if payload.first() != Some(&0) || payload.len() < 4 || payload[1] != 0x02 {
        return Err(OppoProtocolError::InvalidPacketLength);
    }
    let cycle_type = payload[2];
    let mut mask = 0u32;
    for (i, &byte) in payload[3..].iter().enumerate() {
        if i >= 4 {
            break;
        }
        mask |= (byte as u32) << (i * 8);
    }
    Ok((cycle_type, mask))
}

pub use parse_oppo_noise_cycle_response as parse_noise_cycle_response;

pub fn encode_oppo_set_key_function(
    side: EarSide,
    button: u8,
    action: GestureAction,
    function: u8,
) -> Vec<u8> {
    vec![0x01, side as u8, button, action as u8, function]
}

pub use encode_oppo_set_key_function as encode_set_key_function;

pub fn encode_oppo_find_earbuds(ring: bool) -> Vec<u8> {
    vec![if ring { 0x01 } else { 0x00 }]
}

pub use encode_oppo_find_earbuds as encode_find_earbuds;

pub fn encode_oppo_fit_test(start: bool) -> Vec<u8> {
    vec![if start { 0x01 } else { 0x00 }]
}

pub use encode_oppo_fit_test as encode_fit_test;

pub fn encode_oppo_set_prompt_volume(volume: u8) -> Vec<u8> {
    vec![volume]
}

pub use encode_oppo_set_prompt_volume as encode_set_prompt_volume;

pub fn encode_oppo_set_head_motion_type(motion_type: u8) -> Vec<u8> {
    vec![motion_type]
}

pub use encode_oppo_set_head_motion_type as encode_set_head_motion_type;

pub fn encode_oppo_set_noise_cycle(side: EarSide, mask: u32, is_shared: bool) -> Vec<u8> {
    let cycle_type = if is_shared {
        0x01
    } else if side == EarSide::Left {
        0x03
    } else {
        0x04
    };
    let mut payload = vec![0x02, cycle_type];
    // Minimal bytes representing mask
    let bytes = mask.to_le_bytes();
    let len = if bytes[3] != 0 {
        4
    } else if bytes[2] != 0 {
        3
    } else if bytes[1] != 0 {
        2
    } else {
        1
    };
    payload.extend_from_slice(&bytes[..len]);
    payload
}

pub use encode_oppo_set_noise_cycle as encode_set_noise_cycle;

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
