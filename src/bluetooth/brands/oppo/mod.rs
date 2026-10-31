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

pub mod driver;
pub mod features;
pub mod init;
pub mod transport;
pub mod unsolicited;

pub use driver::OppoDriver;
#[allow(unused_imports)]
pub use features::{
    dual_device::set_paired_device,
    noise::set_noise_mode,
    sound::{
        set_bass_engine_enabled, set_bass_engine_level, set_equalizer, set_game_mode, set_hi_res,
        set_spatial_audio,
    },
};
#[allow(unused_imports)]
pub use init::{
    OppoDeviceSnapshot, initialize_oppo_session, initialize_session, probe_read_only_features,
};
#[allow(unused_imports)]
pub use transport::request;
#[allow(unused_imports)]
pub(crate) use transport::{open_oppo_rfcomm, oppo_candidate_service_uuids};

use std::{
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, worker::SessionRunner},
};

pub fn spawn_oppo_monitor(
    device: DeviceChoice,
    updates: Sender<UiUpdate>,
    commands: Receiver<WorkerCommand>,
) -> std::io::Result<JoinHandle<()>> {
    SessionRunner::spawn(device, OppoDriver::new(), updates, commands)
}

#[allow(unused_imports)]
pub use spawn_oppo_monitor as spawn_monitor;

// Backwards compatibility alias for older test paths
#[doc(hidden)]
#[allow(unused_imports)]
pub mod oppo_session {
    pub use super::transport::request;
}
