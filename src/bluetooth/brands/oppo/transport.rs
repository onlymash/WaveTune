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

use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use crate::bluetooth::transport::{RfcommServiceQuery, is_socket_timeout, open_rfcomm};
use crate::bluetooth::{DeviceChoice, UiUpdate};
use crate::protocol::{AppPacket, FrameDecoder, encode_frame};

use super::unsolicited::handle_oppo_unsolicited;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const OPPO_FALLBACK_RFCOMM_CHANNEL: u8 = 15;
pub const OPPO_PRIMARY_SPP_NAMES: &[&str] = &["elnk", "enco link", "encolink"];
pub const OPPO_SECONDARY_SPP_NAMES: &[&str] = &["jl_spp"];

pub fn oppo_candidate_service_uuids(discovered: &[bluer::Uuid]) -> Vec<bluer::Uuid> {
    let mut ordered = discovered.to_vec();
    for candidate in crate::protocol::brands::oppo::OPPO_CONTROL_SERVICE_CANDIDATES {
        if let Ok(uuid) = bluer::Uuid::parse_str(candidate) {
            if !ordered.contains(&uuid) {
                ordered.push(uuid);
            }
        }
    }
    ordered
}

pub fn open_oppo_rfcomm(device: &DeviceChoice) -> std::io::Result<std::fs::File> {
    let service_uuids = oppo_candidate_service_uuids(&device.control_service_uuids);
    let query = RfcommServiceQuery {
        service_uuids: &service_uuids,
        primary_spp_names: OPPO_PRIMARY_SPP_NAMES,
        secondary_spp_names: OPPO_SECONDARY_SPP_NAMES,
        fallback_channel: Some(OPPO_FALLBACK_RFCOMM_CHANNEL),
    };
    open_rfcomm(device.address, &query)
}

pub fn reply_packet<T: Write>(
    stream: &mut T,
    command: u16,
    sequence: u8,
    payload: Vec<u8>,
) -> Result<(), String> {
    let packet = AppPacket {
        command: command | 0x8000,
        sequence,
        payload,
    };
    let packet_bytes = packet
        .encode()
        .map_err(|error| crate::t!("protocol-command-encode-failed", error))?;
    let frame = encode_frame(&packet_bytes)
        .map_err(|error| crate::t!("protocol-frame-encode-failed", error))?;
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;
    let _ = stream.flush();
    Ok(())
}

pub fn request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut FrameDecoder,
    command: u16,
    sequence: u8,
    payload: Vec<u8>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<AppPacket, String> {
    let packet = AppPacket {
        command,
        sequence,
        payload,
    };
    let frame = encode_frame(
        &packet
            .encode()
            .map_err(|error| crate::t!("protocol-command-encode-failed", error))?,
    )
    .map_err(|error| crate::t!("protocol-frame-encode-failed", error))?;
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    let deadline = Instant::now() + REQUEST_TIMEOUT;
    loop {
        let mut bytes = [0; 1024];
        let length = match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(length) => length,
            Err(error) if is_socket_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(crate::t!(
                        "protocol-query-timeout",
                        format!("{command:04X}")
                    ));
                }
                continue;
            }
            Err(error) => return Err(crate::t!("protocol-response-read-failed", error)),
        };
        let frames = decoder
            .push(&bytes[..length])
            .map_err(|error| crate::t!("protocol-frame-decode-failed", error))?;
        let mut response = None;
        let mut trailing_packets = Vec::new();
        for (index, frame) in frames.iter().enumerate() {
            let Ok(received) = AppPacket::decode(&frame) else {
                continue;
            };
            if received.command == (command | 0x8000) && received.sequence == sequence {
                response = Some(received);
                trailing_packets.extend(frames.iter().skip(index + 1).cloned());
                break;
            }
            handle_oppo_unsolicited(stream, &received, updates);
        }
        if let Some(response) = response {
            for bytes in trailing_packets {
                if let Ok(packet) = AppPacket::decode(&bytes) {
                    handle_oppo_unsolicited(stream, &packet, updates);
                }
            }
            if response.payload.first() != Some(&0) {
                let missing = crate::t!("common-missing");
                let status_str = response
                    .payload
                    .first()
                    .map_or(missing, |status| format!("0x{status:02X}"));
                return Err(crate::t!(
                    "protocol-command-rejected",
                    format!("{command:04X}"),
                    status_str
                ));
            }
            return Ok(response);
        }
        if Instant::now() >= deadline {
            return Err(crate::t!(
                "protocol-query-timeout",
                format!("{command:04X}")
            ));
        }
    }
}
