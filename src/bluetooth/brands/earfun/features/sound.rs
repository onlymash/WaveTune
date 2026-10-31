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
    time::Duration,
};

use crate::app::events::UiUpdate;
use crate::bluetooth::brands::earfun::transport::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, drain_stream, send_command,
};
use crate::compatibility::brands::earfun::equalizer_bands_for_preset;
use crate::protocol::brands::earfun::{
    EarFunAudioCodec, EarFunCommands, EarFunFrameDecoder, EarFunPacket, GaiaEqBand,
    encode_earfun_eq_band,
};

pub fn set_game_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    send_command(stream, EarFunCommands::SET_GAME_MODE, vec![enabled as u8])?;
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(enabled)
}

pub fn set_audio_codec<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    target_codec: u8,
    dual_device_enabled: Option<bool>,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    // If setting LDAC on EarFun Air Pro 4 and dual device is currently ON,
    // turn OFF dual device first!
    if product_id == EARFUN_AIR_PRO_4_PRODUCT_ID
        && target_codec == EarFunAudioCodec::Ldac.as_u8()
        && dual_device_enabled.unwrap_or(false)
    {
        let _ = send_command(stream, EarFunCommands::SET_CONNECT_TWO_DEVICES, vec![0x00]);
        let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(false));
        thread::sleep(Duration::from_millis(50));
    }

    send_command(stream, EarFunCommands::SET_AUDIO_CODEC, vec![target_codec])?;

    // Changing audio codec requires rebooting the earbuds
    let _ = updates.send(UiUpdate::DeviceRebooting);
    let _ = updates.send(UiUpdate::Status(crate::t!("device-rebooting")));
    thread::sleep(Duration::from_millis(50));
    let _ = send_command(stream, EarFunCommands::REBOOT, vec![]);
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(target_codec)
}

pub fn apply_equalizer_preset<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    preset_index: u8,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(), String> {
    let bands = equalizer_bands_for_preset(product_id, preset_index)
        .ok_or_else(|| crate::t!("equalizer-blocked"))?;
    send_equalizer_bands(stream, decoder, &bands, updates)
}

pub fn send_equalizer_bands<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    bands: &[GaiaEqBand],
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(), String> {
    let mut joined_frames = Vec::with_capacity(bands.len() * 17);

    for band in bands {
        let payload = encode_earfun_eq_band(band);
        let pkt = EarFunPacket::new_v3(EarFunCommands::SET_EQUALIZER_BAND, payload);
        joined_frames.extend_from_slice(&pkt.encode());
    }

    stream
        .write_all(&joined_frames)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    // Drain incoming ACK frames from socket buffer
    drain_stream(stream, decoder, updates, Duration::from_millis(150));

    Ok(())
}
