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

#[cfg(test)]
mod tests {
    use crate::protocol::brands::earfun::*;

    #[test]
    fn test_earfun_packet_encode_decode() {
        let pkt = EarFunPacket::new(EarFunCommands::FIRMWARE_VERSION, vec![]);
        let encoded = pkt.encode();
        assert_eq!(
            encoded,
            vec![0xFF, 0x04, 0x00, 0x00, 0x00, 0x0A, 0x03, 0x09]
        );

        let (decoded, consumed) = EarFunPacket::decode(&encoded).unwrap();
        assert_eq!(consumed, 8);
        assert_eq!(decoded.command, EarFunCommands::FIRMWARE_VERSION);
        assert_eq!(decoded.vendor_id, EarFunCommands::VENDOR_GAIA_LEGACY);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn test_earfun_frame_decoder_coalesced_and_noise() {
        let mut decoder = EarFunFrameDecoder::default();
        let pkt1 = EarFunPacket::new(EarFunCommands::BATTERY_LEFT, vec![0x00, 0x5A]);
        let pkt2 = EarFunPacket::new(EarFunCommands::BATTERY_RIGHT, vec![0x00, 0x64]);

        let mut stream = vec![0xAA, 0xBB]; // garbage prefix
        stream.extend_from_slice(&pkt1.encode());
        stream.extend_from_slice(&[0x12, 0x34]); // garbage in between
        stream.extend_from_slice(&pkt2.encode());

        let packets = decoder.push(&stream);
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].command, EarFunCommands::BATTERY_LEFT);
        assert_eq!(packets[0].payload, vec![0x00, 0x5A]);
        assert_eq!(packets[1].command, EarFunCommands::BATTERY_RIGHT);
        assert_eq!(packets[1].payload, vec![0x00, 0x64]);
    }

    #[test]
    fn ef_packet_encoding_and_incremental_decoding() {
        let packet = EarFunEfPacket::new(0x1010, vec![0x31, 0x32]).unwrap();
        let frame = packet.encode().unwrap();
        assert_eq!(frame, [0xEF, 0x10, 0x10, 2, 0x31, 0x32, 0x65, 0xFE]);

        let mut decoder = EarFunEfFrameDecoder::default();
        assert!(decoder.push(&[0xAA, 0xEF, 0x10]).is_empty());
        assert!(decoder.push(&frame[2..6]).is_empty());
        let packets = decoder.push(&frame[6..]);
        assert_eq!(packets, vec![packet]);
    }

    #[test]
    fn ef_decoder_handles_coalesced_frames_and_recovers_from_bad_checksum() {
        let first = EarFunEfPacket::new(0x1014, vec![1, 2])
            .unwrap()
            .encode()
            .unwrap();
        let second = EarFunEfPacket::new(0x2080, vec![3])
            .unwrap()
            .encode()
            .unwrap();
        let mut bad = first.clone();
        let checksum_index = bad.len() - 2;
        bad[checksum_index] ^= 0x01;

        let mut decoder = EarFunEfFrameDecoder::default();
        let mut stream = bad;
        stream.extend_from_slice(&second);
        let packets = decoder.push(&stream);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].command, 0x2080);
        assert_eq!(packets[0].payload, [3]);
    }

    #[test]
    fn test_parse_real_hardware_capture() {
        // Real FW capture from EarFun Air Pro 4
        let raw_fw = [
            0x00, 0x54, 0x57, 0x36, 0x30, 0x30, 0x2D, 0x33, 0x30, 0x39, 0x31, 0x5F, 0x32, 0x30,
            0x32, 0x35, 0x30, 0x39, 0x31, 0x39, 0x5F, 0x76, 0x30, 0x2E, 0x34, 0x2E, 0x31,
        ];
        let fw = parse_earfun_firmware_version(&raw_fw).unwrap();
        assert_eq!(fw.left.unwrap(), "0.4.1");

        // Real battery capture
        let bat = parse_earfun_battery_component(&[0x00, 0x5A])
            .unwrap()
            .unwrap();
        assert_eq!(bat.percentage, 90);
        assert_eq!(bat.charging, None);

        // Disconnected or unavailable battery (e.g. charging case offline/lid closed returns 0xFF / 255)
        let unavailable_bat = parse_earfun_battery_component(&[0x00, 0xFF]).unwrap();
        assert!(unavailable_bat.is_none());

        let overflow_bat = parse_earfun_battery_component(&[0x00, 101]).unwrap();
        assert!(overflow_bat.is_none());

        let zero_bat = parse_earfun_battery_component(&[0x00, 0x00])
            .unwrap()
            .unwrap();
        assert_eq!(zero_bat.percentage, 0);

        assert!(parse_earfun_battery_component(&[0x01, 0x5A]).is_err());
        assert!(parse_earfun_battery_component(&[0x00]).is_err());

        // Real ANC submode capture
        let anc_sub = parse_earfun_anc_sub_mode(&[0x00, 0x04]).unwrap();
        assert_eq!(anc_sub, 4); // AI Environment Adaptive

        // Real paired device capture from EarFun Air Pro 4 (slot 9: NUC12SNKi72)
        let slot9_payload = [
            0x00, 0x09, 0x4E, 0x55, 0x43, 0x31, 0x32, 0x53, 0x4E, 0x4B, 0x69, 0x37, 0x32, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x61, 0x33, 0x3D, 0x01,
        ];
        let dev = parse_earfun_paired_device_entry(&slot9_payload, Some([0x61, 0x33, 0x3D]))
            .unwrap()
            .expect("should parse valid device");
        assert_eq!(dev.name, "NUC12SNKi72");
        assert!(dev.connected);
        assert!(dev.is_current_host);
        assert_eq!(dev.device_type, 0);
        assert_eq!(dev.address.command_bytes(), [0, 0, 9, 0x61, 0x33, 0x3D]);

        // Slot 0 (arch-work, disconnected)
        let slot0_payload = [
            0x00, 0x00, 0x61, 0x72, 0x63, 0x68, 0x2D, 0x77, 0x6F, 0x72, 0x6B, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xC6, 0x2C, 0x00,
        ];
        let dev0 = parse_earfun_paired_device_entry(&slot0_payload, Some([0x61, 0x33, 0x3D]))
            .unwrap()
            .expect("should parse valid device");
        assert_eq!(dev0.name, "arch-work");
        assert!(!dev0.connected);
        assert!(!dev0.is_current_host);
        assert_eq!(dev0.device_type, 0);

        // Empty slot (slot 6: name is all zeros)
        let slot6_payload = [
            0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1D, 0xCC, 0x09, 0x00,
        ];
        let dev6 =
            parse_earfun_paired_device_entry(&slot6_payload, Some([0x61, 0x33, 0x3D])).unwrap();
        assert!(dev6.is_none());

        // Real codec query capture (0x13 = aptX Lossless)
        let codec = parse_earfun_audio_codec(&[0x00, 0x13]).unwrap();
        assert_eq!(codec, 19);
        assert_eq!(
            EarFunAudioCodec::from_u8(codec),
            Some(EarFunAudioCodec::AptXLossless)
        );

        // Codec LDAC (0x08)
        let ldac_codec = parse_earfun_audio_codec(&[0x00, 0x08]).unwrap();
        assert_eq!(ldac_codec, 8);
        assert_eq!(
            EarFunAudioCodec::from_u8(ldac_codec),
            Some(EarFunAudioCodec::Ldac)
        );

        // Real audio mode query (0x03)
        let audio_mode = parse_earfun_audio_mode(&[0x00, 0x03]).unwrap();
        assert_eq!(audio_mode, 3);

        // Real codec query capture (0x13 = aptX Lossless)
        let codec = parse_earfun_audio_codec(&[0x00, 0x13]).unwrap();
        assert_eq!(codec, 19);
        assert_eq!(
            EarFunAudioCodec::from_u8(codec),
            Some(EarFunAudioCodec::AptXLossless)
        );

        // Codec LDAC (0x08)
        let ldac_codec = parse_earfun_audio_codec(&[0x00, 0x08]).unwrap();
        assert_eq!(ldac_codec, 8);
        assert_eq!(
            EarFunAudioCodec::from_u8(ldac_codec),
            Some(EarFunAudioCodec::Ldac)
        );

        // Real audio mode query (0x03)
        let audio_mode = parse_earfun_audio_mode(&[0x00, 0x03]).unwrap();
        assert_eq!(audio_mode, 3);
    }

    #[test]
    fn test_earfun_audio_codec_and_reboot_encode() {
        let set_codec_pkt = EarFunPacket::new(
            EarFunCommands::SET_AUDIO_CODEC,
            vec![EarFunAudioCodec::Ldac.as_u8()],
        );
        assert_eq!(
            set_codec_pkt.encode(),
            vec![0xFF, 0x04, 0x00, 0x01, 0x00, 0x0A, 0x03, 0x2E, 0x08]
        );

        let reboot_pkt = EarFunPacket::new(EarFunCommands::REBOOT, vec![]);
        assert_eq!(
            reboot_pkt.encode(),
            vec![0xFF, 0x04, 0x00, 0x00, 0x00, 0x0A, 0x03, 0x08]
        );
    }

    #[test]
    fn test_earfun_touch_gestures_parse() {
        use crate::protocol::core::types::{EarSide, GestureAction};

        // 17-byte payload matching Gadgetbridge test vector
        let payload = [
            0x01, // status byte
            0x00, 0x01, // L Single -> 1 (Play/Pause)
            0x00, 0x02, // R Single -> 2 (Vol Up)
            0x00, 0x03, // L Double -> 3 (Vol Down)
            0x00, 0x04, // R Double -> 4 (Prev)
            0x00, 0x05, // L Triple -> 5 (Next)
            0x00, 0x06, // R Triple -> 6 (Voice Assistant)
            0x00, 0x07, // L Long   -> 7 (Game Mode)
            0x00, 0x08, // R Long   -> 8 (Noise Control)
        ];

        let gestures = parse_earfun_touch_gestures_response(&payload).unwrap();
        assert_eq!(gestures.len(), 8);

        assert_eq!(gestures[0].side, EarSide::Left);
        assert_eq!(gestures[0].action, GestureAction::SingleSqueeze);
        assert_eq!(gestures[0].function, 1);

        assert_eq!(gestures[1].side, EarSide::Right);
        assert_eq!(gestures[1].action, GestureAction::SingleSqueeze);
        assert_eq!(gestures[1].function, 2);

        assert_eq!(gestures[2].side, EarSide::Left);
        assert_eq!(gestures[2].action, GestureAction::DoubleSqueeze);
        assert_eq!(gestures[2].function, 3);

        assert_eq!(gestures[3].side, EarSide::Right);
        assert_eq!(gestures[3].action, GestureAction::DoubleSqueeze);
        assert_eq!(gestures[3].function, 4);

        assert_eq!(gestures[4].side, EarSide::Left);
        assert_eq!(gestures[4].action, GestureAction::TripleSqueeze);
        assert_eq!(gestures[4].function, 5);

        assert_eq!(gestures[5].side, EarSide::Right);
        assert_eq!(gestures[5].action, GestureAction::TripleSqueeze);
        assert_eq!(gestures[5].function, 6);

        assert_eq!(gestures[6].side, EarSide::Left);
        assert_eq!(gestures[6].action, GestureAction::SqueezeAndHold);
        assert_eq!(gestures[6].function, 7);

        assert_eq!(gestures[7].side, EarSide::Right);
        assert_eq!(gestures[7].action, GestureAction::SqueezeAndHold);
        assert_eq!(gestures[7].function, 8);

        // Status 0x00 is also accepted
        let mut payload_status_0 = payload;
        payload_status_0[0] = 0x00;
        assert!(parse_earfun_touch_gestures_response(&payload_status_0).is_ok());

        // Error cases
        assert!(parse_earfun_touch_gestures_response(&payload[..16]).is_err());
        let mut invalid_status = payload;
        invalid_status[0] = 0x02;
        assert!(parse_earfun_touch_gestures_response(&invalid_status).is_err());
    }

    #[test]
    fn test_earfun_touch_gesture_encode() {
        use crate::protocol::core::types::{EarSide, GestureAction};

        // Left ear Single Tap -> Play/Pause (1)
        let l_single =
            encode_earfun_set_touch_action(EarSide::Left, EarFunInteractionType::SingleTap, 1);
        assert_eq!(l_single, vec![0x01, 0x01, 0x00, 0x00]);

        // Right ear Long Press (0x11) -> Noise Control (8)
        let r_long =
            encode_earfun_set_touch_action(EarSide::Right, EarFunInteractionType::LongPress, 8);
        assert_eq!(r_long, vec![0x00, 0x00, 0x11, 0x08]);

        // Generic GestureAction conversion helper
        let encoded_l =
            encode_earfun_set_gesture(EarSide::Left, GestureAction::DoubleSqueeze, 5).unwrap();
        assert_eq!(encoded_l, vec![0x02, 0x05, 0x00, 0x00]);

        let encoded_r =
            encode_earfun_set_gesture(EarSide::Right, GestureAction::SqueezeAndHold, 8).unwrap();
        assert_eq!(encoded_r, vec![0x00, 0x00, 0x11, 0x08]);

        // Whole packet encoding
        let pkt = EarFunPacket::new(EarFunCommands::SET_TOUCH_ACTION, encoded_r);
        assert_eq!(
            pkt.encode(),
            vec![
                0xFF, 0x04, 0x00, 0x04, 0x00, 0x0A, 0x03, 0x0A, 0x00, 0x00, 0x11, 0x08
            ]
        );
    }

    #[test]
    fn test_earfun_touch_lock_parse_and_encode() {
        // Mode 0: Unlocked (touch enabled)
        let unlocked = parse_earfun_touch_lock_response(&[0x00, 0x00]).unwrap();
        assert!(!unlocked);

        // Mode 1: Locked (touch disabled)
        let locked = parse_earfun_touch_lock_response(&[0x00, 0x01]).unwrap();
        assert!(locked);

        // Errors
        assert!(parse_earfun_touch_lock_response(&[0x01, 0x01]).is_err());
        assert!(parse_earfun_touch_lock_response(&[0x00]).is_err());

        // Encoding
        assert_eq!(encode_earfun_set_touch_lock(true), vec![0x01]);
        assert_eq!(encode_earfun_set_touch_lock(false), vec![0x00]);

        let pkt_lock = EarFunPacket::new(
            EarFunCommands::SET_TOUCH_MODE,
            encode_earfun_set_touch_lock(true),
        );
        assert_eq!(
            pkt_lock.encode(),
            vec![0xFF, 0x04, 0x00, 0x01, 0x00, 0x0A, 0x03, 0x34, 0x01]
        );
    }
}
