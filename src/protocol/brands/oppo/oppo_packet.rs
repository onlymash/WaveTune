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

use super::oppo_types::{OppoAppPacket, OppoProtocolError};

impl OppoAppPacket {
    pub fn new(command: u16, sequence: u8, payload: Vec<u8>) -> Self {
        Self {
            command,
            sequence,
            payload,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, OppoProtocolError> {
        let payload_len =
            u16::try_from(self.payload.len()).map_err(|_| OppoProtocolError::MessageTooLong)?;
        let mut packet = Vec::with_capacity(5 + self.payload.len());
        packet.extend_from_slice(&self.command.to_le_bytes());
        packet.push(self.sequence);
        packet.extend_from_slice(&payload_len.to_le_bytes());
        packet.extend_from_slice(&self.payload);
        Ok(packet)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, OppoProtocolError> {
        if bytes.len() < 5 {
            return Err(OppoProtocolError::InvalidPacketLength);
        }

        let payload_len = u16::from_le_bytes([bytes[3], bytes[4]]) as usize;
        if bytes.len() != 5 + payload_len {
            return Err(OppoProtocolError::InvalidPacketLength);
        }

        Ok(Self {
            command: u16::from_le_bytes([bytes[0], bytes[1]]),
            sequence: bytes[2],
            payload: bytes[5..].to_vec(),
        })
    }
}
