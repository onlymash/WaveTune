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
};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, DeviceSnapshot, driver::HeadsetDriver},
    compatibility::{self, DeviceProfile},
    protocol::{AppPacket, FrameDecoder, WearState, parse_paired_devices_event, parse_wear_event},
};

use super::{
    features::{
        OppoControlsState, OppoDualDeviceState, OppoFeatureContext, OppoSoundState,
        controls::handle_controls_command, dual_device::handle_dual_device_command,
        noise::handle_noise_command, sound::handle_sound_command,
    },
    init::initialize_oppo_session,
    transport::open_oppo_rfcomm,
    unsolicited::{handle_oppo_unsolicited, merge_wear_state, publish_read_only_event},
};

pub struct OppoDriver {
    decoder: FrameDecoder,
    read_only: bool,
    is_known_model: bool,
    profile: Option<&'static DeviceProfile>,
    supported_indices: Vec<u8>,
    supports_wear_detection: bool,
    wear: WearState,
    pub sound: OppoSoundState,
    pub controls: OppoControlsState,
    pub dual_device: OppoDualDeviceState,
}

impl Default for OppoDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl OppoDriver {
    pub fn new() -> Self {
        Self {
            decoder: FrameDecoder::default(),
            read_only: false,
            is_known_model: false,
            profile: None,
            supported_indices: Vec::new(),
            supports_wear_detection: false,
            wear: WearState::default(),
            sound: OppoSoundState::default(),
            controls: OppoControlsState::default(),
            dual_device: OppoDualDeviceState::default(),
        }
    }
}

impl HeadsetDriver for OppoDriver {
    fn brand_name(&self) -> &'static str {
        "oppo"
    }

    fn open_rfcomm(&mut self, device: &DeviceChoice) -> std::io::Result<File> {
        open_oppo_rfcomm(device)
    }

    fn initialize<T: Read + Write>(
        &mut self,
        stream: &mut T,
        device: &DeviceChoice,
        updates: Option<&Sender<UiUpdate>>,
    ) -> Result<DeviceSnapshot, String> {
        let (snapshot, decoder) = initialize_oppo_session(stream, &device.name, updates)?;
        self.decoder = decoder;
        self.read_only = snapshot.identity.read_only;
        self.is_known_model = snapshot.identity.is_known_model;
        self.profile = compatibility::profile_for_product_id(snapshot.identity.product_id);
        self.supported_indices = snapshot.noise.supported_indices.clone();
        self.supports_wear_detection = snapshot.wear.supports_wear_detection;
        self.wear = snapshot.wear.wear.clone();
        self.sound = OppoSoundState {
            custom_equalizers: snapshot.sound.custom_equalizers.clone(),
            bass_engine_level: snapshot.sound.bass_engine_level.clone(),
            game_mode_feature_id: snapshot.sound.game_mode_feature_id,
        };
        self.controls = OppoControlsState {
            key_functions: snapshot.controls.key_functions.clone(),
            noise_cycle: snapshot.controls.noise_cycle.clone(),
            is_finding_earbuds: snapshot.controls.is_finding_earbuds,
            prompt_volume: snapshot.controls.prompt_volume,
            head_motion: snapshot.controls.head_motion_type,
            in_ear_detection: snapshot.controls.in_ear_detection_enabled,
            sleep_detection: snapshot.controls.sleep_detection_enabled,
        };
        self.dual_device = OppoDualDeviceState {
            dual_device_enabled: snapshot.dual_device.dual_device_enabled,
            preferred_device: snapshot.dual_device.preferred_device,
            paired_devices: snapshot.dual_device.paired_devices.clone(),
            controls_enabled: snapshot.dual_device.controls_enabled.clone(),
        };

        Ok(snapshot)
    }

    fn handle_command<T: Read + Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let mut ctx = OppoFeatureContext {
            stream,
            decoder: &mut self.decoder,
            updates,
            read_only: self.read_only,
            profile: self.profile,
            supported_indices: &self.supported_indices,
            supports_wear_detection: self.supports_wear_detection,
            wear: &mut self.wear,
        };

        if handle_noise_command(&mut ctx, &command) {
            return Ok(());
        }
        if handle_sound_command(&mut ctx, &command, &mut self.sound) {
            return Ok(());
        }
        if handle_controls_command(&mut ctx, &command, &mut self.controls) {
            return Ok(());
        }
        if handle_dual_device_command(&mut ctx, &command, &mut self.dual_device) {
            return Ok(());
        }

        Ok(())
    }

    fn handle_incoming_bytes<T: Read + Write>(
        &mut self,
        stream: &mut T,
        bytes: &[u8],
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let packets = self
            .decoder
            .push(bytes)
            .map_err(|error| crate::t!("protocol-frame-decode-failed", error))?;
        for frame_bytes in packets {
            if let Ok(packet) = AppPacket::decode(&frame_bytes) {
                if self.read_only {
                    if self.is_known_model
                        && packet.command == 0x0204
                        && matches!(packet.payload.first(), Some(1 | 2))
                    {
                        if packet.payload.first() == Some(&2)
                            && let Ok(update) = parse_wear_event(&packet.payload)
                        {
                            merge_wear_state(&mut self.wear, update);
                        }
                        publish_read_only_event(&packet, updates);
                    }
                    continue;
                }
                if packet.command == 0x0204
                    && packet.payload.first() == Some(&6)
                    && let Ok(devices) = parse_paired_devices_event(&packet.payload)
                {
                    self.dual_device.paired_devices = Ok(devices);
                }
                if packet.command == 0x0204
                    && packet.payload.first() == Some(&2)
                    && let Ok(update) = parse_wear_event(&packet.payload)
                {
                    merge_wear_state(&mut self.wear, update);
                }
                handle_oppo_unsolicited(stream, &packet, Some(updates));
            }
        }
        Ok(())
    }
}
