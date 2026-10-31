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

//! OPPO/OnePlus/realme noise mode parsing and command encoding.

pub fn parse_oppo_noise_mode_response(payload: &[u8]) -> Option<u8> {
    if payload.first() != Some(&0) || payload.get(1) != Some(&1) || payload.len() < 4 {
        return None;
    }
    bitmap_index(&payload[3..])
}

pub use parse_oppo_noise_mode_response as parse_noise_mode_response;

pub fn parse_oppo_supported_noise_modes_response(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.first() != Some(&0) || payload.len() < 3 {
        return None;
    }
    if payload.iter().all(|byte| *byte == 0) {
        return Some(Vec::new());
    }
    if payload.get(1) != Some(&3) {
        return None;
    }
    Some(
        payload[3..]
            .iter()
            .enumerate()
            .flat_map(|(byte_index, byte)| {
                (0..8).filter_map(move |bit| {
                    if byte & (1 << bit) == 0 {
                        return None;
                    }
                    u8::try_from(byte_index * 8 + bit).ok()
                })
            })
            .collect(),
    )
}

pub use parse_oppo_supported_noise_modes_response as parse_supported_noise_modes_response;

pub fn parse_oppo_noise_mode_event(payload: &[u8]) -> Option<u8> {
    if payload.first() != Some(&3) || payload.get(1) != Some(&1) || payload.get(2) != Some(&1) {
        return None;
    }
    bitmap_index(&payload[3..])
}

pub use parse_oppo_noise_mode_event as parse_noise_mode_event;

pub fn encode_oppo_noise_mode(protocol_index: u8) -> Vec<u8> {
    let byte_count = usize::from(protocol_index / 8) + 1;
    let mut payload = vec![1, 1];
    payload.resize(2 + byte_count, 0);
    payload[2 + usize::from(protocol_index / 8)] = 1 << (protocol_index % 8);
    payload
}

pub use encode_oppo_noise_mode as encode_noise_mode;

fn bitmap_index(bitmap: &[u8]) -> Option<u8> {
    bitmap.iter().enumerate().find_map(|(byte_index, byte)| {
        (*byte != 0).then(|| (byte_index * 8 + byte.trailing_zeros() as usize) as u8)
    })
}
