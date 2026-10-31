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
    EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, drain_stream, send_command,
};
use crate::protocol::brands::earfun::{
    EarFunAudioCodec, EarFunCommands, EarFunFrameDecoder, EarFunPacket, encode_earfun_eq_band,
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
    let mut joined_frames = Vec::new();

    if product_id == EARFUN_AIR_S_PRODUCT_ID {
        // 6-band EQ for EarFun Air S (with 4 dummy bands)
        let frequencies: [u16; 10] = [
            0x00BD, 0x021C, 0x05DC, 0x0BB8, 0x5DC0, 0xAFC8, // 63, 180, 500, 1000, 8000, 15000
            0x0063, 0x0258, 0x1770, 0xAFC8, // dummy 7..10
        ];
        let q_factors: [u16; 10] = [
            0x0B33, 0x0999, 0x0999, 0x0999, 0x0999, 0x0999, 0x0800, 0x0999, 0x0E66, 0x0999,
        ];
        let dummy_gains = [4.5, -4.5, 1.5, 3.5];

        let gains_6: [f64; 6] = match preset_index {
            0 => [0.0; 6],                          // Classic
            1 => [0.0, 0.0, 1.0, 1.0, 2.0, 3.0],    // Natural
            2 => [8.0, 3.0, 2.0, 0.0, 0.0, 0.0],    // Bass Boost
            3 => [0.0, 0.0, 0.0, 2.0, 3.0, 5.0],    // Treble Boost
            4 => [-5.0, -2.0, 2.0, 3.0, 0.0, -3.0], // Soft
            5 => [7.0, 3.0, 2.0, 3.0, 5.0, 7.0],    // Dynamic
            6 => [2.0, 0.0, 3.0, 5.0, 3.0, 5.0],    // Clear
            7 => [2.0, 1.0, 0.0, -1.0, -3.0, -5.0], // Relaxed
            8 => [-2.0, 0.0, 4.0, 5.0, 2.0, -1.0],  // Vocal
            _ => [0.0; 6],
        };

        for i in 0..10 {
            let band_id = 0xA1 + i as u8;
            let gain = if i < 6 {
                gains_6[i]
            } else {
                dummy_gains[i - 6]
            };
            let payload = encode_earfun_eq_band(band_id, frequencies[i], q_factors[i], gain);
            let pkt = EarFunPacket::new_v3(EarFunCommands::SET_EQUALIZER_BAND, payload);
            joined_frames.extend_from_slice(&pkt.encode());
        }
    } else {
        // 10 bands for Air Pro 4 & Free Pro 3
        let frequencies: [u16; 10] = [
            0x005E, 0x00BD, 0x0177, 0x02EE, 0x05DC, 0x0BB8, 0x1770, 0x2EE0, 0x5DC0, 0xBB80,
        ];
        let q_factor: u16 = 0x0B33;

        let gains: [f64; 10] = match preset_index {
            0 => [0.0; 10],                                                // Classic / Flat
            1 => [0.0, 0.0, 1.0, 2.0, 2.0, -1.0, -1.0, -1.0, -2.0, 1.0],   // Natural
            2 => [8.0, 6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],       // Bass Boost
            3 => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 3.0, 4.0],       // Clear Treble
            4 => [-5.0, -4.0, -2.0, 0.0, 2.0, 3.0, 0.0, -2.0, -3.0, -5.0], // Soft
            5 => [6.0, 6.0, 4.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],       // Dynamic
            6 => [3.0, 2.0, 2.0, 2.0, 3.0, 5.0, 3.0, 3.0, 4.0, 5.0],       // Clear
            7 => [2.0, 1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -3.0, -4.0, -5.0],  // Relaxed
            8 => [-3.0, -2.0, 0.0, 2.0, 3.0, 5.0, 3.0, 2.0, 0.0, -1.0],    // Pure Vocal
            _ => [0.0; 10],
        };

        for i in 0..10 {
            let band_id = 0xA1 + i as u8;
            let payload = encode_earfun_eq_band(band_id, frequencies[i], q_factor, gains[i]);
            let pkt = EarFunPacket::new_v3(EarFunCommands::SET_EQUALIZER_BAND, payload);
            joined_frames.extend_from_slice(&pkt.encode());
        }
    }

    stream
        .write_all(&joined_frames)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    // Drain incoming ACK frames from socket buffer
    drain_stream(stream, decoder, updates, Duration::from_millis(150));

    Ok(())
}
