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

use crate::{
    bluetooth::DeviceChoice,
    compatibility::{EqualizerPreset, GestureRow, NoiseMode},
    protocol::{
        BassEngineLevel, BatteryState, CustomEqualizer, EarSide, FirmwareVersions, FitTestResult,
        GestureAction, KeyFunction, NoiseCycleState, PairedDevice, PairedDeviceAction,
        PairedDeviceAddress, WearState,
    },
};

#[derive(Debug, Clone)]
pub enum UiUpdate {
    Status(String),
    Devices(Result<Vec<DeviceChoice>, String>),
    Connecting {
        device: DeviceChoice,
    },
    Connected {
        device: DeviceChoice,
    },
    Battery(BatteryState),
    PairedDevices(Result<Vec<PairedDevice>, String>),
    PairedDeviceControlsEnabled(Result<bool, String>),
    PairedDeviceActionFinished,
    Firmware(FirmwareVersions),
    Wear(WearState),
    ProductId {
        product_id: u32,
        color_id: Option<u8>,
    },
    DeviceArtwork(std::path::PathBuf),
    DeviceModel(std::path::PathBuf),
    DeviceScene(std::path::PathBuf),
    NoiseMode(u8),
    NoiseControlsEnabled(bool),
    NoiseModeOptions {
        indices: Vec<u8>,
        note: String,
        modes: Vec<NoiseMode>,
    },
    Equalizer(u8),
    CustomEqualizers(Vec<CustomEqualizer>),
    SpatialAudio(u8),
    GameMode(bool),
    BassEngineEnabled(bool),
    BassEngineLevel(BassEngineLevel),
    HiRes(bool),
    SoundFeatures {
        supports_equalizer: bool,
        supports_custom_equalizer: bool,
        supports_spatial_audio: bool,
        supports_head_tracking: bool,
        supports_game_mode: bool,
        supports_bass_engine: bool,
        supports_hi_res: bool,
        presets: Vec<EqualizerPreset>,
    },
    KeyFunctions(Vec<KeyFunction>),
    NoiseCycle(NoiseCycleState),
    IsFindingEarbuds(bool),
    FitTestRunning(bool),
    FitTestResult(FitTestResult),
    PromptVolume(u8),
    HeadMotionType(u8),
    DualDeviceConnectionEnabled(bool),
    InEarDetectionEnabled(bool),
    SleepDetectionEnabled(bool),
    PreferredDevice(Option<PairedDeviceAddress>),
    ControlsFeatures {
        supports_touch_customization: bool,
        supports_dual_device: bool,
        supports_wear_detection: bool,
        supports_sleep_detection: bool,
        supports_head_motion: bool,
        supports_find_device: bool,
        supports_fit_detection: bool,
        supports_prompt_volume: bool,
        gesture_rows: Vec<GestureRow>,
    },
    Finished,
}

#[derive(Debug, Clone)]
pub enum WorkerCommand {
    Disconnect,
    SetNoiseMode(u8),
    SetPairedDevice {
        address: PairedDeviceAddress,
        action: PairedDeviceAction,
    },
    SetEqualizer(u8),
    SelectCustomEqualizer(u8),
    AddCustomEqualizer {
        name: String,
        bands: Vec<(u16, i8)>,
    },
    SaveCustomEqualizer {
        id: u8,
        name: String,
        bands: Vec<(u16, i8)>,
    },
    DeleteCustomEqualizer(u8),
    SetSpatialAudio(u8),
    SetGameMode(bool),
    #[allow(dead_code)]
    SetBassEngineEnabled(bool),
    #[allow(dead_code)]
    SetBassEngineLevel(i8),
    #[allow(dead_code)]
    SetHiRes(bool),
    SetGesture {
        side: EarSide,
        button: u8,
        action: GestureAction,
        function: u8,
    },
    SetNoiseCycle {
        side: EarSide,
        mask: u32,
        is_shared: bool,
    },
    FindEarbuds(bool),
    RunFitTest,
    SetPromptVolume(u8),
    SetHeadMotionType(u8),
    SetDualDeviceConnection(bool),
    SetInEarDetection(bool),
    SetSleepDetection(bool),
    SetPreferredDevice(Option<PairedDeviceAddress>),
}
