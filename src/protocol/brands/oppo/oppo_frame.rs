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

use super::oppo_types::OppoProtocolError;

pub const OPPO_FRAME_START: u8 = 0xaa;
pub const FRAME_START: u8 = OPPO_FRAME_START;
const MAX_FRAME_LEN: usize = 660;
const MAX_MESSAGE_LEN: usize = 8192;
const MAX_BUFFER_LEN: usize = 64 * 1024;

pub fn encode_oppo_frame(packet: &[u8]) -> Result<Vec<u8>, OppoProtocolError> {
    let body_len = packet
        .len()
        .checked_add(2)
        .ok_or(OppoProtocolError::MessageTooLong)?;
    if body_len > MAX_FRAME_LEN {
        return Err(OppoProtocolError::MessageTooLong);
    }

    let mut frame = Vec::with_capacity(1 + 2 + body_len);
    frame.push(OPPO_FRAME_START);
    let mut remaining = body_len;
    while remaining >= 0x80 {
        frame.push(((remaining & 0x7f) as u8) | 0x80);
        remaining >>= 7;
    }
    frame.push(remaining as u8);
    frame.extend_from_slice(&[0, 0]);
    frame.extend_from_slice(packet);
    Ok(frame)
}

pub use encode_oppo_frame as encode_frame;

#[derive(Debug, Default)]
pub struct OppoFrameDecoder {
    buffer: Vec<u8>,
    fragmented_message: Option<(u8, Vec<u8>)>,
}

pub type FrameDecoder = OppoFrameDecoder;

impl OppoFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, input: &[u8]) -> Result<Vec<Vec<u8>>, OppoProtocolError> {
        if self.buffer.len().saturating_add(input.len()) > MAX_BUFFER_LEN {
            self.buffer.clear();
            self.fragmented_message = None;
            return Err(OppoProtocolError::MessageTooLong);
        }
        self.buffer.extend_from_slice(input);
        let mut packets = Vec::new();

        loop {
            let Some(start) = self
                .buffer
                .iter()
                .position(|byte| *byte == OPPO_FRAME_START)
            else {
                self.buffer.clear();
                break;
            };
            if start > 0 {
                self.buffer.drain(..start);
            }
            if self.buffer.len() < 2 {
                break;
            }

            let mut body_len = 0usize;
            let mut shift = 0;
            let mut length_bytes = 0;
            let mut length_complete = false;
            for byte in self.buffer.iter().skip(1).copied().take(2) {
                length_bytes += 1;
                body_len |= ((byte & 0x7f) as usize) << shift;
                if byte & 0x80 == 0 {
                    length_complete = true;
                    break;
                }
                shift += 7;
            }
            if !length_complete {
                if self.buffer.len() < 3 {
                    break;
                }
                self.buffer.drain(..1);
                self.fragmented_message = None;
                return Err(OppoProtocolError::InvalidVarint);
            }
            if !(2..=MAX_FRAME_LEN).contains(&body_len) {
                self.buffer.drain(..1);
                self.fragmented_message = None;
                return Err(OppoProtocolError::InvalidFrameLength);
            }

            let frame_len = 1 + length_bytes + body_len;
            if self.buffer.len() < frame_len {
                break;
            }

            let control_offset = 1 + length_bytes;
            let control = self.buffer[control_offset] & 0x03;
            let data_offset = control_offset + 2;
            let frame_end = frame_len;
            let data = self.buffer[data_offset..frame_end].to_vec();
            self.buffer.drain(..frame_len);

            match control {
                0 => {
                    self.fragmented_message = None;
                    packets.push(data);
                }
                1 => {
                    if body_len < 3 || data.is_empty() {
                        self.fragmented_message = None;
                        return Err(OppoProtocolError::InvalidFrameLength);
                    }
                    if data.len() > MAX_MESSAGE_LEN {
                        self.fragmented_message = None;
                        return Err(OppoProtocolError::MessageTooLong);
                    }
                    self.fragmented_message = Some((data[0], data[1..].to_vec()));
                }
                2 | 3 => {
                    if body_len < 3 {
                        self.fragmented_message = None;
                        return Err(OppoProtocolError::InvalidFrameLength);
                    }
                    let Some((expected_sequence, mut message)) = self.fragmented_message.take()
                    else {
                        return Err(OppoProtocolError::InvalidFragmentSequence);
                    };
                    let sequence = data[0];
                    if sequence != expected_sequence.wrapping_add(1) {
                        return Err(OppoProtocolError::InvalidFragmentSequence);
                    }
                    message.extend_from_slice(&data[1..]);
                    if message.len() > MAX_MESSAGE_LEN {
                        return Err(OppoProtocolError::MessageTooLong);
                    }
                    if control == 3 {
                        packets.push(message);
                    } else {
                        self.fragmented_message = Some((sequence, message));
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(packets)
    }
}
