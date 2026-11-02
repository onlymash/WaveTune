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

//! EarFun device utility and system audio setting codecs.

use super::earfun_packet::EarFunProtocolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EarFunFindMode {
    Off = 0,
    Left = 1,
    Right = 2,
    Both = 3,
}

impl EarFunFindMode {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Off),
            1 => Some(Self::Left),
            2 => Some(Self::Right),
            3 => Some(Self::Both),
            _ => None,
        }
    }

    pub const fn is_finding(self) -> bool {
        !matches!(self, Self::Off)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EarFunMicrophoneMode {
    Auto = 0,
    Left = 1,
    Right = 2,
}

impl EarFunMicrophoneMode {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Auto),
            1 => Some(Self::Left),
            2 => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EarFunAdvancedAudioMode {
    GoogleFastPair = 0,
    LeAudio = 1,
}

impl EarFunAdvancedAudioMode {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::GoogleFastPair),
            1 => Some(Self::LeAudio),
            _ => None,
        }
    }
}

fn response_value(payload: &[u8]) -> Result<u8, EarFunProtocolError> {
    if payload.len() < 2 || payload[0] != 0 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(payload[1])
}

pub fn parse_earfun_find_mode(payload: &[u8]) -> Result<EarFunFindMode, EarFunProtocolError> {
    EarFunFindMode::from_u8(response_value(payload)?)
        .ok_or(EarFunProtocolError::InvalidPacketLength)
}

pub fn encode_earfun_set_find_mode(mode: EarFunFindMode) -> Vec<u8> {
    vec![mode as u8]
}

pub fn parse_earfun_microphone_mode(
    payload: &[u8],
) -> Result<EarFunMicrophoneMode, EarFunProtocolError> {
    EarFunMicrophoneMode::from_u8(response_value(payload)?)
        .ok_or(EarFunProtocolError::InvalidPacketLength)
}

pub fn encode_earfun_set_microphone_mode(mode: EarFunMicrophoneMode) -> Vec<u8> {
    vec![mode as u8]
}

/// Parse a volume value in the user-facing range 0 (off) through 4 (maximum).
pub fn parse_earfun_voice_prompt_volume(payload: &[u8]) -> Result<u8, EarFunProtocolError> {
    let wire_value = response_value(payload)?;
    if wire_value > 4 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(4 - wire_value)
}

/// Encode a user-facing volume value in the range 0 (off) through 4 (maximum).
pub fn encode_earfun_set_voice_prompt_volume(volume: u8) -> Result<Vec<u8>, EarFunProtocolError> {
    if volume > 4 {
        return Err(EarFunProtocolError::InvalidPacketLength);
    }
    Ok(vec![4 - volume])
}

pub fn parse_earfun_advanced_audio_mode(
    payload: &[u8],
) -> Result<EarFunAdvancedAudioMode, EarFunProtocolError> {
    EarFunAdvancedAudioMode::from_u8(response_value(payload)?)
        .ok_or(EarFunProtocolError::InvalidPacketLength)
}

pub fn encode_earfun_set_advanced_audio_mode(mode: EarFunAdvancedAudioMode) -> Vec<u8> {
    vec![mode as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_mode_payloads_are_strict() {
        for value in 0..=3 {
            assert_eq!(parse_earfun_find_mode(&[0, value]).unwrap() as u8, value);
            assert_eq!(
                encode_earfun_set_find_mode(EarFunFindMode::from_u8(value).unwrap()),
                [value]
            );
        }
        assert!(parse_earfun_find_mode(&[0, 4]).is_err());
        assert!(parse_earfun_find_mode(&[1, 0]).is_err());
        assert!(parse_earfun_find_mode(&[0]).is_err());
    }

    #[test]
    fn microphone_modes_are_strict() {
        for value in 0..=2 {
            let mode = EarFunMicrophoneMode::from_u8(value).unwrap();
            assert_eq!(parse_earfun_microphone_mode(&[0, value]).unwrap(), mode);
            assert_eq!(encode_earfun_set_microphone_mode(mode), [value]);
        }
        assert!(parse_earfun_microphone_mode(&[0, 3]).is_err());
        assert!(parse_earfun_microphone_mode(&[1, 0]).is_err());
        assert!(parse_earfun_microphone_mode(&[0]).is_err());
    }

    #[test]
    fn prompt_volume_reverses_wire_range() {
        for ui_value in 0..=4 {
            let payload = encode_earfun_set_voice_prompt_volume(ui_value).unwrap();
            assert_eq!(payload, [4 - ui_value]);
            assert_eq!(
                parse_earfun_voice_prompt_volume(&[0, payload[0]]).unwrap(),
                ui_value
            );
        }
        assert!(encode_earfun_set_voice_prompt_volume(5).is_err());
        assert!(parse_earfun_voice_prompt_volume(&[0, 5]).is_err());
        assert!(parse_earfun_voice_prompt_volume(&[1, 0]).is_err());
    }

    #[test]
    fn advanced_audio_modes_encode_and_parse_without_enabling_runtime_support() {
        for (value, mode) in [
            (0, EarFunAdvancedAudioMode::GoogleFastPair),
            (1, EarFunAdvancedAudioMode::LeAudio),
        ] {
            assert_eq!(parse_earfun_advanced_audio_mode(&[0, value]).unwrap(), mode);
            assert_eq!(encode_earfun_set_advanced_audio_mode(mode), [value]);
        }
        assert!(parse_earfun_advanced_audio_mode(&[0, 2]).is_err());
    }
}
