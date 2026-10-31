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

//! OPPO/OnePlus/realme controls, gestures, wear detection, and hardware settings codecs.

use super::oppo_types::OppoProtocolError;
use crate::protocol::core::types::{
    EarSide, FitQuality, FitTestResult, GestureAction, KeyFunction, NoiseCycleSetting, WearPart,
    WearState,
};

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

pub fn encode_oppo_set_key_function(key_function: &KeyFunction) -> Vec<u8> {
    vec![
        0x01,
        key_function.side as u8,
        key_function.button,
        key_function.action as u8,
        key_function.function,
    ]
}

pub use encode_oppo_set_key_function as encode_set_key_function;

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

pub fn encode_oppo_find_earbuds(ring: bool) -> Vec<u8> {
    vec![if ring { 0x01 } else { 0x00 }]
}

pub use encode_oppo_find_earbuds as encode_find_earbuds;

pub fn encode_oppo_fit_test(start: bool) -> Vec<u8> {
    vec![if start { 0x01 } else { 0x00 }]
}

pub use encode_oppo_fit_test as encode_fit_test;

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

pub fn encode_oppo_set_prompt_volume(volume: u8) -> Vec<u8> {
    vec![volume]
}

pub use encode_oppo_set_prompt_volume as encode_set_prompt_volume;

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

pub fn encode_oppo_set_head_motion_type(motion_type: u8) -> Vec<u8> {
    vec![motion_type]
}

pub use encode_oppo_set_head_motion_type as encode_set_head_motion_type;

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

pub fn encode_oppo_set_noise_cycle(setting: &NoiseCycleSetting) -> Vec<u8> {
    let cycle_type = if setting.is_shared {
        0x01
    } else if setting.side == EarSide::Left {
        0x03
    } else {
        0x04
    };
    let mut payload = vec![0x02, cycle_type];
    // Minimal bytes representing mask
    let bytes = setting.mask.to_le_bytes();
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

