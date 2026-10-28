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
    pub const FIRMWARE_VERSIONS: u16 = 0x0101;
    pub const CAPABILITIES: u16 = 0x0102;
    pub const BATTERY_LEVEL: u16 = 0x0103;
    pub const WEAR_STATE: u16 = 0x0104;
    pub const COLOR_ID: u16 = 0x0105;
    pub const TOUCH_CONTROLS: u16 = 0x0108;
    pub const NOISE_CONTROL: u16 = 0x010c;
    pub const PAIRED_DEVICES: u16 = 0x0112;
    pub const DUAL_DEVICE_FEATURE: u16 = 0x0113;
    pub const EQUALIZER: u16 = 0x0114;
    pub const SPATIAL_AUDIO: u16 = 0x0116;
    pub const GAME_MODE: u16 = 0x0118;
    pub const BASS_ENGINE: u16 = 0x011b;
    pub const HI_RES: u16 = 0x011c;
    pub const QUERY_CUSTOM_EQUALIZER: u16 = 0x0122;
    pub const PROMPT_VOLUME: u16 = 0x0130;
    pub const PRIORITY_DEVICE: u16 = 0x0132;
    pub const HEAD_MOTION_TYPE: u16 = 0x0134;

    pub const FIND_EARBUDS: u16 = 0x0400;
    pub const SET_KEY_FUNCTION: u16 = 0x0401;
    pub const SET_FEATURE: u16 = 0x0403;
    pub const SET_NOISE_CONTROL: u16 = 0x0404;
    pub const FIT_TEST: u16 = 0x0405;
    pub const SET_CUSTOM_EQUALIZER: u16 = 0x0418;
    pub const SET_PROMPT_VOLUME: u16 = 0x0427;
    pub const OPERATE_MULTI_CONNECT: u16 = 0x0429;
    pub const SET_HEAD_MOTION_TYPE: u16 = 0x0431;

    pub const NOTIFY_CUSTOM_EQUALIZER: u16 = 0x0506;

    pub const EVENT_FIT_TEST: u8 = 0x04;
}
