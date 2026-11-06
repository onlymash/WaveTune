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
use crate::compatibility;
use crate::protocol::brands::earfun::{
    EarFunCommands, EarFunEfFrameDecoder, EarFunFindMode, EarFunFrameDecoder, EarFunMicrophoneMode,
    parse_earfun_ambient_mode, parse_earfun_anc_sub_mode, parse_earfun_battery_component,
    parse_earfun_transparency_sub_mode,
};
use crate::protocol::{BatteryState, CustomEqualizer, EarSide, KeyFunction, PairedDeviceAction};

use super::features::{
    controls::{
        query_touch_actions, set_find_mode, set_in_ear_detection, set_microphone_mode,
        set_prompt_volume, set_touch_action, set_touch_lock,
    },
    dual_device::{read_earfun_paired_devices, set_dual_device},
    noise::{earfun_mode_to_protocol_index, set_noise_mode},
    sound::{apply_custom_equalizer, apply_equalizer_preset, set_audio_codec, set_game_mode},
};
use super::init::initialize_earfun_session_with_lap;
use super::transport::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, EARFUN_FREE_PRO_3_PRODUCT_ID,
    POLL_INTERVAL, get_local_rfcomm_lap, open_earfun_rfcomm, request, send_command,
};
use super::unsolicited::handle_unsolicited;

pub struct EarFunDriver {
    decoder: EarFunFrameDecoder,
    ef_decoder: EarFunEfFrameDecoder,
    ef_protocol: bool,
    snapshot: Option<DeviceSnapshot>,
    anc_sub: u8,
    transp_sub: u8,
    device_address: Option<String>,
    local_lap: Option<[u8; 3]>,
    custom_equalizers: Vec<CustomEqualizer>,
}

pub(super) fn has_gaia_identity(device: &DeviceChoice) -> bool {
    let has_gaia_product_id = device.product_id.is_some_and(|product_id| {
        [
            EARFUN_AIR_PRO_4_PRODUCT_ID,
            EARFUN_FREE_PRO_3_PRODUCT_ID,
            EARFUN_AIR_S_PRODUCT_ID,
        ]
        .contains(&product_id)
    });
    let has_gaia_control_service = device.control_service_uuids.iter().any(|uuid| {
        let uuid = uuid.to_string();
        uuid.eq_ignore_ascii_case(super::transport::GAIA_SERVICE_UUID)
    });

    has_gaia_product_id || has_gaia_control_service || device.has_earfun_gaia_vendor_service
}

impl EarFunDriver {
    pub fn new() -> Self {
        Self {
            decoder: EarFunFrameDecoder::default(),
            ef_decoder: EarFunEfFrameDecoder::default(),
            ef_protocol: false,
            snapshot: None,
            anc_sub: 2,
            transp_sub: 0,
            device_address: None,
            local_lap: None,
            custom_equalizers: Vec::new(),
        }
    }
}

impl Default for EarFunDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl EarFunDriver {
    pub fn set_snapshot_for_test(&mut self, product_id: u32) {
        let mut snapshot = DeviceSnapshot::default();
        snapshot.identity.product_id = product_id;
        self.snapshot = Some(snapshot);
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
        if !has_gaia_identity(device)
            && let Some(firmware_payload) = super::ef::probe_firmware(stream, &mut self.ef_decoder)
        {
            self.ef_protocol = true;
            let snapshot =
                super::ef::initialize(stream, &mut self.ef_decoder, device, firmware_payload)?;
            self.snapshot = Some(snapshot.clone());
            return Ok(snapshot);
        }
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
        self.custom_equalizers = snapshot.sound.custom_equalizers.clone();
        let behavior_pid = compatibility::earfun_behavior_product_id(
            snapshot.identity.product_id,
            snapshot.identity.firmware_model_code.as_deref(),
        );

        // Auto-apply saved equalizer preset if configured
        if let Some(preset) = snapshot.sound.equalizer_preset {
            if preset != 0 {
                if let Some(custom) = self.custom_equalizers.iter().find(|c| c.id == preset) {
                    let _ = apply_custom_equalizer(
                        stream,
                        &mut self.decoder,
                        behavior_pid,
                        &custom.bands,
                        updates,
                    );
                } else {
                    let _ = apply_equalizer_preset(
                        stream,
                        &mut self.decoder,
                        behavior_pid,
                        preset,
                        updates,
                    );
                }
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
        if self.ef_protocol {
            let Some(snapshot) = &self.snapshot else {
                return Ok(());
            };
            return super::ef::poll(stream, &mut self.ef_decoder, snapshot, updates);
        }
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
        let right = if compatibility::earfun_is_headphones(
            snapshot.identity.product_id,
            snapshot.identity.firmware_model_code.as_deref(),
        ) {
            None
        } else {
            request(
                stream,
                &mut self.decoder,
                EarFunCommands::BATTERY_RIGHT,
                vec![],
                Some(updates),
            )
            .ok()
            .and_then(|pkt| parse_earfun_battery_component(&pkt.payload).ok().flatten())
        };
        let case = if compatibility::earfun_has_case_battery(
            snapshot.identity.product_id,
            snapshot.identity.firmware_model_code.as_deref(),
        ) {
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
        if !snapshot.noise.modes.is_empty()
            && let Ok(pkt) = request(
                stream,
                &mut self.decoder,
                EarFunCommands::GET_AMBIENT_SOUND,
                vec![],
                Some(updates),
            )
        {
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
        if self.ef_protocol {
            let Some(snapshot) = &mut self.snapshot else {
                return Ok(());
            };
            return super::ef::handle_command(
                stream,
                &mut self.ef_decoder,
                snapshot,
                command,
                updates,
            );
        }
        let Some(snapshot) = &self.snapshot else {
            return Ok(());
        };
        let pid = snapshot.identity.product_id;
        let firmware_model_code = snapshot.identity.firmware_model_code.as_deref();
        let profile = compatibility::profile_for_identity(pid, firmware_model_code);
        let behavior_pid = compatibility::earfun_behavior_product_id(pid, firmware_model_code);
        let current_dual_device = snapshot.dual_device.dual_device_enabled;
        let current_audio_codec = snapshot.sound.audio_codec;
        let noise_controls_enabled =
            !snapshot.identity.read_only && !snapshot.noise.modes.is_empty();

        match command {
            WorkerCommand::Disconnect => Ok(()),
            WorkerCommand::SetNoiseMode(protocol_index) => {
                match set_noise_mode(
                    stream,
                    &mut self.decoder,
                    pid,
                    firmware_model_code,
                    protocol_index,
                    &mut self.anc_sub,
                    &mut self.transp_sub,
                    updates,
                ) {
                    Ok(actual) => {
                        let _ = updates.send(UiUpdate::NoiseModeWriteCompleted(actual));
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("noise-mode-command-sent")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::NoiseMode,
                            message: error,
                        });
                    }
                }
                let _ = updates.send(UiUpdate::NoiseControlsEnabled(noise_controls_enabled));
                Ok(())
            }
            WorkerCommand::SetGameMode(enabled) => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_game_mode())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::GameMode,
                        message: crate::t!("game-mode-blocked"),
                    });
                    return Ok(());
                }
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
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_wear_detection())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("wear-detection-blocked"),
                    });
                    return Ok(());
                }
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
            WorkerCommand::FindEarbuds(mode) => {
                let supported = compatibility::profile_for_identity(pid, firmware_model_code)
                    .is_some_and(|profile| profile.supports_find_device());
                if snapshot.identity.read_only || !supported {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-find-blocked"),
                    });
                    let actual = self
                        .snapshot
                        .as_ref()
                        .and_then(|state| state.controls.find_earbuds_mode);
                    let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                        requested: mode,
                        actual,
                    });
                    return Ok(());
                }
                if matches!(
                    mode,
                    crate::protocol::FindEarbudsMode::Left
                        | crate::protocol::FindEarbudsMode::Right
                ) && !compatibility::profile_for_identity(pid, firmware_model_code)
                    .is_some_and(|profile| profile.supports_find_device_sides())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-find-side-blocked"),
                    });
                    let previous = self
                        .snapshot
                        .as_ref()
                        .and_then(|state| state.controls.find_earbuds_mode);
                    let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                        requested: mode,
                        actual: previous,
                    });
                    return Ok(());
                }

                let target = match mode {
                    crate::protocol::FindEarbudsMode::Off => EarFunFindMode::Off,
                    crate::protocol::FindEarbudsMode::Left => EarFunFindMode::Left,
                    crate::protocol::FindEarbudsMode::Right => EarFunFindMode::Right,
                    crate::protocol::FindEarbudsMode::Both => EarFunFindMode::Both,
                };
                match set_find_mode(stream, &mut self.decoder, target, Some(updates)) {
                    Ok(actual) => {
                        if let Some(state) = &mut self.snapshot {
                            state.controls.is_finding_earbuds = actual.is_finding();
                            state.controls.find_earbuds_mode = Some(mode);
                        }
                        let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                            requested: mode,
                            actual: Some(mode),
                        });
                        let _ = updates.send(UiUpdate::Status(if actual.is_finding() {
                            crate::t!("controls-find-ringing")
                        } else {
                            crate::t!("controls-find-stopped")
                        }));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-find-failed", error),
                        });
                        let previous = self
                            .snapshot
                            .as_ref()
                            .and_then(|state| state.controls.find_earbuds_mode);
                        let _ = updates.send(UiUpdate::FindEarbudsModeResult {
                            requested: mode,
                            actual: previous,
                        });
                    }
                }
                Ok(())
            }
            WorkerCommand::SetPromptVolume(volume) => {
                let supported = compatibility::profile_for_identity(pid, firmware_model_code)
                    .is_some_and(|profile| profile.supports_prompt_volume());
                if snapshot.identity.read_only || !supported {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-prompt-volume-blocked"),
                    });
                    return Ok(());
                }
                match set_prompt_volume(stream, &mut self.decoder, volume, Some(updates)) {
                    Ok(actual) => {
                        if let Some(state) = &mut self.snapshot {
                            state.controls.prompt_volume = Some(actual);
                        }
                        let _ = updates.send(UiUpdate::PromptVolume(actual));
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "controls-prompt-volume-updated"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-prompt-volume-update-failed", error),
                        });
                        if let Some(volume) = self
                            .snapshot
                            .as_ref()
                            .and_then(|state| state.controls.prompt_volume)
                        {
                            let _ = updates.send(UiUpdate::PromptVolume(volume));
                        }
                    }
                }
                Ok(())
            }
            WorkerCommand::SetMicrophoneMode(value) => {
                let supported = compatibility::profile_for_identity(pid, firmware_model_code)
                    .is_some_and(|profile| profile.supports_microphone_mode());
                if snapshot.identity.read_only || !supported {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-microphone-mode-blocked"),
                    });
                    return Ok(());
                }
                let Some(mode) = EarFunMicrophoneMode::from_u8(value) else {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-microphone-mode-invalid"),
                    });
                    let current = self
                        .snapshot
                        .as_ref()
                        .and_then(|state| state.controls.microphone_mode);
                    let _ = updates.send(UiUpdate::MicrophoneMode(current));
                    return Ok(());
                };
                match set_microphone_mode(stream, &mut self.decoder, mode, Some(updates)) {
                    Ok(actual) => {
                        if let Some(state) = &mut self.snapshot {
                            state.controls.microphone_mode = Some(actual as u8);
                        }
                        let _ = updates.send(UiUpdate::MicrophoneMode(Some(actual as u8)));
                        let _ = updates.send(UiUpdate::Status(crate::t!(
                            "controls-microphone-mode-updated"
                        )));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-microphone-mode-update-failed", error),
                        });
                        let mode = self
                            .snapshot
                            .as_ref()
                            .and_then(|state| state.controls.microphone_mode);
                        let _ = updates.send(UiUpdate::MicrophoneMode(mode));
                    }
                }
                Ok(())
            }
            WorkerCommand::SetGesture {
                side,
                button: _,
                action,
                function,
            } => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_touch_customization())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-gesture-blocked"),
                    });
                    return Ok(());
                }
                let _ = updates.send(UiUpdate::Status(crate::t!("controls-gesture-updating")));
                match set_touch_action(
                    stream,
                    &mut self.decoder,
                    side,
                    action,
                    function,
                    Some(updates),
                ) {
                    Ok(()) => {
                        let mut updated = false;
                        if let Ok(gestures) =
                            query_touch_actions(stream, &mut self.decoder, Some(updates))
                        {
                            if !gestures.is_empty() {
                                if let Some(st) = &mut self.snapshot {
                                    st.controls.key_functions = gestures.clone();
                                }
                                let _ = updates.send(UiUpdate::KeyFunctions(gestures));
                                updated = true;
                            }
                        }
                        if !updated {
                            if let Some(st) = &mut self.snapshot {
                                if let Some(existing) =
                                    st.controls.key_functions.iter_mut().find(|k| {
                                        (k.side == side || side == EarSide::Both)
                                            && k.action == action
                                    })
                                {
                                    existing.function = function;
                                } else {
                                    st.controls.key_functions.push(KeyFunction {
                                        side,
                                        button: 1,
                                        action,
                                        function,
                                    });
                                }
                                let _ = updates.send(UiUpdate::KeyFunctions(
                                    st.controls.key_functions.clone(),
                                ));
                            }
                        }
                        let _ =
                            updates.send(UiUpdate::Status(crate::t!("controls-gesture-updated")));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Controls,
                            message: crate::t!("controls-gesture-update-failed", error),
                        });
                        if let Some(st) = &self.snapshot {
                            let _ = updates
                                .send(UiUpdate::KeyFunctions(st.controls.key_functions.clone()));
                        }
                    }
                }
                Ok(())
            }
            WorkerCommand::SetTouchLock(locked) => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_touch_lock())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("controls-gesture-blocked"),
                    });
                    return Ok(());
                }
                match set_touch_lock(stream, &mut self.decoder, locked, Some(updates)) {
                    Ok(actual) => {
                        if let Some(st) = &mut self.snapshot {
                            st.controls.touch_lock_enabled = Some(actual);
                        }
                        let _ = updates.send(UiUpdate::TouchLockEnabled(actual));
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
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_dual_device())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("dual-device-toggle-blocked"),
                    });
                    return Ok(());
                }
                match set_dual_device(
                    stream,
                    &mut self.decoder,
                    behavior_pid,
                    enabled,
                    current_audio_codec,
                    compatibility::earfun_dual_device_restart_required(pid, firmware_model_code),
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
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_audio_codec())
                    || !compatibility::supported_codecs_for_identity(pid, firmware_model_code)
                        .contains(&codec)
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::AudioCodec,
                        message: crate::t!("audio-codec-blocked"),
                    });
                    return Ok(());
                }
                match set_audio_codec(
                    stream,
                    &mut self.decoder,
                    behavior_pid,
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
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_dual_device())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Controls,
                        message: crate::t!("dual-device-management-blocked"),
                    });
                    let _ = updates.send(UiUpdate::PairedDeviceActionFinished);
                    return Ok(());
                }
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
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_equalizer())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-blocked"),
                    });
                    return Ok(());
                }
                match apply_equalizer_preset(
                    stream,
                    &mut self.decoder,
                    behavior_pid,
                    preset_index,
                    Some(updates),
                ) {
                    Ok(()) => {
                        for c in &mut self.custom_equalizers {
                            c.is_selected = false;
                        }
                        let _ = crate::resources::save_custom_equalizers(
                            behavior_pid,
                            self.device_address.as_deref(),
                            &self.custom_equalizers,
                        );
                        let _ = crate::resources::save_equalizer_preset(
                            behavior_pid,
                            self.device_address.as_deref(),
                            preset_index,
                        );
                        let _ = updates
                            .send(UiUpdate::CustomEqualizers(self.custom_equalizers.clone()));
                        let _ = updates.send(UiUpdate::Equalizer(preset_index));
                    }
                    Err(error) => {
                        let _ = updates.send(UiUpdate::SettingError {
                            domain: SettingDomain::Equalizer,
                            message: crate::t!("equalizer-switch-failed", error),
                        });
                        let _ = updates.send(UiUpdate::Equalizer(
                            self.snapshot
                                .as_ref()
                                .and_then(|s| s.sound.equalizer_preset)
                                .unwrap_or(0),
                        ));
                    }
                }
                Ok(())
            }
            WorkerCommand::SelectCustomEqualizer(id) => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_custom_equalizer())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    return Ok(());
                }
                let custom_eq = self.custom_equalizers.iter().find(|c| c.id == id).cloned();

                if let Some(eq) = custom_eq {
                    let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-switching")));
                    match apply_custom_equalizer(
                        stream,
                        &mut self.decoder,
                        behavior_pid,
                        &eq.bands,
                        Some(updates),
                    ) {
                        Ok(()) => {
                            for c in &mut self.custom_equalizers {
                                c.is_selected = c.id == id;
                            }
                            let _ = crate::resources::save_custom_equalizers(
                                behavior_pid,
                                self.device_address.as_deref(),
                                &self.custom_equalizers,
                            );
                            let _ = crate::resources::save_equalizer_preset(
                                behavior_pid,
                                self.device_address.as_deref(),
                                id,
                            );
                            let _ = updates
                                .send(UiUpdate::CustomEqualizers(self.custom_equalizers.clone()));
                            let _ = updates.send(UiUpdate::Equalizer(id));
                            let _ = updates
                                .send(UiUpdate::Status(crate::t!("equalizer-custom-applied")));
                        }
                        Err(error) => {
                            let _ = updates.send(UiUpdate::SettingError {
                                domain: SettingDomain::Equalizer,
                                message: crate::t!("equalizer-custom-switch-failed", error),
                            });
                            let cur = self
                                .snapshot
                                .as_ref()
                                .and_then(|s| s.sound.equalizer_preset)
                                .unwrap_or(0);
                            let _ = updates.send(UiUpdate::Equalizer(cur));
                        }
                    }
                }
                Ok(())
            }
            WorkerCommand::AddCustomEqualizer { name, bands } => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_custom_equalizer())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    return Ok(());
                }
                if self.custom_equalizers.len() >= 3 {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-max-reached"),
                    });
                    return Ok(());
                }

                let mut next_id = 100u8;
                while self.custom_equalizers.iter().any(|c| c.id == next_id) {
                    next_id = next_id.saturating_add(1);
                }

                let new_eq = CustomEqualizer {
                    id: next_id,
                    name,
                    min_gain: -6,
                    max_gain: 6,
                    is_selected: false,
                    bands,
                };

                self.custom_equalizers.push(new_eq);
                let _ = crate::resources::save_custom_equalizers(
                    behavior_pid,
                    self.device_address.as_deref(),
                    &self.custom_equalizers,
                );
                let _ = updates.send(UiUpdate::CustomEqualizers(self.custom_equalizers.clone()));
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-added")));
                Ok(())
            }
            WorkerCommand::SaveCustomEqualizer { id, name, bands } => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_custom_equalizer())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    return Ok(());
                }
                let mut is_active = false;
                if let Some(target) = self.custom_equalizers.iter_mut().find(|c| c.id == id) {
                    target.name = name;
                    target.bands = bands.clone();
                    is_active = target.is_selected;
                }

                let _ = crate::resources::save_custom_equalizers(
                    behavior_pid,
                    self.device_address.as_deref(),
                    &self.custom_equalizers,
                );
                let _ = updates.send(UiUpdate::CustomEqualizers(self.custom_equalizers.clone()));

                // If currently selected, re-apply the modified bands to the earbud
                if is_active {
                    let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saving")));
                    match apply_custom_equalizer(
                        stream,
                        &mut self.decoder,
                        behavior_pid,
                        &bands,
                        Some(updates),
                    ) {
                        Ok(()) => {
                            let _ =
                                updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saved")));
                        }
                        Err(error) => {
                            let _ = updates.send(UiUpdate::SettingError {
                                domain: SettingDomain::Equalizer,
                                message: crate::t!("equalizer-custom-save-failed", error),
                            });
                        }
                    }
                } else {
                    let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-saved")));
                }
                Ok(())
            }
            WorkerCommand::DeleteCustomEqualizer(id) => {
                if snapshot.identity.read_only
                    || !profile.is_some_and(|profile| profile.supports_custom_equalizer())
                {
                    let _ = updates.send(UiUpdate::SettingError {
                        domain: SettingDomain::Equalizer,
                        message: crate::t!("equalizer-custom-blocked"),
                    });
                    return Ok(());
                }
                let was_active = self
                    .custom_equalizers
                    .iter()
                    .find(|c| c.id == id)
                    .is_some_and(|c| c.is_selected);

                self.custom_equalizers.retain(|c| c.id != id);

                let _ = crate::resources::save_custom_equalizers(
                    behavior_pid,
                    self.device_address.as_deref(),
                    &self.custom_equalizers,
                );
                let _ = updates.send(UiUpdate::CustomEqualizers(self.custom_equalizers.clone()));
                let _ = updates.send(UiUpdate::Status(crate::t!("equalizer-custom-deleted")));

                if was_active {
                    let _ = apply_equalizer_preset(
                        stream,
                        &mut self.decoder,
                        behavior_pid,
                        0,
                        Some(updates),
                    );
                    let _ = crate::resources::save_equalizer_preset(
                        behavior_pid,
                        self.device_address.as_deref(),
                        0,
                    );
                    let _ = updates.send(UiUpdate::Equalizer(0));
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
        if self.ef_protocol {
            super::ef::handle_incoming_bytes(
                &mut self.ef_decoder,
                &mut self.snapshot,
                bytes,
                updates,
            );
            return Ok(());
        }
        let packets = self.decoder.push(bytes);
        for packet in packets {
            handle_unsolicited(&packet, Some(updates));
        }
        Ok(())
    }
}
