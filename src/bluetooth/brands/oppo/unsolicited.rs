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

use std::{io::Write, sync::mpsc::Sender};

use crate::bluetooth::UiUpdate;
use crate::protocol::{
    AppPacket, FEATURE_BASS_ENGINE, FEATURE_HI_RES, WearState, parse_bass_engine_event,
    parse_battery_event, parse_custom_equalizer_list, parse_equalizer_event, parse_fit_test_event,
    parse_noise_mode_event, parse_paired_devices_event, parse_spatial_audio_response,
    parse_wear_event,
};

use super::transport::reply_packet;

pub fn handle_oppo_unsolicited<T: Write>(
    stream: &mut T,
    packet: &AppPacket,
    updates: Option<&Sender<UiUpdate>>,
) {
    match packet.command {
        0x0500 => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            let mut payload = vec![0x00];
            payload.extend_from_slice(&now.to_le_bytes());
            let _ = reply_packet(stream, 0x0500, packet.sequence, payload);
            return;
        }
        0x051C => {
            let _ = reply_packet(stream, 0x051C, packet.sequence, vec![0x00, 0x00, 0x00]);
            return;
        }
        0x051D => {
            let _ = reply_packet(stream, 0x051D, packet.sequence, vec![0x01]);
            return;
        }
        0x050C => {
            let _ = reply_packet(stream, 0x050C, packet.sequence, vec![0x00, 0x01, 0x00]);
            return;
        }
        0x0402 => {
            let _ = reply_packet(stream, 0x0402, packet.sequence, vec![0x00]);
            return;
        }
        _ => {}
    }

    if packet.command == 0x0204 {
        match packet.payload.first() {
            Some(&0xF2) => {
                let _ = reply_packet(stream, 0x0204, packet.sequence, vec![0x00, 0xF2]);
                return;
            }
            Some(&0xF4) => {
                let mut cmd = String::new();
                if packet.payload.len() > 1 {
                    if let Ok(value) =
                        serde_json::from_slice::<serde_json::Value>(&packet.payload[1..])
                    {
                        if let Some(c) = value.get("cmd").and_then(|v| v.as_str()) {
                            cmd = c.to_string();
                        }
                    }
                }
                let reply_json = serde_json::json!({ "cmd": cmd }).to_string();
                let mut reply_payload = vec![0x00, 0xF4];
                reply_payload.extend_from_slice(reply_json.as_bytes());
                let _ = reply_packet(stream, 0x0204, packet.sequence, reply_payload);
                return;
            }
            _ => {}
        }
    }

    let Some(updates) = updates else {
        return;
    };

    if packet.command == 0x0506 {
        if let Ok(list) = parse_custom_equalizer_list(&packet.payload, false) {
            let _ = updates.send(UiUpdate::CustomEqualizers(list));
        }
        return;
    }
    if packet.command == 0x0504 {
        if let Ok(preset) = parse_equalizer_event(&packet.payload) {
            let _ = updates.send(UiUpdate::Equalizer(preset));
        }
        return;
    }
    if packet.command == 0x0508 {
        if let Ok(level) = parse_bass_engine_event(&packet.payload) {
            let _ = updates.send(UiUpdate::BassEngineLevel(level));
        }
        return;
    }
    if packet.command == 0x0503 {
        let pairs: Vec<(u8, u8)> = if packet.payload.len() >= 3
            && packet.payload.len() == 1 + (packet.payload[0] as usize) * 2
        {
            packet.payload[1..]
                .chunks_exact(2)
                .map(|c| (c[0], c[1]))
                .collect()
        } else if packet.payload.len() >= 2 {
            vec![(packet.payload[0], packet.payload[1])]
        } else {
            Vec::new()
        };
        for (feature_id, val) in pairs {
            if feature_id == 0x28 || feature_id == 0x06 {
                let _ = updates.send(UiUpdate::GameMode(val != 0));
            } else if feature_id == FEATURE_BASS_ENGINE {
                let _ = updates.send(UiUpdate::BassEngineEnabled(val != 0));
            } else if feature_id == FEATURE_HI_RES {
                let _ = updates.send(UiUpdate::HiRes(val != 0));
            } else if feature_id == 0x04 {
                let _ = updates.send(UiUpdate::InEarDetectionEnabled(val != 0));
            } else if feature_id == 17 {
                let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(val != 0));
                let _ = updates.send(UiUpdate::PairedDeviceControlsEnabled(Ok(val != 0)));
            } else if feature_id == 0x3a {
                let _ = updates.send(UiUpdate::SleepDetectionEnabled(val != 0));
            } else if feature_id == 27 {
                let mode = if val != 0 { 1 } else { 0 };
                let _ = updates.send(UiUpdate::SpatialAudio(mode));
            }
        }
        return;
    }
    if (packet.command == 0x012a || packet.command == 0x812a || packet.command == 0x0510)
        && !packet.payload.is_empty()
    {
        if let Ok(mode) = parse_spatial_audio_response(&packet.payload) {
            let _ = updates.send(UiUpdate::SpatialAudio(mode));
        }
        return;
    }
    if packet.command != 0x0204 {
        return;
    }
    match packet.payload.first() {
        Some(1) => {
            if let Ok(state) = parse_battery_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(state));
            }
        }
        Some(2) => {
            if let Ok(state) = parse_wear_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Wear(state));
            }
        }
        Some(3) => {
            if let Some(mode) = parse_noise_mode_event(&packet.payload) {
                let _ = updates.send(UiUpdate::NoiseMode(mode));
            }
        }
        Some(4) => {
            if let Ok(result) = parse_fit_test_event(&packet.payload) {
                let _ = updates.send(UiUpdate::FitTestRunning(false));
                let _ = updates.send(UiUpdate::FitTestResult(result));
            }
        }
        Some(6) => {
            let result = parse_paired_devices_event(&packet.payload)
                .map_err(|error| crate::t!("dual-device-event-invalid", error));
            let _ = updates.send(UiUpdate::PairedDevices(result));
        }
        _ => {}
    }
}

pub fn publish_read_only_event(packet: &AppPacket, updates: &Sender<UiUpdate>) {
    if packet.command != 0x0204 {
        return;
    }
    match packet.payload.first() {
        Some(1) => {
            if let Ok(state) = parse_battery_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(state));
            }
        }
        Some(2) => {
            if let Ok(state) = parse_wear_event(&packet.payload) {
                let _ = updates.send(UiUpdate::Wear(state));
            }
        }
        _ => {}
    }
}

pub fn merge_wear_state(current: &mut WearState, update: WearState) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case_open.is_some() {
        current.case_open = update.case_open;
    }
}
