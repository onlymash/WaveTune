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

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OppoAppPacket {
    pub command: u16,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

pub type AppPacket = OppoAppPacket;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OppoProtocolError {
    InvalidFrameLength,
    InvalidVarint,
    InvalidFragmentSequence,
    MessageTooLong,
    InvalidPacketLength,
}

pub type ProtocolError = OppoProtocolError;

impl fmt::Display for OppoProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrameLength => f.write_str("invalid OPPO frame length"),
            Self::InvalidVarint => f.write_str("invalid OPPO frame length varint"),
            Self::InvalidFragmentSequence => f.write_str("invalid OPPO fragment sequence"),
            Self::MessageTooLong => f.write_str("OPPO message exceeds the safety limit"),
            Self::InvalidPacketLength => f.write_str("invalid OPPO application packet length"),
        }
    }
}

impl std::error::Error for OppoProtocolError {}
