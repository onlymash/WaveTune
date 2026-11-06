// SPDX-License-Identifier: GPL-3.0-only

pub struct EarFunEfCommands;

impl EarFunEfCommands {
    pub const FIRMWARE: u16 = 0x1010;
    pub const BATTERY: u16 = 0x1014;
    pub const CASE_BATTERY: u16 = 0x1018;
    pub const CHARGING: u16 = 0x1016;
    pub const CASE_CHARGING: u16 = 0x1020;
    pub const NOISE: (u16, u16) = (0x2080, 0x2085);
    pub const GAME: (u16, u16) = (0x2030, 0x2035);
    pub const DUAL_DEVICE: (u16, u16) = (0x2140, 0x2145);
    pub const PROMPT_VOLUME: (u16, u16) = (0x2120, 0x2125);
    pub const FIND_DEVICE: (u16, u16) = (0x2050, 0x2055);
    pub const SPATIAL_AUDIO: (u16, u16) = (0x2190, 0x2195);
    pub const ANC_SUB_MODE: (u16, u16) = (0x2310, 0x2315);
    pub const TRANSPARENCY_SUB_MODE: (u16, u16) = (0x2300, 0x2305);
    pub const AUTO_OFF: (u16, u16) = (0x2130, 0x2135);
    pub const MICROPHONE: (u16, u16) = (0x2200, 0x2205);
    pub const QUALITY: (u16, u16) = (0x2100, 0x2105);
    pub const TOUCH_LOCK: (u16, u16) = (0x2060, 0x2065);
    pub const TOUCH_ACTIONS: (u16, u16) = (0x2070, 0x2075);
    pub const PAIRED_DEVICES: u16 = 0x2150;
    pub const CONNECT_DEVICE: u16 = 0x2165;
    pub const DISCONNECT_DEVICE: u16 = 0x2175;
    pub const EQ_BAND: u16 = 0x2095;
}
