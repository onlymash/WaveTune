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

//! EarFun GAIA protocol constants and command IDs.

pub struct EarFunCommands;

impl EarFunCommands {
    pub const VENDOR_GAIA_LEGACY: u16 = 0x000A;
    pub const VENDOR_GAIA_V3: u16 = 0x001D;

    pub const DEFAULT_REQUEST_VERSION: u8 = 0x04;
    pub const DEFAULT_RESPONSE_VERSION: u8 = 0x03;

    pub const SOP: u8 = 0xFF;
    pub const FLAGS_NONE: u8 = 0x00;

    // GAIA API & Version
    pub const GAIA_API_VERSION: u16 = 0x0300;
    pub const APPLICATION_VERSION: u16 = 0x0304;
    pub const FACTORY_RESET: u16 = 0x0305;
    pub const FIRMWARE_VERSION: u16 = 0x0309;
    pub const REBOOT: u16 = 0x0308;
    pub const DEVICE_NAME: u16 = 0x0316;
    pub const DEVICE_COLOR: u16 = 0x0318;

    // Battery
    pub const BATTERY_LEFT: u16 = 0x0306;
    pub const BATTERY_RIGHT: u16 = 0x0307;
    pub const BATTERY_CASE: u16 = 0x0317;

    // Noise Control / Ambient Sound
    pub const SET_AMBIENT_SOUND: u16 = 0x0314;
    pub const GET_AMBIENT_SOUND: u16 = 0x0315;
    pub const SET_ANC_SUB_MODE: u16 = 0x033A;
    pub const GET_ANC_SUB_MODE: u16 = 0x033B;
    pub const SET_TRANSPARENCY_SUB_MODE: u16 = 0x033C;
    pub const GET_TRANSPARENCY_SUB_MODE: u16 = 0x033D;

    // Low Latency / Game Mode
    pub const SET_GAME_MODE: u16 = 0x0312;
    pub const GET_GAME_MODE: u16 = 0x0313;

    // Dual Device / Multipoint
    pub const GET_CONNECT_TWO_DEVICES: u16 = 0x0326;
    pub const SET_CONNECT_TWO_DEVICES: u16 = 0x0327;
    pub const CONNECT_DEVICE: u16 = 0x0329;
    pub const DISCONNECT_DEVICE: u16 = 0x032A;
    pub const TRIGGER_PAIRING: u16 = 0x032B;
    pub const GET_CONNECTED_DEVICES: u16 = 0x032C;

    // Codec & Audio
    pub const SET_AUDIO_CODEC: u16 = 0x032E;
    pub const GET_AUDIO_CODEC: u16 = 0x032F;
    pub const SET_ADVANCED_AUDIO_MODE: u16 = 0x034B;
    pub const GET_ADVANCED_AUDIO_MODE: u16 = 0x034C;
    pub const GET_AUDIO_MODE: u16 = 0x034D;

    // Hardware & Sensors
    pub const SET_IN_EAR_DETECTION: u16 = 0x0349;
    pub const GET_IN_EAR_DETECTION: u16 = 0x034A;
    pub const SET_MICROPHONE_MODE: u16 = 0x0330;
    pub const GET_MICROPHONE_MODE: u16 = 0x0331;
    pub const SET_FIND_DEVICE: u16 = 0x0332;
    pub const GET_FIND_DEVICE: u16 = 0x0333;
    pub const SET_VOICE_PROMPT_VOLUME: u16 = 0x0338;
    pub const GET_VOICE_PROMPT_VOLUME: u16 = 0x0339;

    // Touch
    pub const SET_TOUCH_ACTION: u16 = 0x030A;
    pub const GET_TOUCH_ACTION: u16 = 0x030B;
    pub const SET_TOUCH_MODE: u16 = 0x0334;
    pub const GET_TOUCH_MODE: u16 = 0x0335;

    // Equalizer (GAIA V3 vendor 0x001D)
    pub const SET_EQUALIZER_BAND: u16 = 0x0E01;
    pub const GET_EQUALIZER_CONFIG: u16 = 0x0E02;
    pub const ACK_EQUALIZER_BAND: u16 = 0x0F81;

    // Playback state
    pub const GET_PLAY_STATE: u16 = 0x0350;
    pub const SET_CONTROL_PLAY: u16 = 0x0351;
}
