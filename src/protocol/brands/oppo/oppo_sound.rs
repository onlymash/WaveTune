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

//! OPPO/OnePlus/realme sound, equalizer, and audio enhancement codecs.

use super::oppo_types::OppoProtocolError;
use crate::protocol::core::types::{
    BassEngineLevel, CustomEqualizer, CustomEqualizerAction, CustomEqualizerParams,
};

pub const FEATURE_HI_RES: u8 = 24;
pub const FEATURE_BASS_ENGINE: u8 = 29;

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
    payload
        .first()
        .copied()
        .ok_or(OppoProtocolError::InvalidPacketLength)
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

pub fn encode_oppo_custom_equalizer(
    action: CustomEqualizerAction,
    params: &CustomEqualizerParams<'_>,
) -> Vec<u8> {
    let name_bytes = params.name.as_bytes();
    let name_len = name_bytes.len().min(255);
    let mut payload = Vec::with_capacity(6 + name_len + params.bands.len() * 3);
    payload.push(action as u8);
    payload.push(params.min_gain as u8);
    payload.push(params.max_gain as u8);
    payload.push(params.id);
    payload.push(name_len as u8);
    payload.extend_from_slice(&name_bytes[..name_len]);
    payload.push(params.bands.len().min(255) as u8);
    for &(freq, gain) in params.bands {
        payload.extend_from_slice(&freq.to_le_bytes());
        payload.push(gain.clamp(params.min_gain, params.max_gain) as u8);
    }
    payload
}

pub use encode_oppo_custom_equalizer as encode_oppo_custom_equalizer_command;
pub use encode_oppo_custom_equalizer as encode_custom_equalizer_command;

