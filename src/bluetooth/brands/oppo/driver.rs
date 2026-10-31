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
    compatibility::{self, DeviceProfile, NoiseMode},
    protocol::{
        AppPacket, BassEngineLevel, CustomEqualizer, FrameDecoder, KeyFunction, NoiseCycleState,
        PairedDevice, PairedDeviceAddress, WearState, parse_paired_devices_event, parse_wear_event,
    },
};

use super::{
    features::{
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
    game_mode_feature_id: u8,
    bass_engine_level: Option<BassEngineLevel>,
    is_finding_earbuds: bool,
    current_custom_equalizers: Vec<CustomEqualizer>,
    current_key_functions: Vec<KeyFunction>,
    current_noise_cycle: NoiseCycleState,
    current_prompt_volume: Option<u8>,
    current_head_motion: Option<u8>,
    current_dual_device: Option<bool>,
    current_in_ear_detection: Option<bool>,
    current_sleep_detection: Option<bool>,
    current_preferred_device: Option<PairedDeviceAddress>,
    paired_devices: Result<Vec<PairedDevice>, String>,
    controls_enabled: Result<bool, String>,
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
            game_mode_feature_id: 0x06,
            bass_engine_level: None,
            is_finding_earbuds: false,
            current_custom_equalizers: Vec::new(),
            current_key_functions: Vec::new(),
            current_noise_cycle: NoiseCycleState::default(),
            current_prompt_volume: None,
            current_head_motion: None,
            current_dual_device: None,
            current_in_ear_detection: None,
            current_sleep_detection: None,
            current_preferred_device: None,
            paired_devices: Ok(Vec::new()),
            controls_enabled: Ok(false),
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
        self.game_mode_feature_id = snapshot.sound.game_mode_feature_id;
        self.bass_engine_level = snapshot.sound.bass_engine_level.clone();
        self.is_finding_earbuds = snapshot.controls.is_finding_earbuds;
        self.current_custom_equalizers = snapshot.sound.custom_equalizers.clone();
        self.current_key_functions = snapshot.controls.key_functions.clone();
        self.current_noise_cycle = snapshot.controls.noise_cycle.clone();
        self.current_prompt_volume = snapshot.controls.prompt_volume;
        self.current_head_motion = snapshot.controls.head_motion_type;
        self.current_dual_device = snapshot.dual_device.dual_device_enabled;
        self.current_in_ear_detection = snapshot.controls.in_ear_detection_enabled;
        self.current_sleep_detection = snapshot.controls.sleep_detection_enabled;
        self.current_preferred_device = snapshot.dual_device.preferred_device;
        self.paired_devices = snapshot.dual_device.paired_devices.clone();
        self.controls_enabled = snapshot.dual_device.controls_enabled.clone();

        Ok(snapshot)
    }

    fn handle_command<T: Read + Write>(
        &mut self,
        stream: &mut T,
        command: WorkerCommand,
        updates: &Sender<UiUpdate>,
    ) -> Result<(), String> {
        let modes = self
            .profile
            .map_or(&[] as &[NoiseMode], |profile| profile.noise_modes);
        if handle_noise_command(
            stream,
            &mut self.decoder,
            &command,
            self.read_only,
            &self.supported_indices,
            modes,
            &mut self.wear,
            self.supports_wear_detection,
            updates,
        ) {
            return Ok(());
        }

        let supports_equalizer = self.profile.is_some_and(|p| p.supports_equalizer());
        let supports_custom_equalizer = self.profile.is_some_and(|p| p.supports_custom_equalizer());
        let supports_spatial_audio = self.profile.is_some_and(|p| p.supports_spatial_audio());
        let supports_game_mode = self.profile.is_some_and(|p| p.supports_game_mode());
        let supports_bass_engine = self.profile.is_some_and(|p| p.supports_bass_engine());
        let supports_hi_res = self.profile.is_some_and(|p| p.supports_hi_res());
        if handle_sound_command(
            stream,
            &mut self.decoder,
            &command,
            self.read_only,
            supports_equalizer,
            supports_custom_equalizer,
            &mut self.current_custom_equalizers,
            supports_spatial_audio,
            supports_game_mode,
            self.game_mode_feature_id,
            supports_bass_engine,
            &mut self.bass_engine_level,
            supports_hi_res,
            updates,
        ) {
            return Ok(());
        }

        let supports_touch_customization = self
            .profile
            .is_some_and(|p| p.supports_touch_customization());
        let noise_modes_empty = self.profile.map_or(true, |p| p.noise_modes.is_empty());
        let supports_find_device = self.profile.is_some_and(|p| p.supports_find_device());
        let supports_fit_detection = self.profile.is_some_and(|p| p.supports_fit_detection());
        let supports_prompt_volume = self.profile.is_some_and(|p| p.supports_prompt_volume());
        let supports_head_motion = self.profile.is_some_and(|p| p.supports_head_motion());
        let supports_sleep_detection = self.profile.is_some_and(|p| p.supports_sleep_detection());
        if handle_controls_command(
            stream,
            &mut self.decoder,
            &command,
            self.read_only,
            supports_touch_customization,
            &mut self.current_key_functions,
            noise_modes_empty,
            &mut self.current_noise_cycle,
            supports_find_device,
            &mut self.is_finding_earbuds,
            supports_fit_detection,
            &self.wear,
            supports_prompt_volume,
            &mut self.current_prompt_volume,
            supports_head_motion,
            &mut self.current_head_motion,
            self.supports_wear_detection,
            &mut self.current_in_ear_detection,
            supports_sleep_detection,
            &mut self.current_sleep_detection,
            updates,
        ) {
            return Ok(());
        }

        let supports_dual_device = self.profile.is_some_and(|p| p.supports_dual_device());
        if handle_dual_device_command(
            stream,
            &mut self.decoder,
            &command,
            self.read_only,
            supports_dual_device,
            &mut self.controls_enabled,
            &mut self.current_dual_device,
            &mut self.paired_devices,
            &mut self.current_preferred_device,
            updates,
        ) {
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
                    self.paired_devices = Ok(devices);
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
