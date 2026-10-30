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
    fn test_parse_real_hardware_capture() {
        // Real FW capture from EarFun Air Pro 4
        let raw_fw = [
            0x00, 0x54, 0x57, 0x36, 0x30, 0x30, 0x2D, 0x33, 0x30, 0x39, 0x31, 0x5F, 0x32, 0x30,
            0x32, 0x35, 0x30, 0x39, 0x31, 0x39, 0x5F, 0x76, 0x30, 0x2E, 0x34, 0x2E, 0x31,
        ];
        let fw = parse_earfun_firmware_version(&raw_fw).unwrap();
        assert_eq!(fw.left.unwrap(), "0.4.1");

        // Real battery capture
        let bat = parse_earfun_battery_component(&[0x00, 0x5A]).unwrap();
        assert_eq!(bat.percentage, 90);

        // Real ANC submode capture
        let anc_sub = parse_earfun_anc_sub_mode(&[0x00, 0x04]).unwrap();
        assert_eq!(anc_sub, 4); // AI Environment Adaptive
    }
}
