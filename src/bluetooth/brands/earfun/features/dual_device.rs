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
    thread,
    time::{Duration, Instant},
};

use crate::app::events::UiUpdate;
use crate::bluetooth::brands::earfun::transport::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, drain_stream, send_command,
};
use crate::bluetooth::brands::earfun::unsolicited::handle_unsolicited;
use crate::bluetooth::transport::is_socket_timeout;
use crate::protocol::PairedDevice;
use crate::protocol::brands::earfun::{
    EarFunAudioCodec, EarFunCommands, EarFunFrameDecoder, parse_earfun_paired_device_entry,
};

pub fn set_dual_device<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    enabled: bool,
    current_codec: Option<u8>,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    // If enabling dual device on EarFun Air Pro 4 and current codec is LDAC (8),
    // automatically fall back to aptX Adaptive (3) before enabling dual device!
    if product_id == EARFUN_AIR_PRO_4_PRODUCT_ID
        && enabled
        && current_codec == Some(EarFunAudioCodec::Ldac.as_u8())
    {
        let fallback = EarFunAudioCodec::AptXAdaptive.as_u8();
        let _ = send_command(stream, EarFunCommands::SET_AUDIO_CODEC, vec![fallback]);
        let _ = updates.send(UiUpdate::AudioCodec(fallback));
        thread::sleep(Duration::from_millis(50));
    }

    send_command(
        stream,
        EarFunCommands::SET_CONNECT_TWO_DEVICES,
        vec![enabled as u8],
    )?;

    // Send reboot command for dual-device setting to take effect
    let _ = updates.send(UiUpdate::DeviceRebooting);
    let _ = updates.send(UiUpdate::Status(crate::t!("device-rebooting")));
    thread::sleep(Duration::from_millis(50));
    let _ = send_command(stream, EarFunCommands::REBOOT, vec![]);
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(enabled)
}

pub fn read_earfun_paired_devices<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    local_lap: Option<[u8; 3]>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<PairedDevice>, String> {
    send_command(stream, EarFunCommands::GET_CONNECTED_DEVICES, vec![])?;

    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut slots_received = std::collections::BTreeMap::new();

    while Instant::now() < deadline {
        let mut bytes = [0u8; 1024];
        let n = match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(n) => n,
            Err(e) if is_socket_timeout(&e) => {
                if !slots_received.is_empty() {
                    break;
                }
                continue;
            }
            Err(e) => return Err(crate::t!("protocol-response-read-failed", e)),
        };

        let packets = decoder.push(&bytes[..n]);
        for packet in packets {
            if packet.command == EarFunCommands::GET_CONNECTED_DEVICES
                || packet.command == (EarFunCommands::GET_CONNECTED_DEVICES | 0x8000)
            {
                if packet.payload.len() >= 27 {
                    let slot = packet.payload[1];
                    slots_received.insert(slot, packet.payload);
                }
            } else {
                handle_unsolicited(&packet, updates);
            }
        }

        if slots_received.len() >= 10 {
            break;
        }
    }

    if slots_received.is_empty() {
        return Err(crate::t!("dual-device-read-list-failed", "timeout"));
    }

    let mut devices = Vec::new();
    for (_, payload) in slots_received {
        if let Ok(Some(dev)) = parse_earfun_paired_device_entry(&payload, local_lap) {
            devices.push(dev);
        }
    }
    devices.sort_by_key(|d| (!d.connected, !d.is_current_host));

    Ok(devices)
}
