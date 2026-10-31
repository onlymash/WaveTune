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
    fs::File,
    io::{Read, Write},
    sync::mpsc::Sender,
    thread,
    time::Duration,
};

use crate::app::events::{SettingDomain, UiUpdate, WorkerCommand};
use crate::bluetooth::{DeviceChoice, DeviceSnapshot, driver::HeadsetDriver};
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunFrameDecoder, parse_earfun_ambient_mode, parse_earfun_anc_sub_mode,
    parse_earfun_battery_component, parse_earfun_transparency_sub_mode,
};
use crate::protocol::{BatteryState, PairedDeviceAction};

use super::features::{
    controls::set_in_ear_detection,
    dual_device::{read_earfun_paired_devices, set_dual_device},
    noise::{earfun_mode_to_protocol_index, set_noise_mode},
    sound::{apply_equalizer_preset, set_audio_codec, set_game_mode},
};
use super::init::initialize_earfun_session_with_lap;
use super::transport::{
    EARFUN_AIR_S_PRODUCT_ID, POLL_INTERVAL, get_local_rfcomm_lap, open_earfun_rfcomm, request,
    send_command,
};
use super::unsolicited::handle_unsolicited;

pub struct EarFunDriver {
    decoder: EarFunFrameDecoder,
    snapshot: Option<DeviceSnapshot>,
    anc_sub: u8,
    transp_sub: u8,
    device_address: Option<String>,
    local_lap: Option<[u8; 3]>,
}

impl EarFunDriver {
    pub fn new() -> Self {
        Self {
            decoder: EarFunFrameDecoder::default(),
            snapshot: None,
            anc_sub: 2,
            transp_sub: 0,
            device_address: None,
            local_lap: None,
        }
    }
}

impl Default for EarFunDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadsetDriver for EarFunDriver {
    fn brand_name(&self) -> &'static str {
        "earfun"
    }

    fn open_rfcomm(&mut self, device: &DeviceChoice) -> std::io::Result<File> {
        let file = open_earfun_rfcomm(device)?;
        self.local_lap = get_local_rfcomm_lap(&file);
        Ok(file)
    }

    fn initialize<T: Read + Write>(
        &mut self,
        stream: &mut T,
        device: &DeviceChoice,
        updates: Option<&Sender<UiUpdate>>,
    ) -> Result<DeviceSnapshot, String> {
        let mac_str = device.address.to_string();
        self.device_address = Some(mac_str.clone());
        let (snapshot, decoder) = initialize_earfun_session_with_lap(
            stream,
            &device.name,
            Some(&mac_str),
            device.product_id,
            self.local_lap,
            updates,
        )?;
        self.decoder = decoder;
        self.anc_sub = snapshot.noise.anc_sub;
        self.transp_sub = snapshot.noise.transp_sub;

        // Auto-apply saved equalizer preset if configured
        if let Some(preset) = snapshot.sound.equalizer_preset {
            if preset != 0 {
                let _ = apply_equalizer_preset(
                    stream,
                    &mut self.decoder,
                    snapshot.identity.product_id,
                    preset,
                    updates,
                );
            }
        }

        self.snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(POLL_INTERVAL)
    }

    fn periodic_tick<T: Read + Write>(
        &mut self,
        stream: &mut T,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let Some(snapshot) = &self.snapshot else {
            return Ok(());
        };
        let left = request(
            stream,
            &mut self.decoder,
            EarFunCommands::BATTERY_LEFT,
            vec![],
            Some(updates),
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
        let right = request(
            stream,
            &mut self.decoder,
            EarFunCommands::BATTERY_RIGHT,
            vec![],
            Some(updates),
        )
        .ok()
        .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten());
        let case = if snapshot.identity.product_id != EARFUN_AIR_S_PRODUCT_ID {
            request(
                stream,
                &mut self.decoder,
                EarFunCommands::BATTERY_CASE,
                vec![],
                Some(updates),
            )
            .ok()
            .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten())
        } else {
            None
        };

        if left.is_some() || right.is_some() || case.is_some() {
            let _ = updates.send(UiUpdate::Battery(BatteryState { left, right, case }));
        }

        // Periodic ambient sound poll to sync with earbud physical touches
        let has_submodes = snapshot.noise.modes.iter().any(|m| m.parent.is_some());
        if let Ok(pkt) = request(
            stream,
            &mut self.decoder,
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
                        stream,
                        &mut self.decoder,
                        EarFunCommands::GET_ANC_SUB_MODE,
                        vec![],
                        Some(updates),
                    ) {
                        if let Ok(sub) = parse_earfun_anc_sub_mode(&sub_pkt.payload) {
                            self.anc_sub = sub;
                        }
                    }
                    earfun_mode_to_protocol_index(1, self.anc_sub, self.transp_sub)
                } else if master == 2 {
                    if let Ok(sub_pkt) = request(
                        stream,
                        &mut self.decoder,
                        EarFunCommands::GET_TRANSPARENCY_SUB_MODE,
                        vec![],
                        Some(updates),
                    ) {
                        if let Ok(sub) = parse_earfun_transparency_sub_mode(&sub_pkt.payload) {
                            self.transp_sub = sub;
                        }
                    }
                    earfun_mode_to_protocol_index(2, self.anc_sub, self.transp_sub)
                } else {
                    0
                };
                let _ = updates.send(UiUpdate::NoiseMode(actual_idx));
            }
        }

        Ok(())
    }

    fn handle_command<T: Read + Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let Some(snapshot) = &self.snapshot else {
            return Ok(());
        };
        let pid = snapshot.identity.product_id;
        let current_dual_device = snapshot.dual_device.dual_device_enabled;
        let current_audio_codec = snapshot.sound.audio_codec;

        match command {
            WorkerCommand::Disconnect => Ok(()),
            WorkerCommand::SetNoiseMode(protocol_index) => {
                let _ = updates.send(UiUpdate::Status(crate::t!("noise-mode-waiting-confirm")));
                match set_noise_mode(
                    stream,
                    &mut self.decoder,
                    pid,
                    protocol_index,
                    &mut self.anc_sub,
                    &mut self.transp_sub,
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
                Ok(())
            }
            WorkerCommand::SetGameMode(enabled) => {
                match set_game_mode(stream, &mut self.decoder, enabled, updates) {
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
                Ok(())
            }
            WorkerCommand::SetInEarDetection(enabled) => {
                match set_in_ear_detection(stream, &mut self.decoder, enabled, updates) {
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
                Ok(())
            }
            WorkerCommand::SetDualDeviceConnection(enabled) => {
                match set_dual_device(
                    stream,
                    &mut self.decoder,
                    pid,
                    enabled,
                    current_audio_codec,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::DualDeviceConnectionEnabled(actual));
                        let controls_enabled = if actual {
                            Ok(true)
                        } else {
                            Err(crate::t!("dual-device-not-enabled"))
                        };
                        let _ =
                            updates.send(UiUpdate::PairedDeviceControlsEnabled(controls_enabled));
                        thread::sleep(Duration::from_millis(200));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: error,
                        });
                    }
                }
                Ok(())
            }
            WorkerCommand::SetAudioCodec(codec) => {
                match set_audio_codec(
                    stream,
                    &mut self.decoder,
                    pid,
                    codec,
                    current_dual_device,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::AudioCodec(actual));
                        thread::sleep(Duration::from_millis(200));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::AudioCodec,
                            message: error,
                        });
                    }
                }
                Ok(())
            }
            WorkerCommand::SetPairedDevice { address, action } => {
                let slot = address.command_bytes()[2];
                match action {
                    PairedDeviceAction::Connect => {
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("dual-device-waiting-update")));
                        if let Err(_e) =
                            send_command(stream, EarFunCommands::CONNECT_DEVICE, vec![slot])
                        {
                            let _ = updates.send(UiUpdate::Status(crate::t!(
                                "dual-device-send-connect-failed"
                            )));
                            let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                            return Ok(());
                        }
                        thread::sleep(Duration::from_millis(300));
                        match read_earfun_paired_devices(
                            stream,
                            &mut self.decoder,
                            self.local_lap,
                            Some(updates),
                        ) {
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
                            send_command(stream, EarFunCommands::DISCONNECT_DEVICE, vec![slot])
                        {
                            let _ = updates.send(UiUpdate::Status(crate::t!(
                                "dual-device-send-disconnect-failed"
                            )));
                            let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                            return Ok(());
                        }
                        thread::sleep(Duration::from_millis(300));
                        match read_earfun_paired_devices(
                            stream,
                            &mut self.decoder,
                            self.local_lap,
                            Some(updates),
                        ) {
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
                Ok(())
            }
            WorkerCommand::SetEqualizer(preset_index) => {
                match apply_equalizer_preset(
                    stream,
                    &mut self.decoder,
                    pid,
                    preset_index,
                    Some(updates),
                ) {
                    Ok(()) => {
                        let _ = crate::resources::save_equalizer_preset(
                            pid,
                            self.device_address.as_deref(),
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
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_incoming_bytes<T: Read + Write>(
        &mut self,
        _stream: &mut T,
        bytes: &[u8],
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let packets = self.decoder.push(bytes);
        for packet in packets {
            handle_unsolicited(&packet, Some(updates));
        }
        Ok(())
    }
}
