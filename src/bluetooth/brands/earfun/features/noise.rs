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
use crate::bluetooth::brands::earfun::transport::{drain_stream, send_command};
use crate::compatibility;
use crate::protocol::brands::earfun::{EarFunCommands, EarFunFrameDecoder};

pub fn earfun_mode_to_protocol_index(master: u8, anc_sub: u8, transp_sub: u8) -> u8 {
    match master {
        0 => 0, // Off
        1 => match anc_sub {
            0 => 7, // AI Ear Adaptive
            1 => 6, // Wind Noise Cancelling
            2 => 4, // Strong ANC
            3 => 5, // Balanced ANC
            4 => 3, // AI Environment Adaptive
            _ => 1, // ANC parent
        },
        2 => match transp_sub {
            0 => 8, // Default
            1 => 9, // Vocal / Natural
            _ => 2, // Transparency parent
        },
        _ => 0,
    }
}

pub fn protocol_index_to_earfun_mode(protocol_index: u8) -> (u8, Option<u8>, bool) {
    match protocol_index {
        0 => (0, None, false),
        1 => (1, None, true),
        2 => (2, None, false),
        3 => (1, Some(4), true),
        4 => (1, Some(2), true),
        5 => (1, Some(3), true),
        6 => (1, Some(1), true),
        7 => (1, Some(0), true),
        8 => (2, Some(0), false),
        9 => (2, Some(1), false),
        _ => (0, None, false),
    }
}

pub fn set_noise_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    protocol_index: u8,
    last_anc_sub: &mut u8,
    last_transp_sub: &mut u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    let profile = compatibility::profile_for_product_id(product_id);
    let has_submodes = profile
        .map(|p| p.noise_modes.iter().any(|m| m.parent.is_some()))
        .unwrap_or(false);

    if !has_submodes {
        send_command(
            stream,
            EarFunCommands::SET_AMBIENT_SOUND,
            vec![protocol_index],
        )?;
        drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
        return Ok(protocol_index);
    }

    let confirmed_index = match protocol_index {
        0 => {
            // Off: set master mode to 0
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![0])?;
            0
        }
        1 => {
            // ANC primary button clicked: activate master 1 + last ANC submode
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![1])?;
            thread::sleep(Duration::from_millis(20));
            send_command(
                stream,
                EarFunCommands::SET_ANC_SUB_MODE,
                vec![*last_anc_sub],
            )?;
            earfun_mode_to_protocol_index(1, *last_anc_sub, *last_transp_sub)
        }
        2 => {
            // Transparency primary button clicked: activate master 2 + last Transparency submode
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![2])?;
            thread::sleep(Duration::from_millis(20));
            send_command(
                stream,
                EarFunCommands::SET_TRANSPARENCY_SUB_MODE,
                vec![*last_transp_sub],
            )?;
            earfun_mode_to_protocol_index(2, *last_anc_sub, *last_transp_sub)
        }
        sub_index => {
            // Specific submode selected from dropdown
            let (target_master, target_sub, is_anc) = protocol_index_to_earfun_mode(sub_index);
            send_command(
                stream,
                EarFunCommands::SET_AMBIENT_SOUND,
                vec![target_master],
            )?;
            if let Some(sub) = target_sub {
                thread::sleep(Duration::from_millis(20));
                if is_anc {
                    *last_anc_sub = sub;
                    send_command(stream, EarFunCommands::SET_ANC_SUB_MODE, vec![sub])?;
                } else {
                    *last_transp_sub = sub;
                    send_command(stream, EarFunCommands::SET_TRANSPARENCY_SUB_MODE, vec![sub])?;
                }
            }
            sub_index
        }
    };

    drain_stream(stream, decoder, Some(updates), Duration::from_millis(100));
    Ok(confirmed_index)
}
