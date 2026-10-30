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

//! OPPO/OnePlus/realme protocol command IDs.

pub struct OppoCommands;

impl OppoCommands {
    // Queries (0x01xx)
    pub const HANDSHAKE: u16 = 0x0100;
    pub const QUERY_MTU: u16 = 0x0101;
    pub const QUERY_VID: u16 = 0x0102;
    pub const QUERY_PRODUCT_ID: u16 = 0x0103;
    pub const FIRMWARE_VERSIONS: u16 = 0x0105;
    pub const BATTERY_LEVEL: u16 = 0x0106;
    pub const QUERY_UPGRADE_CAPABILITY: u16 = 0x0107;
    pub const TOUCH_CONTROLS: u16 = 0x0108;
    pub const WEAR_STATE: u16 = 0x0109;
    pub const COLOR_ID: u16 = 0x010b;
    pub const NOISE_CONTROL: u16 = 0x010c;
    pub const FEATURE_STATUS: u16 = 0x010d;
    pub const EQUALIZER: u16 = 0x010f;
    pub const PAIRED_DEVICES: u16 = 0x0112;
    pub const QUERY_CODEC: u16 = 0x0114;
    pub const QUERY_CUSTOM_EQUALIZER: u16 = 0x0122;
    pub const BASS_ENGINE: u16 = 0x0124;
    pub const SPATIAL_AUDIO: u16 = 0x012a;
    pub const QUERY_GAME_SOUND: u16 = 0x012b;
    pub const PROMPT_VOLUME: u16 = 0x0130;
    pub const PRIORITY_DEVICE: u16 = 0x0132;
    pub const HEAD_MOTION_TYPE: u16 = 0x0134;

    // Events (0x02xx)
    pub const QUERY_EVENT_CODES: u16 = 0x0200;
    pub const EVENT_CODES_ALT_RESPONSE: u16 = 0x8203;
    pub const DEVICE_EVENT: u16 = 0x0204;
    pub const SUBSCRIBE_EVENTS: u16 = 0x0205;

    // Settings (0x04xx)
    pub const FIND_EARBUDS: u16 = 0x0400;
    pub const SET_KEY_FUNCTION: u16 = 0x0401;
    pub const FREE_MUSIC_REQUEST: u16 = 0x0402;
    pub const SET_FEATURE: u16 = 0x0403;
    pub const SET_NOISE_CONTROL: u16 = 0x0404;
    pub const FIT_TEST: u16 = 0x0405;
    pub const SET_EQUALIZER: u16 = 0x0406;
    pub const SET_CUSTOM_EQUALIZER: u16 = 0x0418;
    pub const SET_HOST_CAPABILITIES: u16 = 0x0419;
    pub const SET_BASS_ENGINE: u16 = 0x041b;
    pub const SET_SPATIAL_AUDIO: u16 = 0x0422;
    pub const SET_GAME_SOUND: u16 = 0x0423;
    pub const SET_PROMPT_VOLUME: u16 = 0x0427;
    pub const OPERATE_MULTI_CONNECT: u16 = 0x0429;
    pub const SET_HEAD_MOTION_TYPE: u16 = 0x0431;

    // Device-originated & Notifications (0x05xx)
    pub const TIME_REQUEST: u16 = 0x0500;
    pub const FEATURE_CHANGED_NOTIFICATION: u16 = 0x0503;
    pub const EQUALIZER_CHANGED_NOTIFICATION: u16 = 0x0504;
    pub const NOTIFY_CUSTOM_EQUALIZER: u16 = 0x0506;
    pub const BASS_ENGINE_CHANGED_NOTIFICATION: u16 = 0x0508;
    pub const GAME_EQ_REQUEST: u16 = 0x050c;
    pub const SPATIAL_AUDIO_NOTIFICATION: u16 = 0x0510;
    pub const APP_CAPABILITIES_REQUEST: u16 = 0x051c;
    pub const PHONE_SPATIAL_REQUEST: u16 = 0x051d;

    // Event codes in first payload byte of DEVICE_EVENT (0x0204)
    pub const EVENT_BATTERY: u8 = 0x01;
    pub const EVENT_WEAR_STATE: u8 = 0x02;
    pub const EVENT_NOISE_CONTROL: u8 = 0x03;
    pub const EVENT_FIT_TEST: u8 = 0x04;
    pub const EVENT_CONNECTED_DEVICES: u8 = 0x06;
    pub const EVENT_CONNECTION_HISTORY: u8 = 0xf2;
    pub const EVENT_TELEMETRY: u8 = 0xf4;
}
