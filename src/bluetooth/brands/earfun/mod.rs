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

#[cfg(test)]
mod tests;

use std::{
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};

use crate::{
    app::events::{UiUpdate, WorkerCommand},
    bluetooth::{DeviceChoice, worker::SessionRunner},
};

pub use driver::EarFunDriver;
#[allow(unused_imports)]
pub use features::{
    controls::set_in_ear_detection,
    dual_device::{read_earfun_paired_devices, set_dual_device},
    noise::{earfun_mode_to_protocol_index, protocol_index_to_earfun_mode, set_noise_mode},
    sound::{apply_equalizer_preset, set_audio_codec, set_game_mode},
};
#[allow(unused_imports)]
pub use init::{
    EarFunDeviceSnapshot, initialize_earfun_session, initialize_earfun_session_with_lap,
    parse_earfun_product_id_from_firmware, probe_earfun,
};
#[allow(unused_imports)]
pub use transport::{
    EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, EARFUN_FALLBACK_RFCOMM_CHANNEL,
    EARFUN_FREE_PRO_3_PRODUCT_ID, EARFUN_PRIMARY_SPP_NAMES, EARFUN_SECONDARY_SPP_NAMES,
    GAIA_SERVICE_UUID, POLL_INTERVAL, REQUEST_TIMEOUT, get_local_rfcomm_lap, open_earfun_rfcomm,
    request, send_command,
};

#[allow(dead_code)]
pub fn spawn_earfun_monitor(
    device: DeviceChoice,
    updates: Sender<UiUpdate>,
    commands: Receiver<WorkerCommand>,
) -> std::io::Result<JoinHandle<()>> {
    SessionRunner::spawn(device, EarFunDriver::new(), updates, commands)
}

#[allow(unused_imports)]
pub use spawn_earfun_monitor as spawn_monitor;

// Backwards compatibility module for existing tests
#[doc(hidden)]
#[allow(unused_imports)]
pub mod earfun_session {
    pub use super::{
        EARFUN_AIR_PRO_4_PRODUCT_ID, EARFUN_AIR_S_PRODUCT_ID, EARFUN_FALLBACK_RFCOMM_CHANNEL,
        EARFUN_FREE_PRO_3_PRODUCT_ID, EARFUN_PRIMARY_SPP_NAMES, EARFUN_SECONDARY_SPP_NAMES,
        EarFunDeviceSnapshot, EarFunDriver, GAIA_SERVICE_UUID, POLL_INTERVAL, REQUEST_TIMEOUT,
        apply_equalizer_preset, earfun_mode_to_protocol_index, get_local_rfcomm_lap,
        initialize_earfun_session, initialize_earfun_session_with_lap, open_earfun_rfcomm,
        parse_earfun_product_id_from_firmware, probe_earfun, protocol_index_to_earfun_mode,
        read_earfun_paired_devices, request, send_command, set_audio_codec, set_dual_device,
        set_game_mode, set_in_ear_detection, set_noise_mode, spawn_earfun_monitor,
    };
}
