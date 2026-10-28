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

pub mod oppo_session;

#[allow(unused_imports)]
pub use oppo_session::{
    DeviceSnapshot, OppoDeviceSnapshot, initialize_oppo_session, initialize_session,
    probe_read_only_features, set_bass_engine_enabled, set_bass_engine_level, set_equalizer,
    set_game_mode, set_hi_res, set_noise_mode, set_paired_device, set_spatial_audio, spawn_monitor,
    spawn_oppo_monitor,
};
