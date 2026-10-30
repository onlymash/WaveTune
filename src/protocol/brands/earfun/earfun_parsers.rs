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

use super::earfun_packet::EarFunProtocolError;
use crate::protocol::core::types::{BatteryPart, FirmwareVersions};

/// Parse a single battery level response (commands 0x0306, 0x0307, 0x0317).
/// EarFun payload format: [0x00 (status), level (0..100)]
pub fn parse_earfun_battery_component(payload: &[u8]) -> Result<BatteryPart, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    let percentage = payload[1];
    Ok(BatteryPart {
        percentage,
        charging: false,
    })
}

/// Parse firmware version response (command 0x0309).
/// Format: [0x00, UTF-8 string] e.g. "TW600-3091_20250919_v0.4.1"
pub fn parse_earfun_firmware_version(
    payload: &[u8],
) -> Result<FirmwareVersions, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    let s =
        std::str::from_utf8(&payload[1..]).map_err(|_| EarFunProtocolError::InvalidPacketLength)?;
    let parts: Vec<&str> = s.split('_').collect();
    let version_str = parts.last().unwrap_or(&s).trim_start_matches('v');
    Ok(FirmwareVersions {
        left: Some(version_str.to_string()),
        right: Some(version_str.to_string()),
        case: None,
    })
}

/// Parse ambient sound master mode (0x0315).
/// Returns 0: Off, 1: ANC, 2: Ambient/Transparency.
pub fn parse_earfun_ambient_mode(payload: &[u8]) -> Result<u8, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

/// Parse ANC sub-mode (0x033B).
/// 0: AI Ear, 1: Wind, 2: Strong, 3: Balanced, 4: AI Environment.
pub fn parse_earfun_anc_sub_mode(payload: &[u8]) -> Result<u8, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

/// Parse transparency sub-mode (0x033D).
/// 0: Default, 1: Natural.
pub fn parse_earfun_transparency_sub_mode(payload: &[u8]) -> Result<u8, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

/// Parse game mode / low latency (0x0313).
pub fn parse_earfun_game_mode(payload: &[u8]) -> Result<bool, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1] == 1)
}

/// Parse in-ear detection status (0x034A).
/// 0: In-ear detection enabled, 1: disabled.
pub fn parse_earfun_in_ear_detection(payload: &[u8]) -> Result<bool, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1] == 0)
}

/// Parse dual device / multipoint connection (0x0326).
pub fn parse_earfun_dual_device(payload: &[u8]) -> Result<bool, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1] == 1)
}

/// Encode 10-band equalizer band command (0x0E01).
/// band_id: 0xA1..0xAA
/// gain_db: -10..+10 (float/double)
pub fn encode_earfun_eq_band(band_id: u8, frequency: u16, q_factor: u16, gain_val: f64) -> Vec<u8> {
    // Gaia uses factor 60 to convert dB to raw, EarFun projects +-6dB over slider scale 10.
    // factor = 60 * 0.6 = 36.0
    let gain_raw = (gain_val * 36.0).round() as i16;
    let mut payload = Vec::with_capacity(9);
    payload.push(band_id);
    payload.push(0xFF);
    payload.push(0x98);
    payload.extend_from_slice(&frequency.to_be_bytes());
    payload.extend_from_slice(&gain_raw.to_be_bytes());
    payload.extend_from_slice(&q_factor.to_be_bytes());
    payload
}
