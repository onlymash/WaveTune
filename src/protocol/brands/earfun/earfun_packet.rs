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

pub const EARFUN_HEADER_LENGTH: usize = 8;
pub const EARFUN_SOP: u8 = 0xFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarFunProtocolError {
    InvalidHeader,
    InvalidStartOfPacket,
    InvalidPacketLength,
    BufferTooShort,
}

impl fmt::Display for EarFunProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => f.write_str("invalid EarFun/GAIA packet header"),
            Self::InvalidStartOfPacket => f.write_str("invalid EarFun/GAIA start of packet byte"),
            Self::InvalidPacketLength => f.write_str("invalid EarFun/GAIA packet length"),
            Self::BufferTooShort => f.write_str("EarFun/GAIA buffer too short for complete frame"),
        }
    }
}

impl std::error::Error for EarFunProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarFunPacket {
    pub version: u8,
    pub flags: u8,
    pub vendor_id: u16,
    pub command: u16,
    pub payload: Vec<u8>,
}

impl EarFunPacket {
    pub fn new(command: u16, payload: Vec<u8>) -> Self {
        Self {
            version: 0x04,
            flags: 0x00,
            vendor_id: 0x000A,
            command,
            payload,
        }
    }

    pub fn new_v3(command: u16, payload: Vec<u8>) -> Self {
        Self {
            version: 0x04,
            flags: 0x00,
            vendor_id: 0x001D,
            command,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(EARFUN_HEADER_LENGTH + self.payload.len());
        buf.push(EARFUN_SOP);
        buf.push(self.version);
        buf.push(self.flags);
        buf.push(self.payload.len() as u8);
        buf.extend_from_slice(&self.vendor_id.to_be_bytes());
        buf.extend_from_slice(&self.command.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), EarFunProtocolError> {
        if bytes.len() < EARFUN_HEADER_LENGTH {
            return Err(EarFunProtocolError::BufferTooShort);
        }
        if bytes[0] != EARFUN_SOP {
            return Err(EarFunProtocolError::InvalidStartOfPacket);
        }
        let version = bytes[1];
        let flags = bytes[2];
        let length = bytes[3] as usize;
        let total_length = EARFUN_HEADER_LENGTH + length;
        if bytes.len() < total_length {
            return Err(EarFunProtocolError::BufferTooShort);
        }
        let vendor_id = u16::from_be_bytes([bytes[4], bytes[5]]);
        let command = u16::from_be_bytes([bytes[6], bytes[7]]);
        let payload = bytes[EARFUN_HEADER_LENGTH..total_length].to_vec();

        Ok((
            Self {
                version,
                flags,
                vendor_id,
                command,
                payload,
            },
            total_length,
        ))
    }
}

#[derive(Default)]
pub struct EarFunFrameDecoder {
    buffer: Vec<u8>,
}

impl EarFunFrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<EarFunPacket> {
        self.buffer.extend_from_slice(bytes);
        let mut packets = Vec::new();

        loop {
            // Find SOP 0xFF
            if let Some(sop_idx) = self.buffer.iter().position(|&b| b == EARFUN_SOP) {
                if sop_idx > 0 {
                    self.buffer.drain(..sop_idx);
                }
            } else {
                self.buffer.clear();
                break;
            }

            if self.buffer.len() < EARFUN_HEADER_LENGTH {
                break;
            }

            let payload_len = self.buffer[3] as usize;
            let packet_len = EARFUN_HEADER_LENGTH + payload_len;

            if self.buffer.len() < packet_len {
                break;
            }

            match EarFunPacket::decode(&self.buffer[..packet_len]) {
                Ok((packet, consumed)) => {
                    self.buffer.drain(..consumed);
                    packets.push(packet);
                }
                Err(_) => {
                    // Skip invalid SOP and retry
                    self.buffer.drain(..1);
                }
            }
        }

        packets
    }
}
