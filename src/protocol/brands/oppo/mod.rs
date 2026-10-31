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

pub mod oppo_capabilities;
pub mod oppo_commands;
pub mod oppo_controls;
pub mod oppo_device;
pub mod oppo_frame;
pub mod oppo_noise;
mod oppo_packet;
pub mod oppo_sound;
pub mod oppo_types;
pub mod oppo_uuids;

#[cfg(test)]
mod tests;

pub use oppo_capabilities::{
    OppoCapabilityBitmap, capability_bitmap_allows, command_capability_bit,
};
pub use oppo_commands::OppoCommands;
pub use oppo_controls::*;
pub use oppo_device::*;
pub use oppo_frame::{
    FRAME_START, FrameDecoder, OPPO_FRAME_START, OppoFrameDecoder, encode_frame, encode_oppo_frame,
};
pub use oppo_noise::*;
pub use oppo_sound::*;
pub use oppo_types::{AppPacket, OppoAppPacket, OppoProtocolError, ProtocolError};
pub use oppo_uuids::{
    ONEPLUS_COMPANY_ID, OPPO_COMPANY_ID, OPPO_CONTROL_SERVICE_CANDIDATES,
    OPPO_CONTROL_SERVICE_UUID, OPPO_PRIMARY_CONTROL_SERVICE_UUID,
    OPPO_SECONDARY_CONTROL_SERVICE_UUID, REALME_COMPANY_ID, STANDARD_SPP_SERVICE_UUID,
    is_candidate_control_service_uuid,
};
