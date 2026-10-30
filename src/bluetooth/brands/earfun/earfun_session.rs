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
    os::fd::AsRawFd,
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::app::events::SettingDomain;
use crate::bluetooth::transport::{RfcommServiceQuery, is_socket_timeout, open_rfcomm};
use crate::bluetooth::{DeviceChoice, UiUpdate, WorkerCommand};
use crate::compatibility::{self, NoiseMode};
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunFrameDecoder, EarFunPacket, encode_earfun_eq_band,
    parse_earfun_ambient_mode, parse_earfun_anc_sub_mode, parse_earfun_battery_component,
    parse_earfun_dual_device, parse_earfun_firmware_version, parse_earfun_game_mode,
    parse_earfun_in_ear_detection, parse_earfun_paired_device_entry,
    parse_earfun_transparency_sub_mode,
};
use crate::protocol::{
    BatteryPart, BatteryState, FirmwareVersions, PairedDevice, PairedDeviceAction, WearPart,
    WearState,
};

pub const EARFUN_AIR_PRO_4_PRODUCT_ID: u32 = 0x004535;
pub const EARFUN_FREE_PRO_3_PRODUCT_ID: u32 = 0x004534;
pub const EARFUN_AIR_S_PRODUCT_ID: u32 = 0x004c45;

/// Extracts the EarFun Product ID from hardware firmware version string (e.g. "TW600-3091_...").
///
/// In EarFun's architecture:
/// - TW600: EarFun Air Pro 4 (PID 0x004535)
/// - TW400 / TW400B: EarFun Free Pro 3 (PID 0x004534)
/// - TW201: EarFun Air S (PID 0x004C45)
pub fn parse_earfun_product_id_from_firmware(fw: &str) -> Option<u32> {
    if fw.starts_with("TW600") {
        Some(EARFUN_AIR_PRO_4_PRODUCT_ID)
    } else if fw.starts_with("TW400") {
        Some(EARFUN_FREE_PRO_3_PRODUCT_ID)
    } else if fw.starts_with("TW201") {
        Some(EARFUN_AIR_S_PRODUCT_ID)
    } else {
        None
    }
}
pub const EARFUN_FALLBACK_RFCOMM_CHANNEL: u8 = 11;
pub const GAIA_SERVICE_UUID: &str = "00001107-d102-11e1-9b23-00025b00a5a5";
pub const EARFUN_PRIMARY_SPP_NAMES: &[&str] = &[];
pub const EARFUN_SECONDARY_SPP_NAMES: &[&str] = &["gaia", "earfun", "spp", "serial port"];

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_secs(15);

#[allow(dead_code)]
pub struct EarFunDeviceSnapshot {
    pub name: String,
    pub product_id: u32,
    pub color_id: Option<u8>,
    pub model: String,
    pub is_known_model: bool,
    pub read_only: bool,
    pub battery: BatteryState,
    pub firmware: FirmwareVersions,
    pub wear: WearState,
    pub supports_wear_detection: bool,
    pub in_ear_detection_enabled: Option<bool>,
    pub noise_mode: Option<u8>,
    pub noise_mode_indices: Vec<u8>,
    pub noise_mode_support_note: String,
    pub noise_modes: &'static [NoiseMode],
    pub anc_sub: u8,
    pub transp_sub: u8,
    pub supports_dual_device: bool,
    pub dual_device_enabled: Option<bool>,
    pub supports_paired_devices: bool,
    pub paired_devices: Result<Vec<PairedDevice>, String>,
    pub paired_device_controls_enabled: Result<bool, String>,
    pub supports_game_mode: bool,
    pub game_mode_enabled: Option<bool>,
    pub supports_equalizer: bool,
    pub equalizer_preset: Option<u8>,
    pub detected_features: Vec<String>,
    pub probe_errors: Vec<String>,
}

pub fn open_earfun_rfcomm(device: &DeviceChoice) -> std::io::Result<std::fs::File> {
    let mut service_uuids = device.control_service_uuids.clone();
    if service_uuids.is_empty() {
        if let Ok(gaia_uuid) = bluer::Uuid::parse_str(GAIA_SERVICE_UUID) {
            service_uuids.push(gaia_uuid);
        }
    }
    let query = RfcommServiceQuery {
        service_uuids: &service_uuids,
        primary_spp_names: EARFUN_PRIMARY_SPP_NAMES,
        secondary_spp_names: EARFUN_SECONDARY_SPP_NAMES,
        fallback_channel: Some(EARFUN_FALLBACK_RFCOMM_CHANNEL),
    };
    open_rfcomm(device.address, &query)
}

pub fn spawn_earfun_monitor(
    device: DeviceChoice,
    updates: Sender<UiUpdate>,
    commands: Receiver<WorkerCommand>,
) -> std::io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("earfun-bluetooth".to_string())
        .spawn(move || {
            let result = monitor_device(device, &updates, &commands);
            if let Err(error) = result {
                let _ = updates.send(UiUpdate::Status(error));
            }
            let _ = updates.send(UiUpdate::Finished);
        })
}

pub(crate) fn send_command<T: Write>(
    stream: &mut T,
    command: u16,
    payload: Vec<u8>,
) -> Result<(), String> {
    let pkt = if command == EarFunCommands::SET_EQUALIZER_BAND {
        EarFunPacket::new_v3(command, payload)
    } else {
        EarFunPacket::new(command, payload)
    };
    let frame = pkt.encode();
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;
    Ok(())
}

fn drain_stream<T: Read>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
    duration: Duration,
) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        let mut bytes = [0u8; 512];
        match stream.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => {
                let packets = decoder.push(&bytes[..n]);
                for packet in packets {
                    handle_unsolicited(&packet, updates);
                }
            }
            Err(e) if is_socket_timeout(&e) => break,
            Err(_) => break,
        }
    }
}

pub(crate) fn request<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    command: u16,
    payload: Vec<u8>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<EarFunPacket, String> {
    let pkt = if command == EarFunCommands::SET_EQUALIZER_BAND {
        EarFunPacket::new_v3(command, payload)
    } else {
        EarFunPacket::new(command, payload)
    };
    let frame = pkt.encode();
    stream
        .write_all(&frame)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    let deadline = Instant::now() + REQUEST_TIMEOUT;
    loop {
        let mut bytes = [0; 1024];
        let length = match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(length) => length,
            Err(error) if is_socket_timeout(&error) => {
                if Instant::now() >= deadline {
                    return Err(crate::t!(
                        "protocol-query-timeout",
                        format!("{command:04X}")
                    ));
                }
                continue;
            }
            Err(error) => return Err(crate::t!("protocol-response-read-failed", error)),
        };

        let packets = decoder.push(&bytes[..length]);
        let mut response = None;
        let mut trailing_packets = Vec::new();

        for (index, packet) in packets.iter().enumerate() {
            let is_match = packet.command == (command | 0x8000)
                || (command == EarFunCommands::SET_EQUALIZER_BAND
                    && (packet.command == EarFunCommands::ACK_EQUALIZER_BAND
                        || packet.command == (command | 0x8000)));
            if is_match {
                response = Some(packet.clone());
                trailing_packets.extend(packets.iter().skip(index + 1).cloned());
                break;
            }
            handle_unsolicited(&packet, updates);
        }

        if let Some(response) = response {
            for packet in trailing_packets {
                handle_unsolicited(&packet, updates);
            }
            if !response.payload.is_empty()
                && response.command != EarFunCommands::ACK_EQUALIZER_BAND
                && response.payload[0] != 0x00
            {
                let status_str = format!("0x{:02X}", response.payload[0]);
                return Err(crate::t!(
                    "protocol-command-rejected",
                    format!("{command:04X}"),
                    status_str
                ));
            }
            return Ok(response);
        }

        if Instant::now() >= deadline {
            return Err(crate::t!(
                "protocol-query-timeout",
                format!("{command:04X}")
            ));
        }
    }
}

fn handle_unsolicited(packet: &EarFunPacket, updates: Option<&Sender<UiUpdate>>) {
    let Some(updates) = updates else { return };
    match packet.command {
        EarFunCommands::BATTERY_LEFT => {
            if let Ok(part) = parse_earfun_battery_component(&packet.payload) {
                let _ = updates.send(UiUpdate::Battery(BatteryState {
                    left: Some(part),
                    right: None,
                    case: None,
                }));
            }
        }
        EarFunCommands::BATTERY_RIGHT => {
            if let Ok(part) = parse_earfun_battery_component(&packet.payload) {
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
                    case: Some(part),
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

#[repr(C)]
struct SockAddrRc {
    family: libc::sa_family_t,
    address: [u8; 6],
    channel: u8,
}

pub(crate) fn get_local_rfcomm_lap<T: AsRawFd>(stream: &T) -> Option<[u8; 3]> {
    let mut addr = SockAddrRc {
        family: 0,
        address: [0; 6],
        channel: 0,
    };
    let mut len = std::mem::size_of::<SockAddrRc>() as libc::socklen_t;
    let res = unsafe {
        libc::getsockname(
            stream.as_raw_fd(),
            &mut addr as *mut SockAddrRc as *mut libc::sockaddr,
            &mut len,
        )
    };
    if res == 0 && addr.family == libc::AF_BLUETOOTH as libc::sa_family_t {
        Some([addr.address[2], addr.address[1], addr.address[0]])
    } else {
        None
    }
}

pub(crate) fn read_earfun_paired_devices<T: Read + Write + AsRawFd>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<Vec<PairedDevice>, String> {
    let local_lap = get_local_rfcomm_lap(stream);
    send_command(stream, EarFunCommands::GET_CONNECTED_DEVICES, vec![])?;

    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut slots_received = std::collections::BTreeMap::new();

    while Instant::now() < deadline {
        let mut bytes = [0u8; 1024];
        let n = match stream.read(&mut bytes) {
            Ok(0) => return Err(crate::t!("channel-headset-closed")),
            Ok(n) => n,
            Err(e) if is_socket_timeout(&e) => {
                if !slots_received.is_empty() {
                    break;
                }
                continue;
            }
            Err(e) => return Err(crate::t!("protocol-response-read-failed", e)),
        };

        let packets = decoder.push(&bytes[..n]);
        for packet in packets {
            if packet.command == EarFunCommands::GET_CONNECTED_DEVICES
                || packet.command == (EarFunCommands::GET_CONNECTED_DEVICES | 0x8000)
            {
                if packet.payload.len() >= 27 {
                    let slot = packet.payload[1];
                    slots_received.insert(slot, packet.payload);
                }
            } else {
                handle_unsolicited(&packet, updates);
            }
        }

        if slots_received.len() >= 10 {
            break;
        }
    }

    if slots_received.is_empty() {
        return Err(crate::t!("dual-device-read-list-failed", "timeout"));
    }

    let mut devices = Vec::new();
    for (_, payload) in slots_received {
        if let Ok(Some(dev)) = parse_earfun_paired_device_entry(&payload, local_lap) {
            devices.push(dev);
        }
    }
    devices.sort_by_key(|d| (!d.connected, !d.is_current_host));

    Ok(devices)
}

pub fn initialize_earfun_session<T: Read + Write + AsRawFd>(
    stream: &mut T,
    device_name: &str,
    device_address: Option<&str>,
    discovered_product_id: Option<u32>,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(EarFunDeviceSnapshot, EarFunFrameDecoder), String> {
    let mut decoder = EarFunFrameDecoder::default();
    let mut detected_features = Vec::new();
    let mut probe_errors = Vec::new();

    // 1. Firmware version
    let mut raw_firmware_string: Option<String> = None;
    let firmware = match request(
        stream,
        &mut decoder,
        EarFunCommands::FIRMWARE_VERSION,
        vec![],
        updates,
    ) {
        Ok(pkt) => {
            if pkt.payload.len() > 1 && pkt.payload[0] == 0x00 {
                if let Ok(s) = std::str::from_utf8(&pkt.payload[1..]) {
                    raw_firmware_string = Some(s.to_string());
                }
            }
            match parse_earfun_firmware_version(&pkt.payload) {
                Ok(fw) => {
                    detected_features.push("firmware-version".to_string());
                    fw
                }
                Err(e) => {
                    probe_errors.push(format!("Firmware version parse error: {e}"));
                    FirmwareVersions::default()
                }
            }
        }
        Err(e) => {
            probe_errors.push(format!("Firmware query failed: {e}"));
            FirmwareVersions::default()
        }
    };

    let product_id = discovered_product_id.or_else(|| {
        raw_firmware_string
            .as_deref()
            .and_then(parse_earfun_product_id_from_firmware)
    });
    let profile = product_id.and_then(compatibility::profile_for_product_id);
    let is_known_model = profile.is_some();
    let read_only = profile.is_none_or(|profile| !profile.supports_controls());
    let model = profile.map_or_else(
        || crate::t!("device-unknown-model"),
        |p| p.model.to_string(),
    );

    if read_only {
        let left_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_LEFT,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok());
        let right_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_RIGHT,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok());
        let case_bat = request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_CASE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok());

        let snapshot = EarFunDeviceSnapshot {
            name: device_name.to_string(),
            product_id: product_id.unwrap_or(0),
            color_id: None,
            model,
            is_known_model,
            read_only: true,
            battery: BatteryState {
                left: left_bat,
                right: right_bat,
                case: case_bat,
            },
            firmware,
            wear: WearState::default(),
            supports_wear_detection: false,
            in_ear_detection_enabled: None,
            noise_mode: None,
            noise_mode_indices: vec![],
            noise_mode_support_note: String::new(),
            noise_modes: &[],
            anc_sub: 2,
            transp_sub: 0,
            supports_dual_device: false,
            dual_device_enabled: None,
            supports_paired_devices: false,
            paired_devices: Err(crate::t!("dual-device-not-supported")),
            paired_device_controls_enabled: Err(crate::t!("dual-device-not-supported")),
            supports_game_mode: false,
            game_mode_enabled: None,
            supports_equalizer: false,
            equalizer_preset: None,
            detected_features,
            probe_errors,
        };
        return Ok((snapshot, decoder));
    }

    let profile = profile.unwrap();
    let pid = product_id.unwrap();
    let noise_modes = profile.noise_modes;
    let has_submodes = noise_modes.iter().any(|m| m.parent.is_some());
    let noise_mode_indices: Vec<u8> = noise_modes.iter().map(|m| m.index).collect();
    let supports_wear_detection = profile.supports_wear_detection();
    let supports_dual_device = profile.supports_dual_device();
    let supports_game_mode = profile.supports_game_mode();
    let supports_equalizer = profile.supports_equalizer();

    // 2. Battery levels
    let left_bat = request(
        stream,
        &mut decoder,
        EarFunCommands::BATTERY_LEFT,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok());
    let right_bat = request(
        stream,
        &mut decoder,
        EarFunCommands::BATTERY_RIGHT,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok());
    let case_bat = if pid != EARFUN_AIR_S_PRODUCT_ID {
        request(
            stream,
            &mut decoder,
            EarFunCommands::BATTERY_CASE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok())
    } else {
        None
    };

    if left_bat.is_some() || right_bat.is_some() || case_bat.is_some() {
        detected_features.push("battery-status".to_string());
    }

    let battery = BatteryState {
        left: left_bat,
        right: right_bat,
        case: case_bat,
    };

    // 3. Audio & Noise Modes
    let ambient_mode = request(
        stream,
        &mut decoder,
        EarFunCommands::GET_AMBIENT_SOUND,
        vec![],
        updates,
    )
    .ok()
    .and_then(|pkt| parse_earfun_ambient_mode(&pkt.payload).ok())
    .unwrap_or(0);

    let anc_sub = if ambient_mode == 1 && has_submodes {
        request(
            stream,
            &mut decoder,
            EarFunCommands::GET_ANC_SUB_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_anc_sub_mode(&pkt.payload).ok())
        .unwrap_or(2)
    } else {
        2 // Default to Strong ANC
    };

    let transp_sub = if ambient_mode == 2 && has_submodes {
        request(
            stream,
            &mut decoder,
            EarFunCommands::GET_TRANSPARENCY_SUB_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_transparency_sub_mode(&pkt.payload).ok())
        .unwrap_or(0)
    } else {
        0 // Default
    };

    let current_protocol_index = if !has_submodes {
        ambient_mode
    } else {
        earfun_mode_to_protocol_index(ambient_mode, anc_sub, transp_sub)
    };

    // 4. Game mode
    let game_mode_enabled = if supports_game_mode {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_GAME_MODE,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_game_mode(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("game-mode".to_string());
        }
        enabled
    } else {
        None
    };

    // 5. In-ear detection setting
    let in_ear_detection_enabled = if supports_wear_detection {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_IN_EAR_DETECTION,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_in_ear_detection(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("wear-detection".to_string());
        }
        enabled
    } else {
        None
    };

    // 6. Dual device (multipoint)
    let (dual_device_enabled, paired_devices) = if supports_dual_device {
        let enabled = request(
            stream,
            &mut decoder,
            EarFunCommands::GET_CONNECT_TWO_DEVICES,
            vec![],
            updates,
        )
        .ok()
        .and_then(|pkt| parse_earfun_dual_device(&pkt.payload).ok());
        if enabled.is_some() {
            detected_features.push("dual-device".to_string());
        }

        let devs = read_earfun_paired_devices(stream, &mut decoder, updates).map_err(|e| {
            probe_errors.push(e.clone());
            e
        });

        (enabled, devs)
    } else {
        (None, Err(crate::t!("dual-device-not-supported")))
    };

    let paired_device_controls_enabled = if !supports_dual_device {
        Err(crate::t!("dual-device-not-supported"))
    } else if dual_device_enabled == Some(false) {
        Err(crate::t!("dual-device-not-enabled"))
    } else {
        Ok(true)
    };

    // When connected via RFCOMM, earbuds are active and in-ear
    let wear = if supports_wear_detection {
        WearState {
            left: Some(WearPart {
                out_of_case: true,
                in_ear: true,
                raw_flags: 0,
            }),
            right: Some(WearPart {
                out_of_case: true,
                in_ear: true,
                raw_flags: 0,
            }),
            case_open: Some(true),
        }
    } else {
        WearState::default()
    };

    let snapshot = EarFunDeviceSnapshot {
        name: device_name.to_string(),
        product_id: pid,
        color_id: None,
        model,
        is_known_model: true,
        read_only: false,
        battery,
        firmware,
        wear,
        supports_wear_detection,
        in_ear_detection_enabled,
        noise_mode: Some(current_protocol_index),
        noise_mode_indices,
        noise_mode_support_note: "EarFun GAIA V3 Noise Architecture".to_string(),
        noise_modes,
        anc_sub,
        transp_sub,
        supports_dual_device,
        dual_device_enabled,
        supports_paired_devices: supports_dual_device,
        paired_devices,
        paired_device_controls_enabled,
        supports_game_mode,
        game_mode_enabled,
        supports_equalizer,
        equalizer_preset: if supports_equalizer {
            Some(crate::resources::get_saved_equalizer_preset(pid, device_address).unwrap_or(0))
        } else {
            None
        },
        detected_features,
        probe_errors,
    };

    Ok((snapshot, decoder))
}

pub fn earfun_mode_to_protocol_index(master: u8, anc_sub: u8, transp_sub: u8) -> u8 {
    match master {
        0 => 0, // Off
        1 => match anc_sub {
            0 => 7, // AI Ear Adaptive
            1 => 6, // Wind Noise Cancelling
            2 => 4, // Strong ANC
            3 => 5, // Balanced ANC
            4 => 3, // AI Environment Adaptive
            _ => 1, // ANC parent
        },
        2 => match transp_sub {
            0 => 8, // Default
            1 => 9, // Vocal / Natural
            _ => 2, // Transparency parent
        },
        _ => 0,
    }
}

pub fn protocol_index_to_earfun_mode(protocol_index: u8) -> (u8, Option<u8>, bool) {
    match protocol_index {
        0 => (0, None, false),
        1 => (1, None, true),
        2 => (2, None, false),
        3 => (1, Some(4), true),
        4 => (1, Some(2), true),
        5 => (1, Some(3), true),
        6 => (1, Some(1), true),
        7 => (1, Some(0), true),
        8 => (2, Some(0), false),
        9 => (2, Some(1), false),
        _ => (0, None, false),
    }
}

pub(crate) fn set_noise_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    protocol_index: u8,
    last_anc_sub: &mut u8,
    last_transp_sub: &mut u8,
    updates: &Sender<UiUpdate>,
) -> Result<u8, String> {
    let profile = compatibility::profile_for_product_id(product_id);
    let has_submodes = profile
        .map(|p| p.noise_modes.iter().any(|m| m.parent.is_some()))
        .unwrap_or(false);

    if !has_submodes {
        send_command(
            stream,
            EarFunCommands::SET_AMBIENT_SOUND,
            vec![protocol_index],
        )?;
        drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
        return Ok(protocol_index);
    }

    let confirmed_index = match protocol_index {
        0 => {
            // Off: set master mode to 0
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![0])?;
            0
        }
        1 => {
            // ANC primary button clicked: activate master 1 + last ANC submode
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![1])?;
            thread::sleep(Duration::from_millis(20));
            send_command(
                stream,
                EarFunCommands::SET_ANC_SUB_MODE,
                vec![*last_anc_sub],
            )?;
            earfun_mode_to_protocol_index(1, *last_anc_sub, *last_transp_sub)
        }
        2 => {
            // Transparency primary button clicked: activate master 2 + last Transparency submode
            send_command(stream, EarFunCommands::SET_AMBIENT_SOUND, vec![2])?;
            thread::sleep(Duration::from_millis(20));
            send_command(
                stream,
                EarFunCommands::SET_TRANSPARENCY_SUB_MODE,
                vec![*last_transp_sub],
            )?;
            earfun_mode_to_protocol_index(2, *last_anc_sub, *last_transp_sub)
        }
        sub_index => {
            // Specific submode selected from dropdown
            let (target_master, target_sub, is_anc) = protocol_index_to_earfun_mode(sub_index);
            send_command(
                stream,
                EarFunCommands::SET_AMBIENT_SOUND,
                vec![target_master],
            )?;
            if let Some(sub) = target_sub {
                thread::sleep(Duration::from_millis(20));
                if is_anc {
                    *last_anc_sub = sub;
                    send_command(stream, EarFunCommands::SET_ANC_SUB_MODE, vec![sub])?;
                } else {
                    *last_transp_sub = sub;
                    send_command(stream, EarFunCommands::SET_TRANSPARENCY_SUB_MODE, vec![sub])?;
                }
            }
            sub_index
        }
    };

    drain_stream(stream, decoder, Some(updates), Duration::from_millis(100));
    Ok(confirmed_index)
}

pub(crate) fn set_game_mode<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    send_command(stream, EarFunCommands::SET_GAME_MODE, vec![enabled as u8])?;
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(enabled)
}

pub(crate) fn set_in_ear_detection<T: Read + Write>(
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

pub(crate) fn set_dual_device<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    enabled: bool,
    updates: &Sender<UiUpdate>,
) -> Result<bool, String> {
    send_command(
        stream,
        EarFunCommands::SET_CONNECT_TWO_DEVICES,
        vec![enabled as u8],
    )?;
    drain_stream(stream, decoder, Some(updates), Duration::from_millis(50));
    Ok(enabled)
}

pub(crate) fn apply_equalizer_preset<T: Read + Write>(
    stream: &mut T,
    decoder: &mut EarFunFrameDecoder,
    product_id: u32,
    preset_index: u8,
    updates: Option<&Sender<UiUpdate>>,
) -> Result<(), String> {
    let mut joined_frames = Vec::new();

    if product_id == EARFUN_AIR_S_PRODUCT_ID {
        // 6-band EQ for EarFun Air S (with 4 dummy bands)
        let frequencies: [u16; 10] = [
            0x00BD, 0x021C, 0x05DC, 0x0BB8, 0x5DC0, 0xAFC8, // 63, 180, 500, 1000, 8000, 15000
            0x0063, 0x0258, 0x1770, 0xAFC8, // dummy 7..10
        ];
        let q_factors: [u16; 10] = [
            0x0B33, 0x0999, 0x0999, 0x0999, 0x0999, 0x0999, 0x0800, 0x0999, 0x0E66, 0x0999,
        ];
        let dummy_gains = [4.5, -4.5, 1.5, 3.5];

        let gains_6: [f64; 6] = match preset_index {
            0 => [0.0; 6],                          // Classic
            1 => [0.0, 0.0, 1.0, 1.0, 2.0, 3.0],    // Natural
            2 => [8.0, 3.0, 2.0, 0.0, 0.0, 0.0],    // Bass Boost
            3 => [0.0, 0.0, 0.0, 2.0, 3.0, 5.0],    // Treble Boost
            4 => [-5.0, -2.0, 2.0, 3.0, 0.0, -3.0], // Soft
            5 => [7.0, 3.0, 2.0, 3.0, 5.0, 7.0],    // Dynamic
            6 => [2.0, 0.0, 3.0, 5.0, 3.0, 5.0],    // Clear
            7 => [2.0, 1.0, 0.0, -1.0, -3.0, -5.0], // Relaxed
            8 => [-2.0, 0.0, 4.0, 5.0, 2.0, -1.0],  // Vocal
            _ => [0.0; 6],
        };

        for i in 0..10 {
            let band_id = 0xA1 + i as u8;
            let gain = if i < 6 {
                gains_6[i]
            } else {
                dummy_gains[i - 6]
            };
            let payload = encode_earfun_eq_band(band_id, frequencies[i], q_factors[i], gain);
            let pkt = EarFunPacket::new_v3(EarFunCommands::SET_EQUALIZER_BAND, payload);
            joined_frames.extend_from_slice(&pkt.encode());
        }
    } else {
        // 10 bands for Air Pro 4 & Free Pro 3
        let frequencies: [u16; 10] = [
            0x005E, 0x00BD, 0x0177, 0x02EE, 0x05DC, 0x0BB8, 0x1770, 0x2EE0, 0x5DC0, 0xBB80,
        ];
        let q_factor: u16 = 0x0B33;

        let gains: [f64; 10] = match preset_index {
            0 => [0.0; 10],                                                // Classic / Flat
            1 => [0.0, 0.0, 1.0, 2.0, 2.0, -1.0, -1.0, -1.0, -2.0, 1.0],   // Natural
            2 => [8.0, 6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],       // Bass Boost
            3 => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 3.0, 4.0],       // Clear Treble
            4 => [-5.0, -4.0, -2.0, 0.0, 2.0, 3.0, 0.0, -2.0, -3.0, -5.0], // Soft
            5 => [6.0, 6.0, 4.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],       // Dynamic
            6 => [3.0, 2.0, 2.0, 2.0, 3.0, 5.0, 3.0, 3.0, 4.0, 5.0],       // Clear
            7 => [2.0, 1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -3.0, -4.0, -5.0],  // Relaxed
            8 => [-3.0, -2.0, 0.0, 2.0, 3.0, 5.0, 3.0, 2.0, 0.0, -1.0],    // Pure Vocal
            _ => [0.0; 10],
        };

        for i in 0..10 {
            let band_id = 0xA1 + i as u8;
            let payload = encode_earfun_eq_band(band_id, frequencies[i], q_factor, gains[i]);
            let pkt = EarFunPacket::new_v3(EarFunCommands::SET_EQUALIZER_BAND, payload);
            joined_frames.extend_from_slice(&pkt.encode());
        }
    }

    stream
        .write_all(&joined_frames)
        .map_err(|error| crate::t!("protocol-query-send-failed", error))?;

    // Drain incoming ACK frames from socket buffer
    drain_stream(stream, decoder, updates, Duration::from_millis(150));

    Ok(())
}

fn monitor_device(
    device: DeviceChoice,
    updates: &Sender<UiUpdate>,
    commands: &Receiver<WorkerCommand>,
) -> Result<(), String> {
    let mut stream = open_earfun_rfcomm(&device)
        .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
    let mac_str = device.address.to_string();
    let (snapshot, mut decoder) = initialize_earfun_session(
        &mut stream,
        &device.name,
        Some(&mac_str),
        device.product_id,
        Some(updates),
    )?;

    let _ = updates.send(UiUpdate::Connected {
        device: device.clone(),
    });
    let _ = updates.send(UiUpdate::NoiseControlsEnabled(
        !snapshot.read_only && !snapshot.noise_modes.is_empty(),
    ));
    let _ = updates.send(UiUpdate::NoiseModeOptions {
        indices: snapshot.noise_mode_indices.clone(),
        note: snapshot.noise_mode_support_note.clone(),
        modes: snapshot.noise_modes.to_vec(),
    });
    let _ = updates.send(UiUpdate::ProductId {
        product_id: snapshot.product_id,
        color_id: snapshot.color_id,
    });
    let _ = updates.send(UiUpdate::Firmware(snapshot.firmware));
    let _ = updates.send(UiUpdate::Wear(snapshot.wear.clone()));
    let _ = updates.send(UiUpdate::Battery(snapshot.battery));

    let presets = compatibility::equalizer_presets_for_product(snapshot.product_id).to_vec();
    let _ = updates.send(UiUpdate::SoundFeatures {
        supports_equalizer: snapshot.supports_equalizer,
        supports_custom_equalizer: false,
        supports_spatial_audio: false,
        supports_head_tracking: false,
        supports_game_mode: snapshot.supports_game_mode,
        supports_bass_engine: false,
        supports_hi_res: false,
        presets,
    });

    if let Some(preset) = snapshot.equalizer_preset {
        if preset != 0 {
            let _ = apply_equalizer_preset(
                &mut stream,
                &mut decoder,
                snapshot.product_id,
                preset,
                Some(updates),
            );
        }
        let _ = updates.send(UiUpdate::Equalizer(preset));
    }
    if let Some(enabled) = snapshot.game_mode_enabled {
        let _ = updates.send(UiUpdate::GameMode(enabled));
    }
    if let Some(mode) = snapshot.noise_mode {
        let _ = updates.send(UiUpdate::NoiseMode(mode));
    }

    let _ = updates.send(UiUpdate::ControlsFeatures {
        supports_touch_customization: false,
        supports_dual_device: snapshot.supports_dual_device,
        supports_wear_detection: snapshot.supports_wear_detection,
        supports_sleep_detection: false,
        supports_head_motion: false,
        supports_find_device: false,
        supports_fit_detection: false,
        supports_prompt_volume: false,
        gesture_rows: vec![],
    });

    if let Some(enabled) = snapshot.in_ear_detection_enabled {
        let _ = updates.send(UiUpdate::InEarDetectionEnabled(enabled));
    }
    if let Some(enabled) = snapshot.dual_device_enabled {
        let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(enabled));
    }

    let _ = updates.send(UiUpdate::PairedDeviceControlsEnabled(
        snapshot.paired_device_controls_enabled.clone(),
    ));
    let _ = updates.send(UiUpdate::PairedDevices(snapshot.paired_devices.clone()));

    let _ = updates.send(UiUpdate::Status(crate::t!("channel-connected")));

    let mut current_anc_sub = snapshot.anc_sub;
    let mut current_transp_sub = snapshot.transp_sub;
    let has_submodes = snapshot.noise_modes.iter().any(|m| m.parent.is_some());
    let mut last_poll = Instant::now();

    loop {
        // Handle incoming UI commands
        match commands.try_recv() {
            Ok(WorkerCommand::Disconnect) => break,
            Ok(WorkerCommand::SetNoiseMode(protocol_index)) => {
                let _ = updates.send(UiUpdate::Status(crate::t!("noise-mode-waiting-confirm")));
                match set_noise_mode(
                    &mut stream,
                    &mut decoder,
                    snapshot.product_id,
                    protocol_index,
                    &mut current_anc_sub,
                    &mut current_transp_sub,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::NoiseMode(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!("noise-mode-confirmed")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::NoiseMode,
                            message: error,
                        });
                    }
                }
                let _ = updates.send(UiUpdate::NoiseControlsEnabled(true));
            }
            Ok(WorkerCommand::SetGameMode(enabled)) => {
                match set_game_mode(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::GameMode(actual));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::GameMode,
                            message: error,
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetInEarDetection(enabled)) => {
                match set_in_ear_detection(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::InEarDetectionEnabled(actual));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: error,
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetDualDeviceConnection(enabled)) => {
                match set_dual_device(&mut stream, &mut decoder, enabled, updates) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(actual));
                        let controls_enabled = if actual {
                            Ok(true)
                        } else {
                            Err(crate::t!("dual-device-not-enabled"))
                        };
                        let _ =
                            updates.send(UiUpdate::PairedDeviceControlsEnabled(controls_enabled));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: error,
                        });
                    }
                }
            }
            Ok(WorkerCommand::SetPairedDevice { address, action }) => {
                let slot = address.command_bytes()[2];
                match action {
                    PairedDeviceAction::Connect => {
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("dual-device-waiting-update")));
                        if let Err(_e) =
                            send_command(&mut stream, EarFunCommands::CONNECT_DEVICE, vec![slot])
                        {
                            let _ = updates.send(UiUpdate::Status(crate::t!(
                                "dual-device-send-connect-failed"
                            )));
                            let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                            continue;
                        }
                        thread::sleep(Duration::from_millis(300));
                        match read_earfun_paired_devices(&mut stream, &mut decoder, Some(updates)) {
                            Ok(devs) => {
                                let _ = updates.send(UiUpdate::PairedDevices(Ok(devs)));
                                let _ = updates.send(UiUpdate::Status(crate::t!(
                                    "dual-device-status-confirmed"
                                )));
                            }
                            Err(e) => {
                                let _ = updates.send(UiUpdate::Status(crate::t!(
                                    "dual-device-readback-list-failed",
                                    e
                                )));
                            }
                        }
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                    PairedDeviceAction::Disconnect => {
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("dual-device-waiting-update")));
                        if let Err(_e) =
                            send_command(&mut stream, EarFunCommands::DISCONNECT_DEVICE, vec![slot])
                        {
                            let _ = updates.send(UiUpdate::Status(crate::t!(
                                "dual-device-send-disconnect-failed"
                            )));
                            let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                            continue;
                        }
                        thread::sleep(Duration::from_millis(300));
                        match read_earfun_paired_devices(&mut stream, &mut decoder, Some(updates)) {
                            Ok(devs) => {
                                let _ = updates.send(UiUpdate::PairedDevices(Ok(devs)));
                                let _ = updates.send(UiUpdate::Status(crate::t!(
                                    "dual-device-status-confirmed"
                                )));
                            }
                            Err(e) => {
                                let _ = updates.send(UiUpdate::Status(crate::t!(
                                    "dual-device-readback-list-failed",
                                    e
                                )));
                            }
                        }
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                    PairedDeviceAction::Remove => {
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "dual-device-management-blocked"
                        )));
                        let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    }
                }
            }
            Ok(WorkerCommand::SetEqualizer(preset_index)) => {
                match apply_equalizer_preset(
                    &mut stream,
                    &mut decoder,
                    snapshot.product_id,
                    preset_index,
                    Some(updates),
                ) {
                    Ok(()) => {
                        let _ = crate::resources::save_equalizer_preset(
                            snapshot.product_id,
                            Some(&mac_str),
                            preset_index,
                        );
                        let _ = updates.send(UiUpdate::Equalizer(preset_index));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: error,
                        });
                    }
                }
            }
            Ok(_) => {}
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        // Periodic battery poll
        if last_poll.elapsed() >= POLL_INTERVAL {
            last_poll = Instant::now();
            let _ = request(
                &mut stream,
                &mut decoder,
                EarFunCommands::BATTERY_LEFT,
                vec![],
                Some(updates),
            );
            let _ = request(
                &mut stream,
                &mut decoder,
                EarFunCommands::BATTERY_RIGHT,
                vec![],
                Some(updates),
            );
            let _ = request(
                &mut stream,
                &mut decoder,
                EarFunCommands::BATTERY_CASE,
                vec![],
                Some(updates),
            );

            // Periodic ambient sound poll to sync with earbud physical touches
            if let Ok(pkt) = request(
                &mut stream,
                &mut decoder,
                EarFunCommands::GET_AMBIENT_SOUND,
                vec![],
                Some(updates),
            ) {
                if let Ok(master) = parse_earfun_ambient_mode(&pkt.payload) {
                    let actual_idx = if !has_submodes {
                        master
                    } else if master == 0 {
                        0
                    } else if master == 1 {
                        if let Ok(sub_pkt) = request(
                            &mut stream,
                            &mut decoder,
                            EarFunCommands::GET_ANC_SUB_MODE,
                            vec![],
                            Some(updates),
                        ) {
                            if let Ok(sub) = parse_earfun_anc_sub_mode(&sub_pkt.payload) {
                                current_anc_sub = sub;
                            }
                        }
                        earfun_mode_to_protocol_index(1, current_anc_sub, current_transp_sub)
                    } else if master == 2 {
                        if let Ok(sub_pkt) = request(
                            &mut stream,
                            &mut decoder,
                            EarFunCommands::GET_TRANSPARENCY_SUB_MODE,
                            vec![],
                            Some(updates),
                        ) {
                            if let Ok(sub) = parse_earfun_transparency_sub_mode(&sub_pkt.payload) {
                                current_transp_sub = sub;
                            }
                        }
                        earfun_mode_to_protocol_index(2, current_anc_sub, current_transp_sub)
                    } else {
                        0
                    };
                    let _ = updates.send(UiUpdate::NoiseMode(actual_idx));
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

pub fn probe_earfun(device: &DeviceChoice) -> Result<(), String> {
    let mut stream = open_earfun_rfcomm(device)
        .map_err(|error| crate::t!("channel-unable-to-open-rfcomm", error))?;
    let mac_str = device.address.to_string();
    let (snapshot, _) = initialize_earfun_session(
        &mut stream,
        &device.name,
        Some(&mac_str),
        device.product_id,
        None,
    )?;
    print_earfun_snapshot(&snapshot);
    Ok(())
}

fn print_battery_part(name: &str, part: &Option<BatteryPart>) {
    if let Some(part) = part {
        println!("{name}: {}%", part.percentage);
    } else {
        println!("{name}: {}", crate::t!("common-not-reported"));
    }
}

fn print_earfun_snapshot(snapshot: &EarFunDeviceSnapshot) {
    println!("{}", crate::t!("probe-device", snapshot.name.as_str()));
    println!(
        "{}",
        crate::t!("probe-model-profile", snapshot.model.as_str())
    );
    println!(
        "{}",
        crate::t!("probe-product-id", format!("{:06X}", snapshot.product_id))
    );
    print_battery_part(&crate::t!("battery-left-ear"), &snapshot.battery.left);
    print_battery_part(&crate::t!("battery-right-ear"), &snapshot.battery.right);
    print_battery_part(&crate::t!("battery-case"), &snapshot.battery.case);

    let not_reported = crate::t!("common-not-reported");
    println!(
        "{}",
        crate::t!(
            "probe-firmware",
            snapshot.firmware.left.as_deref().unwrap_or(&not_reported),
            snapshot.firmware.right.as_deref().unwrap_or(&not_reported),
            snapshot.firmware.case.as_deref().unwrap_or(&not_reported),
        )
    );

    if let Some(mode) = snapshot.noise_mode {
        println!("{}", crate::t!("probe-noise-mode-index", mode));
    }
    println!(
        "{}",
        crate::t!(
            "probe-noise-mode-support",
            snapshot.noise_mode_support_note.as_str()
        )
    );

    if let Some(game) = snapshot.game_mode_enabled {
        println!("Game Mode: {}", if game { "On" } else { "Off" });
    }
    if let Some(dual) = snapshot.dual_device_enabled {
        println!(
            "Dual-Device Connection: {}",
            if dual { "On" } else { "Off" }
        );
    }
    if let Some(in_ear) = snapshot.in_ear_detection_enabled {
        println!("In-Ear Detection: {}", if in_ear { "On" } else { "Off" });
    }

    if snapshot.supports_equalizer {
        if let Some(preset) = snapshot.equalizer_preset {
            let presets = crate::compatibility::equalizer_presets_for_product(snapshot.product_id);
            let preset_name = crate::compatibility::equalizer_preset_key(preset, presets)
                .map(|key| crate::t!(key))
                .unwrap_or_else(|| preset.to_string());
            println!("Headset Equalizer Preset: {preset_name}");
        } else {
            println!("Equalizer preset: Not returned");
        }
    }

    if snapshot.supports_paired_devices {
        match &snapshot.paired_devices {
            Ok(devices) => {
                println!("Paired Devices: {}", devices.len());
                for dev in devices {
                    let host_tag = if dev.is_current_host {
                        " (This Device)"
                    } else {
                        ""
                    };
                    let status_str = if dev.connected {
                        "Connected"
                    } else {
                        "Disconnected"
                    };
                    println!("  - {}{host_tag} [{status_str}]", dev.name);
                }
            }
            Err(e) => println!("Failed to query paired devices: {e}"),
        }
    }
}
