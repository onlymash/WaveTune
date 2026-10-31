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

//! EarFun GAIA V3 touch controls, gestures, and touch lock codecs.

use super::earfun_packet::EarFunProtocolError;
use crate::protocol::core::types::{EarSide, GestureAction, KeyFunction};

/// EarFun GAIA V3 touch interaction types.
///
/// NOTE: In EarFun firmware, Long Press is 0x11 (17 decimal), not 0x04.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EarFunInteractionType {
    SingleTap = 0x01,
    DoubleTap = 0x02,
    TripleTap = 0x03,
    LongPress = 0x11,
}

impl EarFunInteractionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::SingleTap),
            0x02 => Some(Self::DoubleTap),
            0x03 => Some(Self::TripleTap),
            0x11 => Some(Self::LongPress),
            _ => None,
        }
    }

    pub fn to_gesture_action(self) -> GestureAction {
        match self {
            Self::SingleTap => GestureAction::SingleSqueeze,
            Self::DoubleTap => GestureAction::DoubleSqueeze,
            Self::TripleTap => GestureAction::TripleSqueeze,
            Self::LongPress => GestureAction::SqueezeAndHold,
        }
    }
}

impl TryFrom<u8> for EarFunInteractionType {
    type Error = EarFunProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(EarFunProtocolError::InvalidPacketLength)
    }
}

impl From<EarFunInteractionType> for GestureAction {
    fn from(interaction: EarFunInteractionType) -> Self {
        interaction.to_gesture_action()
    }
}

impl TryFrom<GestureAction> for EarFunInteractionType {
    type Error = EarFunProtocolError;

    fn try_from(action: GestureAction) -> Result<Self, Self::Error> {
        match action {
            GestureAction::SingleSqueeze => Ok(Self::SingleTap),
            GestureAction::DoubleSqueeze => Ok(Self::DoubleTap),
            GestureAction::TripleSqueeze => Ok(Self::TripleTap),
            GestureAction::SqueezeAndHold | GestureAction::LongHold => Ok(Self::LongPress),
            _ => Err(EarFunProtocolError::InvalidPacketLength),
        }
    }
}

/// Parse touch controls/gestures response (command 0x830B).
///
/// Payload layout (17+ bytes):
/// - Byte 0: Status code (`0x00` on success)
/// - Bytes 1..17: 8 big-endian 16-bit short integers:
///   1. Single Tap Left (`short & 0xFF`)
///   2. Single Tap Right (`short & 0xFF`)
///   3. Double Tap Left (`short & 0xFF`)
///   4. Double Tap Right (`short & 0xFF`)
///   5. Triple Tap Left (`short & 0xFF`)
///   6. Triple Tap Right (`short & 0xFF`)
///   7. Long Press Left (`short & 0xFF`)
///   8. Long Press Right (`short & 0xFF`)
pub fn parse_earfun_touch_gestures_response(
    payload: &[u8],
) -> Result<Vec<KeyFunction>, EarFunProtocolError> {
    if payload.len() < 17 || (payload[0] != 0x00 && payload[0] != 0x01) {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }

    const SLOTS: [(EarSide, GestureAction); 8] = [
        (EarSide::Left, GestureAction::SingleSqueeze),
        (EarSide::Right, GestureAction::SingleSqueeze),
        (EarSide::Left, GestureAction::DoubleSqueeze),
        (EarSide::Right, GestureAction::DoubleSqueeze),
        (EarSide::Left, GestureAction::TripleSqueeze),
        (EarSide::Right, GestureAction::TripleSqueeze),
        (EarSide::Left, GestureAction::SqueezeAndHold),
        (EarSide::Right, GestureAction::SqueezeAndHold),
    ];

    let mut key_functions = Vec::with_capacity(8);
    for (i, &(side, action)) in SLOTS.iter().enumerate() {
        let offset = 1 + i * 2;
        // Big-endian short; effective action code is the low byte
        let function = payload[offset + 1];
        key_functions.push(KeyFunction {
            side,
            button: 1,
            action,
            function,
        });
    }

    Ok(key_functions)
}

/// Encode set touch action command payload (command 0x030A).
///
/// Returns 4 bytes:
/// - Left earbud: `[interaction_type, action, 0x00, 0x00]`
/// - Right earbud: `[0x00, 0x00, interaction_type, action]`
/// - Both earbuds: `[interaction_type, action, interaction_type, action]`
pub fn encode_earfun_set_touch_action(
    side: EarSide,
    interaction: EarFunInteractionType,
    action: u8,
) -> Vec<u8> {
    let it = interaction as u8;
    match side {
        EarSide::Left => vec![it, action, 0x00, 0x00],
        EarSide::Right => vec![0x00, 0x00, it, action],
        EarSide::Both => vec![it, action, it, action],
    }
}

/// Convenience function to encode set gesture from generic `GestureAction`.
pub fn encode_earfun_set_gesture(
    side: EarSide,
    action: GestureAction,
    function: u8,
) -> Result<Vec<u8>, EarFunProtocolError> {
    let interaction = EarFunInteractionType::try_from(action)?;
    Ok(encode_earfun_set_touch_action(side, interaction, function))
}

/// Parse touch lock mode response (command 0x8335).
///
/// Payload format: `[status (0x00), mode]`
/// - `0`: Touch enabled (unlocked)
/// - `1`: Touch disabled (locked)
///
/// Returns `true` if touch is locked/disabled, `false` if touch is enabled/unlocked.
pub fn parse_earfun_touch_lock_response(payload: &[u8]) -> Result<bool, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0x00 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1] != 0)
}

/// Encode set touch lock mode command payload (command 0x0334).
///
/// - `locked == true`: `[0x01]` (disable touch / lock)
/// - `locked == false`: `[0x00]` (enable touch / unlock)
pub fn encode_earfun_set_touch_lock(locked: bool) -> Vec<u8> {
    vec![if locked { 0x01 } else { 0x00 }]
}
