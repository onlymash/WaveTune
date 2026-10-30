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

//! OPPO/OnePlus/realme device capability bitmap (command 0x0102) parsers.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OppoCapabilityBitmap(pub Vec<u8>);

impl OppoCapabilityBitmap {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn command_capability_bit(command: u16) -> Option<u8> {
        match command {
            0x0105 => Some(0),
            0x0106 => Some(1),
            0x0107 => Some(2),
            0x0108 | 0x0401 | 0x0416 => Some(3),
            0x0109 => Some(4),
            0x0400 => Some(5),
            0x0402 => Some(6),
            0x010c | 0x0404 => Some(8),
            0x0405 => Some(9),
            0x0406 | 0x010f => Some(10),
            0x0114 => Some(17),
            0x0205 => Some(22),
            0x0112 | 0x040b => Some(29),
            0x0122 | 0x0418 => Some(34),
            0x0419 => Some(35),
            0x0124 | 0x041b => Some(38),
            0x0422 | 0x012a => Some(47),
            0x0423 | 0x012b => Some(49),
            0x0427 | 0x0130 => Some(57),
            0x0429 | 0x0132 => Some(59),
            0x0431 | 0x0134 => Some(66),
            _ => None,
        }
    }

    pub fn allows_command(&self, command: u16) -> bool {
        capability_bitmap_allows(&self.0, command)
    }
}

pub fn command_capability_bit(command: u16) -> Option<u8> {
    OppoCapabilityBitmap::command_capability_bit(command)
}

pub fn capability_bitmap_allows(bitmap: &[u8], command: u16) -> bool {
    command_capability_bit(command).is_some_and(|bit| {
        bitmap
            .get(usize::from(bit / 8))
            .is_some_and(|byte| byte & (1 << (bit % 8)) != 0)
    })
}
