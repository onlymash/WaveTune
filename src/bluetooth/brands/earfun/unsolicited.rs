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

use std::sync::mpsc::Sender;

use crate::app::events::UiUpdate;
use crate::protocol::BatteryState;
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunPacket, parse_earfun_battery_component, parse_earfun_game_mode,
};

pub fn handle_unsolicited(packet: &EarFunPacket, updates: Option<&Sender<UiUpdate>>) {
    let Some(updates) = updates else { return };
    match packet.command {
        EarFunCommands::BATTERY_LEFT => {
            if let Ok(Some(part)) = parse_earfun_battery_component(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(BatteryState {
                    left: Some(part),
                    right: None,
                    case: None,
                }));
            }
        }
        EarFunCommands::BATTERY_RIGHT => {
            if let Ok(Some(part)) = parse_earfun_battery_component(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(BatteryState {
                    left: None,
                    right: Some(part),
                    case: None,
                }));
            }
        }
        EarFunCommands::BATTERY_CASE => {
            if let Ok(part) = parse_earfun_battery_component(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(BatteryState {
                    left: None,
                    right: None,
                    case: part,
                }));
            }
        }
        EarFunCommands::GET_GAME_MODE => {
            if let Ok(enabled) = parse_earfun_game_mode(&packet.payload) {
                let _ = updates.send(UiUpdate::GameMode(enabled));
            }
        }
        _ => {}
    }
}

