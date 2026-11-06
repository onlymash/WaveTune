// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-only

use super::earfun_packet::EarFunProtocolError;

pub const EARFUN_EF_SOP: u8 = 0xEF;
pub const EARFUN_EF_EOP: u8 = 0xFE;
pub const EARFUN_EF_MAX_PAYLOAD: usize = 255;
const MAX_BUFFER_LENGTH: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarFunEfPacket {
    pub command: u16,
    pub payload: Vec<u8>,
}

impl EarFunEfPacket {
    pub fn new(command: u16, payload: Vec<u8>) -> Result<Self, EarFunProtocolError> {
        if payload.len() > EARFUN_EF_MAX_PAYLOAD {
            return Err(EarFunProtocolError::InvalidPacketLength);
        }
        Ok(Self { command, payload })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EarFunProtocolError> {
        if self.payload.len() > EARFUN_EF_MAX_PAYLOAD {
            return Err(EarFunProtocolError::InvalidPacketLength);
        }
        let length = self.payload.len() as u8;
        let checksum =
            self.payload
                .iter()
                .fold(usize::from(length), |sum, byte| sum + usize::from(*byte)) as u8;
        let mut frame = Vec::with_capacity(self.payload.len() + 6);
        frame.extend_from_slice(&[
            EARFUN_EF_SOP,
            (self.command >> 8) as u8,
            self.command as u8,
            length,
        ]);
        frame.extend_from_slice(&self.payload);
        frame.extend_from_slice(&[checksum, EARFUN_EF_EOP]);
        Ok(frame)
    }

    fn decode(frame: &[u8]) -> Result<Self, EarFunProtocolError> {
        if frame.len() < 6 || frame[0] != EARFUN_EF_SOP || frame[frame.len() - 1] != EARFUN_EF_EOP {
            return Err(EarFunProtocolError::InvalidHeader);
        }
        let payload_length = usize::from(frame[3]);
        if frame.len() != payload_length + 6 {
            return Err(EarFunProtocolError::InvalidPacketLength);
        }
        let payload = &frame[4..4 + payload_length];
        let checksum = payload
            .iter()
            .fold(payload_length, |sum, byte| sum + usize::from(*byte))
            as u8;
        if frame[4 + payload_length] != checksum {
            return Err(EarFunProtocolError::InvalidHeader);
        }
        Ok(Self {
            command: u16::from_be_bytes([frame[1], frame[2]]),
            payload: payload.to_vec(),
        })
    }
}

#[derive(Default)]
pub struct EarFunEfFrameDecoder {
    buffer: Vec<u8>,
}

impl EarFunEfFrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<EarFunEfPacket> {
        let mut packets = Vec::new();
        for byte in bytes {
            if self.buffer.len() == MAX_BUFFER_LENGTH {
                self.buffer.clear();
            }
            self.buffer.push(*byte);
            self.decode_available(&mut packets);
        }
        packets
    }

    fn decode_available(&mut self, packets: &mut Vec<EarFunEfPacket>) {
        loop {
            let Some(start) = self.buffer.iter().position(|byte| *byte == EARFUN_EF_SOP) else {
                self.buffer.clear();
                return;
            };
            if start != 0 {
                self.buffer.drain(..start);
            }
            if self.buffer.len() < 4 {
                return;
            }
            let frame_length = usize::from(self.buffer[3]) + 6;
            if self.buffer.len() < frame_length {
                return;
            }
            match EarFunEfPacket::decode(&self.buffer[..frame_length]) {
                Ok(packet) => {
                    packets.push(packet);
                    self.buffer.drain(..frame_length);
                }
                Err(_) => {
                    self.buffer.drain(..1);
                }
            }
        }
    }
}
