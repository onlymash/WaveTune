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
pub mod oppo_frame;
pub mod oppo_noise;
mod oppo_packet;
pub mod oppo_parsers;
pub mod oppo_types;
pub mod oppo_uuids;

#[cfg(test)]
mod tests;

pub use oppo_capabilities::{
    OppoCapabilityBitmap, capability_bitmap_allows, command_capability_bit,
};
pub use oppo_commands::OppoCommands;
pub use oppo_frame::{
    FRAME_START, FrameDecoder, OPPO_FRAME_START, OppoFrameDecoder, encode_frame, encode_oppo_frame,
};
pub use oppo_noise::{
    encode_noise_mode, encode_oppo_noise_mode, parse_noise_mode_event, parse_noise_mode_response,
    parse_oppo_noise_mode_event, parse_oppo_noise_mode_response,
    parse_oppo_supported_noise_modes_response, parse_supported_noise_modes_response,
};
pub use oppo_parsers::{
    FEATURE_BASS_ENGINE, FEATURE_HI_RES, encode_custom_equalizer_command, encode_find_earbuds,
    encode_fit_test, encode_oppo_custom_equalizer, encode_oppo_custom_equalizer_command,
    encode_oppo_find_earbuds, encode_oppo_fit_test, encode_oppo_set_head_motion_type,
    encode_oppo_set_key_function, encode_oppo_set_noise_cycle, encode_oppo_set_priority_device,
    encode_oppo_set_prompt_volume, encode_set_head_motion_type, encode_set_key_function,
    encode_set_noise_cycle, encode_set_priority_device, encode_set_prompt_volume,
    parse_bass_engine_event, parse_bass_engine_response, parse_battery_event,
    parse_battery_response, parse_color_response, parse_custom_equalizer_list,
    parse_equalizer_event, parse_equalizer_response, parse_feature_flags_response,
    parse_firmware_response, parse_fit_test_event, parse_head_motion_type_response,
    parse_noise_cycle_response, parse_oppo_bass_engine_event, parse_oppo_bass_engine_response,
    parse_oppo_battery_event, parse_oppo_battery_response, parse_oppo_color_response,
    parse_oppo_custom_equalizer_list, parse_oppo_equalizer_event, parse_oppo_equalizer_response,
    parse_oppo_feature_flags_response, parse_oppo_firmware_response, parse_oppo_fit_test_event,
    parse_oppo_head_motion_type_response, parse_oppo_noise_cycle_response,
    parse_oppo_paired_devices_event, parse_oppo_paired_devices_response,
    parse_oppo_priority_device_response, parse_oppo_prompt_volume_response,
    parse_oppo_spatial_audio_response, parse_oppo_touch_controls_response, parse_oppo_wear_event,
    parse_oppo_wear_response, parse_paired_devices_event, parse_paired_devices_response,
    parse_priority_device_response, parse_prompt_volume_response, parse_spatial_audio_response,
    parse_touch_controls_response, parse_wear_event, parse_wear_response,
};

pub use oppo_types::{AppPacket, OppoAppPacket, OppoProtocolError, ProtocolError};
pub use oppo_uuids::{
    ONEPLUS_COMPANY_ID, OPPO_COMPANY_ID, OPPO_CONTROL_SERVICE_CANDIDATES,
    OPPO_CONTROL_SERVICE_UUID, OPPO_PRIMARY_CONTROL_SERVICE_UUID,
    OPPO_SECONDARY_CONTROL_SERVICE_UUID, REALME_COMPANY_ID, STANDARD_SPP_SERVICE_UUID,
    is_candidate_control_service_uuid,
};
