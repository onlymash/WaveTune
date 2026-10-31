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
    time::Duration,
};

use crate::app::events::UiUpdate;
use crate::bluetooth::brands::earfun::transport::{drain_stream, request, send_command};
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunFrameDecoder, encode_earfun_set_gesture, encode_earfun_set_touch_lock,
    parse_earfun_touch_gestures_response, parse_earfun_touch_lock_response,
};
use crate::protocol::core::types::{EarSide, GestureAction, KeyFunction};

pub fn set_in_ear_detection<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    let val = if enabled { 0 } else { 1 };
    send_command(stream, EarFunCommands::SET_IN_EAR_DETECTION, vec![val])?;
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(enabled)
}

pub fn set_touch_action<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    side: EarSide,
    action: GestureAction,
    function: u8,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(), String> {
    let payload = encode_earfun_set_gesture(side, action, function).map_err(|e| e.to_string())?;
    send_command(stream, EarFunCommands::SET_TOUCH_ACTION, payload)?;
    drain_stream(stream, decoder, updates, Duration::from_millis(50));
    Ok(())
}

pub fn query_touch_actions<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<KeyFunction>, String> {
    let pkt = request(
        stream,
        decoder,
        EarFunCommands::GET_TOUCH_ACTION,
        vec![],
        updates,
    )?;
    parse_earfun_touch_gestures_response(&pkt.payload).map_err(|e| e.to_string())
}

pub fn set_touch_lock<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    locked: bool,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<bool, String> {
    let payload = encode_earfun_set_touch_lock(locked);
    send_command(stream, EarFunCommands::SET_TOUCH_MODE, payload)?;
    drain_stream(stream, decoder, updates, Duration::from_millis(50));
    Ok(locked)
}

pub fn query_touch_lock<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<bool, String> {
    let pkt = request(
        stream,
        decoder,
        EarFunCommands::GET_TOUCH_MODE,
        vec![],
        updates,
    )?;
    parse_earfun_touch_lock_response(&pkt.payload).map_err(|e| e.to_string())
}
