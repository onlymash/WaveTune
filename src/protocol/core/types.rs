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

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryPart {
    pub percentage: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatteryState {
    pub left: Option<BatteryPart>,
    pub right: Option<BatteryPart>,
    pub case: Option<BatteryPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BassEngineLevel {
    pub min: i8,
    pub max: i8,
    pub current: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FirmwareVersions {
    pub left: Option<String>,
    pub right: Option<String>,
    pub case: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WearPart {
    pub out_of_case: bool,
    pub in_ear: bool,
    pub raw_flags: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WearState {
    pub left: Option<WearPart>,
    pub right: Option<WearPart>,
    pub case_open: Option<bool>,
}

impl WearState {
    pub fn any_earbud_in_ear(&self) -> bool {
        self.left.is_some_and(|part| part.in_ear) || self.right.is_some_and(|part| part.in_ear)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedDevice {
    pub name: String,
    pub connected: bool,
    pub is_current_host: bool,
    pub device_type: u8,
    pub address: PairedDeviceAddress,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PairedDeviceAddress([u8; 6]);

impl PairedDeviceAddress {
    pub fn from_display_bytes(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub fn command_bytes(self) -> [u8; 6] {
        self.0
    }

    pub fn format_mac(&self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Debug for PairedDeviceAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedDeviceAction {
    Connect,
    Disconnect,
    Remove,
}

impl PairedDeviceAction {
    pub fn payload(self, address: PairedDeviceAddress) -> Vec<u8> {
        let action = match self {
            Self::Connect => 1,
            Self::Disconnect => 2,
            Self::Remove => 3,
        };
        let mut payload = Vec::with_capacity(7);
        payload.push(action);
        payload.extend_from_slice(&address.command_bytes());
        payload
    }

    pub fn expected_connected(self) -> Option<bool> {
        match self {
            Self::Connect => Some(true),
            Self::Disconnect => Some(false),
            Self::Remove => None,
        }
    }
}

pub const DEFAULT_EQ_FREQUENCIES: [u16; 10] =
    [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
pub const DEFAULT_MIN_GAIN: i8 = -6;
pub const DEFAULT_MAX_GAIN: i8 = 6;
pub const MAX_CUSTOM_PRESETS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomEqualizer {
    pub id: u8,
    pub name: String,
    pub min_gain: i8,
    pub max_gain: i8,
    pub is_selected: bool,
    pub bands: Vec<(u16, i8)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EarSide {
    Left = 1,
    Right = 2,
    Both = 4,
}

impl EarSide {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Left),
            2 => Some(Self::Right),
            4 => Some(Self::Both),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureAction {
    SingleSqueeze = 1,
    DoubleSqueeze = 2,
    TripleSqueeze = 3,
    SqueezeAndHold = 4,
    Slide = 5,
    LongHold = 6,
}

impl GestureAction {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::SingleSqueeze),
            2 => Some(Self::DoubleSqueeze),
            3 => Some(Self::TripleSqueeze),
            4 => Some(Self::SqueezeAndHold),
            5 => Some(Self::Slide),
            6 => Some(Self::LongHold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyFunction {
    pub side: EarSide,
    pub button: u8,
    pub action: GestureAction,
    pub function: u8,
}

impl KeyFunction {
    pub const CALL_BUTTON: u8 = 6;

    pub fn is_call_control(&self) -> bool {
        self.button == Self::CALL_BUTTON
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoiseCycleState {
    pub left_mask: u32,
    pub right_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitQuality {
    Unknown,
    Good,
    Average,
    Poor,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitTestResult {
    pub left: FitQuality,
    pub right: FitQuality,
}
