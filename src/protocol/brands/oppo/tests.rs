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

use super::*;
use crate::protocol::core::types::{
    BatteryPart, BatteryState, CustomEqualizerAction, CustomEqualizerParams, FirmwareVersions,
    NoiseCycleSetting, PairedDevice, PairedDeviceAction, PairedDeviceAddress, WearPart, WearState,
};

#[test]
fn battery_packet_matches_reference_frame() {
    let packet = OppoAppPacket {
        command: 0x0106,
        sequence: 0xf0,
        payload: vec![],
    };
    let encoded = encode_oppo_frame(&packet.encode().unwrap()).unwrap();
    assert_eq!(
        encoded,
        [0xaa, 0x07, 0x00, 0x00, 0x06, 0x01, 0xf0, 0x00, 0x00]
    );
}

#[test]
fn decoder_accepts_partial_and_coalesced_frames() {
    let first = OppoAppPacket {
        command: 0x0106,
        sequence: 0xf0,
        payload: vec![],
    };
    let second = OppoAppPacket {
        command: 0x0105,
        sequence: 0x00,
        payload: vec![1, 2],
    };
    let mut input = encode_oppo_frame(&first.encode().unwrap()).unwrap();
    input.extend(encode_oppo_frame(&second.encode().unwrap()).unwrap());
    let split = 4;
    let mut decoder = OppoFrameDecoder::default();

    assert!(decoder.push(&input[..split]).unwrap().is_empty());
    let packets = decoder.push(&input[split..]).unwrap();
    assert_eq!(packets, [first.encode().unwrap(), second.encode().unwrap()]);
}

#[test]
fn application_packet_round_trips() {
    let packet = OppoAppPacket {
        command: 0x0404,
        sequence: 7,
        payload: vec![1, 1, 0x20],
    };
    assert_eq!(
        OppoAppPacket::decode(&packet.encode().unwrap()).unwrap(),
        packet
    );
}

#[test]
fn decoder_resynchronizes_after_noise() {
    let packet = OppoAppPacket {
        command: 0x0100,
        sequence: 0,
        payload: vec![],
    };
    let mut input = vec![0x12, 0x34, 0x56];
    input.extend(encode_oppo_frame(&packet.encode().unwrap()).unwrap());
    let decoded = OppoFrameDecoder::default().push(&input).unwrap();
    assert_eq!(decoded, [packet.encode().unwrap()]);
}

#[test]
fn decoder_reassembles_fragments() {
    let make_fragment = |control: u8, sequence: u8, data: &[u8]| {
        let body_len = data.len() + 3;
        let mut frame = vec![OPPO_FRAME_START, body_len as u8, control, 0, sequence];
        frame.extend_from_slice(data);
        frame
    };
    let mut decoder = OppoFrameDecoder::default();
    let first = make_fragment(1, 9, &[1, 2]);
    let last = make_fragment(3, 10, &[3, 4]);

    assert!(decoder.push(&first).unwrap().is_empty());
    assert_eq!(decoder.push(&last).unwrap(), [vec![1, 2, 3, 4]]);
}

#[test]
fn decoder_handles_two_byte_length_varint() {
    let packet = vec![0x5a; 130];
    let frame = encode_oppo_frame(&packet).unwrap();
    assert_eq!(frame[1], 0x84);
    assert_eq!(frame[2], 0x01);
    assert_eq!(OppoFrameDecoder::default().push(&frame).unwrap(), [packet]);
}

#[test]
fn parses_x4_battery_response_and_retains_missing_case_as_unknown() {
    let state = parse_oppo_battery_response(&[0, 2, 1, 0xc6, 2, 0x5a]).unwrap();
    assert_eq!(
        state,
        BatteryState {
            left: Some(BatteryPart {
                percentage: 70,
                charging: true,
            }),
            right: Some(BatteryPart {
                percentage: 90,
                charging: false,
            }),
            case: None,
        }
    );
}

#[test]
fn treats_zero_unplugged_case_battery_as_unreported_while_preserving_charging_zero() {
    let unreported = parse_oppo_battery_response(&[0, 3, 1, 0x50, 2, 0x50, 3, 0x00]).unwrap();
    assert_eq!(
        unreported,
        BatteryState {
            left: Some(BatteryPart {
                percentage: 80,
                charging: false,
            }),
            right: Some(BatteryPart {
                percentage: 80,
                charging: false,
            }),
            case: None,
        }
    );

    let unreported_event = parse_oppo_battery_event(&[1, 3, 1, 0x50, 2, 0x50, 3, 0x00]).unwrap();
    assert_eq!(unreported_event.case, None);

    let charging_zero = parse_oppo_battery_response(&[0, 3, 1, 0x50, 2, 0x50, 3, 0x80]).unwrap();
    assert_eq!(
        charging_zero.case,
        Some(BatteryPart {
            percentage: 0,
            charging: true,
        })
    );
}

#[test]
fn parses_noise_control_bitmap_indices() {
    assert_eq!(parse_oppo_noise_mode_response(&[0, 1, 0, 0x40]), Some(6));
    assert_eq!(parse_oppo_noise_mode_event(&[3, 1, 1, 0x20]), Some(5));
}

#[test]
fn creates_write_payloads_for_every_enabled_noise_index() {
    assert_eq!(encode_oppo_noise_mode(0), vec![1, 1, 1]);
    assert_eq!(encode_oppo_noise_mode(1), vec![1, 1, 2]);
    assert_eq!(encode_oppo_noise_mode(2), vec![1, 1, 4]);
    assert_eq!(encode_oppo_noise_mode(3), vec![1, 1, 0x08]);
    assert_eq!(encode_oppo_noise_mode(4), vec![1, 1, 0x10]);
    assert_eq!(encode_oppo_noise_mode(5), vec![1, 1, 0x20]);
    assert_eq!(encode_oppo_noise_mode(6), vec![1, 1, 0x40]);
    assert_eq!(encode_oppo_noise_mode(7), vec![1, 1, 0x80]);
    assert_eq!(encode_oppo_noise_mode(8), vec![1, 1, 0, 1]);
    assert_eq!(encode_oppo_noise_mode(11), vec![1, 1, 0, 8]);
    assert_eq!(encode_oppo_noise_mode(12), vec![1, 1, 0, 0x10]);
    assert_eq!(encode_oppo_noise_mode(13), vec![1, 1, 0, 0x20]);
    assert_eq!(encode_oppo_noise_mode(14), vec![1, 1, 0, 0x40]);
}

#[test]
fn parses_api_noise_mode_support() {
    assert_eq!(
        parse_oppo_supported_noise_modes_response(&[0, 3, 0, 0x90, 0x20]),
        Some(vec![4, 7, 13])
    );
    assert_eq!(
        parse_oppo_supported_noise_modes_response(&[0, 3, 0, 0, 0]),
        Some(vec![])
    );
    assert_eq!(
        parse_oppo_supported_noise_modes_response(&[0, 3, 0]),
        Some(vec![])
    );
    assert_eq!(
        parse_oppo_supported_noise_modes_response(&[0, 0, 0, 0, 0]),
        Some(vec![])
    );
    assert_eq!(parse_oppo_supported_noise_modes_response(&[0, 3]), None);
    assert_eq!(
        parse_oppo_supported_noise_modes_response(&[0, 1, 0, 0x40]),
        None
    );
}

#[test]
fn capability_bitmap_gates_noise_control_writes() {
    assert!(capability_bitmap_allows(&[0x00, 0x01], 0x0404));
    assert!(!capability_bitmap_allows(&[0x00], 0x0404));
    assert!(!capability_bitmap_allows(&[0x00, 0x00], 0x0404));
    assert!(capability_bitmap_allows(&[0, 0, 0, 0x20], 0x0112));
    assert!(!capability_bitmap_allows(&[0, 0, 0, 0x00], 0x0112));
    assert!(capability_bitmap_allows(
        &[0, 0, 0, 0, 0, 0, 0, 0x08],
        0x0429
    ));
    assert!(!capability_bitmap_allows(&[0, 0, 0, 0, 0, 0, 0, 0], 0x0429));
    assert!(!capability_bitmap_allows(&[0xff, 0xff, 0xff], 0x0403));
}

#[test]
fn parses_x4_firmware_csv_response() {
    assert_eq!(
        parse_oppo_firmware_response(b"\0\x031,2,128,2,2,128,3,2,100").unwrap(),
        FirmwareVersions {
            left: Some("128".to_string()),
            right: Some("128".to_string()),
            case: Some("100".to_string()),
        }
    );
}

#[test]
fn rejects_inconsistent_firmware_component_count() {
    assert_eq!(
        parse_oppo_firmware_response(b"\0\x021,2,128"),
        Err(OppoProtocolError::InvalidPacketLength)
    );
}

#[test]
fn parses_wear_state_response_and_event() {
    let expected = WearState {
        left: Some(WearPart {
            out_of_case: true,
            in_ear: true,
            raw_flags: 0x07,
        }),
        right: Some(WearPart {
            out_of_case: true,
            in_ear: true,
            raw_flags: 0x07,
        }),
        case_open: Some(true),
    };
    assert_eq!(
        parse_oppo_wear_response(&[0, 3, 1, 7, 2, 7, 3, 4]).unwrap(),
        expected
    );
    assert_eq!(
        parse_oppo_wear_event(&[2, 3, 1, 7, 2, 7, 3, 4]).unwrap(),
        expected
    );
    assert!(expected.any_earbud_in_ear());
    assert!(!WearState::default().any_earbud_in_ear());
}

#[test]
fn parses_headset_paired_devices_and_retains_addresses_for_session_commands() {
    let mut response = vec![0, 2];
    response.extend_from_slice(&[1, 2, 3, 4, 5, 6, 6, 1, 0x01, 3]);
    response.extend_from_slice(b"PC1");
    response.extend_from_slice(&[6, 5, 4, 3, 2, 1, 9, 0, 0x0A, 6]);
    response.extend_from_slice(b"Phone1");

    assert_eq!(
        parse_oppo_paired_devices_response(&response).unwrap(),
        vec![
            PairedDevice {
                name: "PC1".to_string(),
                connected: true,
                is_current_host: true,
                device_type: 0,
                address: PairedDeviceAddress::from_display_bytes([6, 5, 4, 3, 2, 1]),
            },
            PairedDevice {
                name: "Phone1".to_string(),
                connected: false,
                is_current_host: false,
                device_type: 1,
                address: PairedDeviceAddress::from_display_bytes([1, 2, 3, 4, 5, 6]),
            },
        ]
    );
    let phone = &parse_oppo_paired_devices_response(&response).unwrap()[1];
    assert_eq!(
        PairedDeviceAction::Connect.payload(phone.address),
        [1, 1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        PairedDeviceAction::Disconnect.payload(phone.address),
        [2, 1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        PairedDeviceAction::Remove.payload(phone.address),
        [3, 1, 2, 3, 4, 5, 6]
    );
    assert_eq!(format!("{:?}", phone.address), "[redacted]");
    assert_eq!(phone.address.format_mac(), "01:02:03:04:05:06");

    let mut event = vec![6, 1];
    event.extend_from_slice(&[1, 2, 3, 4, 5, 6, 6, 1, 0x01, 3]);
    event.extend_from_slice(b"PC1");
    assert_eq!(
        parse_oppo_paired_devices_event(&event).unwrap(),
        vec![PairedDevice {
            name: "PC1".to_string(),
            connected: true,
            is_current_host: true,
            device_type: 0,
            address: PairedDeviceAddress::from_display_bytes([6, 5, 4, 3, 2, 1]),
        }]
    );
}

#[test]
fn parses_multi_device_feature_setting() {
    assert!(parse_oppo_feature_flags_response(&[0, 2, 17, 1, 4, 0], 17).unwrap());
    assert!(!parse_oppo_feature_flags_response(&[0, 1, 17, 0], 17).unwrap());
    assert!(!parse_oppo_feature_flags_response(&[0, 1, 4, 1], 17).unwrap());
    assert_eq!(
        parse_oppo_feature_flags_response(&[0, 1, 17], 17),
        Err(OppoProtocolError::InvalidPacketLength)
    );
}

#[test]
fn rejects_malformed_headset_paired_device_entries() {
    assert_eq!(
        parse_oppo_paired_devices_response(&[0, 1, 1, 2, 3, 4, 5, 6, 5, 1, 0, 3]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
    assert_eq!(
        parse_oppo_paired_devices_response(&[0, 1, 1, 2, 3, 4, 5, 6, 6, 1, 0, 1, 0xff]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
}

#[test]
fn parses_equalizer_and_spatial_audio_responses() {
    assert_eq!(parse_oppo_equalizer_response(&[0, 7]).unwrap(), 7);
    assert_eq!(
        parse_oppo_equalizer_response(&[1, 7]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
    assert_eq!(
        parse_oppo_equalizer_response(&[0]),
        Err(OppoProtocolError::InvalidPacketLength)
    );

    assert_eq!(parse_oppo_equalizer_event(&[2]).unwrap(), 2);
    assert_eq!(
        parse_oppo_equalizer_event(&[]),
        Err(OppoProtocolError::InvalidPacketLength)
    );

    assert_eq!(parse_oppo_spatial_audio_response(&[0, 0]).unwrap(), 0);
    assert_eq!(parse_oppo_spatial_audio_response(&[0, 1]).unwrap(), 1);
    assert_eq!(parse_oppo_spatial_audio_response(&[0, 2]).unwrap(), 2);
    assert_eq!(
        parse_oppo_spatial_audio_response(&[0, 3]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
    assert_eq!(
        parse_oppo_spatial_audio_response(&[1, 0]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
}

#[test]
fn parses_color_response() {
    assert_eq!(parse_oppo_color_response(&[0, 2]).unwrap(), 2);
    assert_eq!(parse_oppo_color_response(&[0, 1, 0xFF]).unwrap(), 1);
    assert!(parse_oppo_color_response(&[1, 2]).is_err());
    assert!(parse_oppo_color_response(&[0]).is_err());
    assert!(parse_oppo_color_response(&[]).is_err());
}

#[test]
fn parses_bass_engine_response_and_event() {
    let level = parse_oppo_bass_engine_response(&[0, 0xFB, 0x05, 0x00]).unwrap();
    assert_eq!(level.min, -5);
    assert_eq!(level.max, 5);
    assert_eq!(level.current, 0);

    let clamped_neg = parse_oppo_bass_engine_response(&[0, 0xFB, 0x05, 0xF5]).unwrap();
    assert_eq!(clamped_neg.current, -5);

    let clamped_pos = parse_oppo_bass_engine_response(&[0, 0xFB, 0x05, 0x0A]).unwrap();
    assert_eq!(clamped_pos.current, 5);

    assert_eq!(
        parse_oppo_bass_engine_response(&[1, 0xFB, 0x05, 0x00]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
    assert_eq!(
        parse_oppo_bass_engine_response(&[0, 0xFB, 0x05]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
    assert_eq!(
        parse_oppo_bass_engine_response(&[0, 0x05, 0xFB, 0x00]),
        Err(OppoProtocolError::InvalidPacketLength)
    );

    let event = parse_oppo_bass_engine_event(&[0xFB, 0x05, 0x02]).unwrap();
    assert_eq!(event.min, -5);
    assert_eq!(event.max, 5);
    assert_eq!(event.current, 2);

    let event_with_status = parse_oppo_bass_engine_event(&[0, 0xFB, 0x05, 0x03]).unwrap();
    assert_eq!(event_with_status.current, 3);

    assert_eq!(
        parse_oppo_bass_engine_event(&[0xFB, 0x05]),
        Err(OppoProtocolError::InvalidPacketLength)
    );
}

#[test]
fn custom_equalizer_encode_matches_reference() {
    let bands = [
        (31, 0),
        (62, 0),
        (125, 0),
        (250, 0),
        (500, 0),
        (1000, 0),
        (2000, 0),
        (4000, 0),
        (8000, 0),
        (16000, 0),
    ];
    let params = CustomEqualizerParams {
        id: 0,
        name: "自定义1",
        min_gain: -6,
        max_gain: 6,
        bands: &bands,
    };
    let encoded = encode_oppo_custom_equalizer_command(CustomEqualizerAction::Create, &params);
    let expected = [
        0x01, 0xFA, 0x06, 0x00, 0x0A, 0xE8, 0x87, 0xAA, 0xE5, 0xAE, 0x9A, 0xE4, 0xB9, 0x89, 0x31,
        0x0A, 0x1F, 0x00, 0x00, 0x3E, 0x00, 0x00, 0x7D, 0x00, 0x00, 0xFA, 0x00, 0x00, 0xF4, 0x01,
        0x00, 0xE8, 0x03, 0x00, 0xD0, 0x07, 0x00, 0xA0, 0x0F, 0x00, 0x40, 0x1F, 0x00, 0x80, 0x3E,
        0x00,
    ];
    assert_eq!(encoded, expected);
}

#[test]
fn custom_equalizer_parse_matches_reference_capture() {
    let payload = [
        0x00, 0x01, 0x00, 0xFA, 0x06, 0x04, 0x0A, 0xE8, 0x87, 0xAA, 0xE5, 0xAE, 0x9A, 0xE4, 0xB9,
        0x89, 0x31, 0x0A, 0x1F, 0x00, 0x00, 0x3E, 0x00, 0x00, 0x7D, 0x00, 0x00, 0xFA, 0x00, 0x00,
        0xF4, 0x01, 0x00, 0xE8, 0x03, 0x00, 0xD0, 0x07, 0x00, 0xA0, 0x0F, 0x00, 0x40, 0x1F, 0x00,
        0x80, 0x3E, 0x00,
    ];
    let list = parse_oppo_custom_equalizer_list(&payload, true).unwrap();
    assert_eq!(list.len(), 1);
    let eq = &list[0];
    assert_eq!(eq.id, 4);
    assert_eq!(eq.name, "自定义1");
    assert!(!eq.is_selected);
    assert_eq!(eq.min_gain, -6);
    assert_eq!(eq.max_gain, 6);
    assert_eq!(eq.bands.len(), 10);
    assert_eq!(eq.bands[0], (31, 0));
    assert_eq!(eq.bands[9], (16000, 0));

    // Also verify push notification (without status byte)
    let push_list = parse_oppo_custom_equalizer_list(&payload[1..], false).unwrap();
    assert_eq!(push_list, list);
}

#[test]
fn touch_controls_parse_and_encode() {
    use crate::protocol::brands::oppo::oppo_controls::{
        encode_oppo_set_key_function, parse_oppo_touch_controls_response,
    };
    use crate::protocol::core::types::{EarSide, GestureAction, KeyFunction};

    // 0x8108 payload: [st=0, count=2, {side=1, button=1, action=2, function=6}, {side=2, button=6, action=1, function=29}]
    let payload = [0x00, 0x02, 0x01, 0x01, 0x02, 0x06, 0x02, 0x06, 0x01, 0x1D];
    let controls = parse_oppo_touch_controls_response(&payload).unwrap();
    assert_eq!(controls.len(), 2);
    assert_eq!(
        controls[0],
        KeyFunction {
            side: EarSide::Left,
            button: 1,
            action: GestureAction::DoubleSqueeze,
            function: 6,
        }
    );
    assert_eq!(
        controls[1],
        KeyFunction {
            side: EarSide::Right,
            button: 6,
            action: GestureAction::SingleSqueeze,
            function: 29,
        }
    );
    assert!(controls[1].is_call_control());

    let key_function = KeyFunction {
        side: EarSide::Right,
        button: 1,
        action: GestureAction::TripleSqueeze,
        function: 5,
    };
    let encoded = encode_oppo_set_key_function(&key_function);
    assert_eq!(encoded, [0x01, 0x02, 0x01, 0x03, 0x05]);
}

#[test]
fn fit_test_event_parse_and_encode() {
    use crate::protocol::brands::oppo::oppo_controls::{
        encode_oppo_fit_test, parse_oppo_fit_test_event,
    };
    use crate::protocol::core::types::FitQuality;

    // [04, dev=1, st=1 (good), dev=2, st=0 (average)]
    let payload = [0x04, 0x01, 0x01, 0x02, 0x00];
    let result = parse_oppo_fit_test_event(&payload).unwrap();
    assert_eq!(result.left, FitQuality::Good);
    assert_eq!(result.right, FitQuality::Average);

    // [04, dev=1, st=6 (poor), dev=2, st=9 (failed)]
    let payload_poor = [0x04, 0x01, 0x06, 0x02, 0x09];
    let result_poor = parse_oppo_fit_test_event(&payload_poor).unwrap();
    assert_eq!(result_poor.left, FitQuality::Poor);
    assert_eq!(result_poor.right, FitQuality::Failed);

    assert_eq!(encode_oppo_fit_test(true), [0x01]);
    assert_eq!(encode_oppo_fit_test(false), [0x00]);
}

#[test]
fn prompt_volume_and_head_motion_parsers_and_encoders() {
    use crate::protocol::brands::oppo::oppo_controls::{
        encode_oppo_find_earbuds, encode_oppo_set_head_motion_type, encode_oppo_set_prompt_volume,
        parse_oppo_head_motion_type_response, parse_oppo_prompt_volume_response,
    };

    assert_eq!(parse_oppo_prompt_volume_response(&[0x00, 0x07]).unwrap(), 7);
    assert!(parse_oppo_prompt_volume_response(&[0x00, 0x00]).is_err());
    assert!(parse_oppo_prompt_volume_response(&[0x00, 0x0B]).is_err());
    assert_eq!(encode_oppo_set_prompt_volume(8), [0x08]);

    assert_eq!(
        parse_oppo_head_motion_type_response(&[0x00, 0x01]).unwrap(),
        1
    );
    assert!(parse_oppo_head_motion_type_response(&[0x00, 0x02]).is_err());
    assert_eq!(encode_oppo_set_head_motion_type(0), [0x00]);

    assert_eq!(encode_oppo_find_earbuds(true), [0x01]);
    assert_eq!(encode_oppo_find_earbuds(false), [0x00]);
}

#[test]
fn noise_cycle_and_priority_device_parsers_and_encoders() {
    use crate::protocol::brands::oppo::oppo_controls::{
        encode_oppo_set_noise_cycle, parse_oppo_noise_cycle_response,
    };
    use crate::protocol::brands::oppo::oppo_device::{
        encode_oppo_set_priority_device, parse_oppo_priority_device_response,
    };
    use crate::protocol::core::types::EarSide;

    // 0x810C [st=0, 02, type=1, mask=0x00000007 (bits 0,1,2)]
    let (cycle_type, mask) =
        parse_oppo_noise_cycle_response(&[0x00, 0x02, 0x01, 0x07, 0x00]).unwrap();
    assert_eq!(cycle_type, 1);
    assert_eq!(mask, 7);

    let enc_shared = encode_oppo_set_noise_cycle(&NoiseCycleSetting {
        side: EarSide::Both,
        mask: 7,
        is_shared: true,
    });
    assert_eq!(enc_shared, [0x02, 0x01, 0x07]);

    let enc_right = encode_oppo_set_noise_cycle(&NoiseCycleSetting {
        side: EarSide::Right,
        mask: 0x0103,
        is_shared: false,
    });
    assert_eq!(enc_right, [0x02, 0x04, 0x03, 0x01]);

    // Priority device: 0x8132 [st=0, 02, flag=1, mac6 reverse ordered]
    let addr_payload = [0x00, 0x02, 0x01, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];
    let addr = parse_oppo_priority_device_response(&addr_payload)
        .unwrap()
        .unwrap();
    assert_eq!(addr.command_bytes(), [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

    let auto_payload = [0x00, 0x02, 0x00];
    let auto_addr = parse_oppo_priority_device_response(&auto_payload).unwrap();
    assert!(auto_addr.is_none());

    let enc_addr = encode_oppo_set_priority_device(Some(addr));
    assert_eq!(enc_addr, [0x04, 0x01, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    let enc_auto = encode_oppo_set_priority_device(None);
    assert_eq!(enc_auto, [0x04, 0x00]);
}
