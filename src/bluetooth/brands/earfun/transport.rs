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
    os::fd::AsRawFd,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use crate::app::events::UiUpdate;
use crate::bluetooth::{
    DeviceChoice,
    transport::{RfcommServiceQuery, is_socket_timeout, open_rfcomm},
};
use crate::protocol::brands::earfun::{EarFunCommands, EarFunFrameDecoder, EarFunPacket};

use super::unsolicited::handle_unsolicited;

pub const EARFUN_AIR_PRO_4_PRODUCT_ID: u32 = 0x004535;
pub const EARFUN_FREE_PRO_3_PRODUCT_ID: u32 = 0x004534;
pub const EARFUN_AIR_S_PRODUCT_ID: u32 = 0x004c45;

pub const EARFUN_FALLBACK_RFCOMM_CHANNEL: u8 = 11;
pub const GAIA_SERVICE_UUID: &str = "00001107-d102-11e1-9b23-00025b00a5a5";
pub const EARFUN_PRIMARY_SPP_NAMES: &[&str] = &[];
pub const EARFUN_SECONDARY_SPP_NAMES: &[&str] = &["gaia", "earfun", "spp", "serial port"];

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const POLL_INTERVAL: Duration = Duration::from_secs(15);

pub fn open_earfun_rfcomm(device: &DeviceChoice) -> std::io::Result<std::fs::File> {
    let mut service_uuids = device.control_service_uuids.clone();
    if service_uuids.is_empty() {
        if let Ok(gaia_uuid) = bluer::Uuid::parse_str(GAIA_SERVICE_UUID) {
            service_uuids.push(gaia_uuid);
        }
    }
    let query = RfcommServiceQuery {
        service_uuids: &service_uuids,
        primary_spp_names: EARFUN_PRIMARY_SPP_NAMES,
        secondary_spp_names: EARFUN_SECONDARY_SPP_NAMES,
        fallback_channel: Some(EARFUN_FALLBACK_RFCOMM_CHANNEL),
    };
    open_rfcomm(device.address, &query)
}

pub fn send_command<T: Write>(
    stream: &mut T,
    command: u16,
    payload: Vec<u8>,
) -> Result<(), String> {
    let pkt = if command == EarFunCommands::SET_EQUALIZER_BAND {
        EarFunPacket::new_v3(command, payload)
    } else {
        EarFunPacket::new(command, payload)
    };
    let frame = pkt.encode();
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;
    Ok(())
}

pub fn drain_stream<T: Read>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
    duration: Duration,
) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        let mut bytes = [0u8; 512];
        match stream.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => {
                let packets = decoder.push(&bytes[..n]);
                for packet in packets {
                    handle_unsolicited(&packet, updates);
                }
            }
            Err(e) if is_socket_timeout(&e) => break,
            Err(_) => break,
        }
    }
}

pub fn request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    command: u16,
    payload: Vec<u8>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<EarFunPacket, String> {
    let pkt = if command == EarFunCommands::SET_EQUALIZER_BAND {
        EarFunPacket::new_v3(command, payload)
    } else {
        EarFunPacket::new(command, payload)
    };
    let frame = pkt.encode();
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

        let packets = decoder.push(&bytes[..length]);
        let mut response = None;
        let mut trailing_packets = Vec::new();

        for (index, packet) in packets.iter().enumerate() {
            let is_match = packet.command == (command | 0x8000)
                || (command == EarFunCommands::SET_EQUALIZER_BAND
                    && (packet.command == EarFunCommands::ACK_EQUALIZER_BAND
                        || packet.command == (command | 0x8000)));
            if is_match {
                response = Some(packet.clone());
                trailing_packets.extend(packets.iter().skip(index + 1).cloned());
                break;
            }
            handle_unsolicited(packet, updates);
        }

        if let Some(response) = response {
            for packet in trailing_packets {
                handle_unsolicited(&packet, updates);
            }
            if !response.payload.is_empty()
                && response.command != EarFunCommands::ACK_EQUALIZER_BAND
                && response.payload[0] != 0x00
            {
                let status_str = format!("0x{:02X}", response.payload[0]);
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

#[repr(C)]
struct SockAddrRc {
    family: libc::sa_family_t,
    address: [u8; 6],
    channel: u8,
}

pub fn get_local_rfcomm_lap<T: AsRawFd>(stream: &T) -> Option<[u8; 3]> {
    let mut addr = SockAddrRc {
        family: 0,
        address: [0; 6],
        channel: 0,
    };
    let mut len = std::mem::size_of::<SockAddrRc>() as libc::socklen_t;
    let res = unsafe {
        libc::getsockname(
            stream.as_raw_fd(),
            &mut addr as *mut SockAddrRc as *mut libc::sockaddr,
            &mut len,
        )
    };
    if res == 0 && addr.family == libc::AF_BLUETOOTH as libc::sa_family_t {
        Some([addr.address[2], addr.address[1], addr.address[0]])
    } else {
        None
    }
}
