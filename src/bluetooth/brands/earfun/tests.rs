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
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
        sync::mpsc,
        thread,
        time::Duration,
    };

    use crate::bluetooth::brands::earfun::earfun_session::*;
    use crate::protocol::brands::earfun::{EarFunCommands, EarFunFrameDecoder, EarFunPacket};

    fn read_mock_request(stream: &mut UnixStream) -> EarFunPacket {
        thread_local! {
            static STATE: std::cell::RefCell<(EarFunFrameDecoder, Vec<EarFunPacket>)> =
                std::cell::RefCell::new((EarFunFrameDecoder::default(), Vec::new()));
        }

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if !state.1.is_empty() {
                return state.1.remove(0);
            }
            let mut buffer = [0u8; 512];
            loop {
                let bytes_read = stream.read(&mut buffer).expect("mock read error");
                assert!(bytes_read > 0, "mock stream closed unexpectedly");
                let mut packets = state.0.push(&buffer[..bytes_read]);
                if !packets.is_empty() {
                    let first = packets.remove(0);
                    state.1.extend(packets);
                    return first;
                }
            }
        })
    }

    fn send_mock_response(stream: &mut UnixStream, req_cmd: u16, payload: Vec<u8>) {
        let resp_cmd = req_cmd | 0x8000;
        let packet = EarFunPacket {
            version: 0x03,
            flags: 0x00,
            vendor_id: 0x000A,
            command: resp_cmd,
            payload,
        };
        stream
            .write_all(&packet.encode())
            .expect("mock write error");
    }

    #[test]
    fn test_earfun_session_initialization() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Firmware version 0x0309
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::FIRMWARE_VERSION);
            let mut fw_payload = vec![0x00];
            fw_payload.extend_from_slice(b"TW600-3091_20250919_v0.4.1");
            send_mock_response(&mut device, req.command, fw_payload);

            // 2. Battery left 0x0306
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_LEFT);
            send_mock_response(&mut device, req.command, vec![0x00, 90]);

            // 3. Battery right 0x0307
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_RIGHT);
            send_mock_response(&mut device, req.command, vec![0x00, 100]);

            // 4. Battery case 0x0317
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_CASE);
            send_mock_response(&mut device, req.command, vec![0x00, 80]);

            // 5. Ambient sound mode 0x0315
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AMBIENT_SOUND);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // ANC

            // 6. ANC submode 0x033B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_ANC_SUB_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 4]); // AI Environment Adaptive

            // 7. Game mode 0x0313
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_GAME_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // Enabled

            // 8. In-ear detection 0x034A
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_IN_EAR_DETECTION);
            send_mock_response(&mut device, req.command, vec![0x00, 0]); // Enabled (0)

            // 9. Dual device 0x0326
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECT_TWO_DEVICES);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // Enabled
        });

        let (snapshot, _) = initialize_earfun_session(&mut client, "EarFun Air Pro 4", None, None)
            .expect("session initialization should succeed");

        assert_eq!(snapshot.model, "EarFun Air Pro 4");
        assert_eq!(snapshot.firmware.left.as_deref(), Some("0.4.1"));
        assert_eq!(snapshot.battery.left.unwrap().percentage, 90);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 100);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 80);
        assert_eq!(snapshot.noise_mode, Some(3)); // AI Environment Adaptive
        assert_eq!(snapshot.game_mode_enabled, Some(true));
        assert_eq!(snapshot.in_ear_detection_enabled, Some(true));
        assert_eq!(snapshot.dual_device_enabled, Some(true));

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_noise_mode_and_readback() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // Set noise mode 4 (Strong ANC: master 1, submode 2)
            // 1. SET_AMBIENT_SOUND 0x0314 with [1]
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_AMBIENT_SOUND);
            assert_eq!(req.payload, vec![1]);

            // 2. SET_ANC_SUB_MODE 0x033A with [2]
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_ANC_SUB_MODE);
            assert_eq!(req.payload, vec![2]);
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let mut last_anc = 2;
        let mut last_transp = 0;
        let actual = set_noise_mode(
            &mut client,
            &mut decoder,
            EARFUN_AIR_PRO_4_PRODUCT_ID,
            4,
            &mut last_anc,
            &mut last_transp,
            &tx,
        )
        .expect("set noise mode should succeed");

        assert_eq!(actual, 4); // Strong ANC mapped index
        assert_eq!(last_anc, 2);
        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_game_mode() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_GAME_MODE);
            assert_eq!(req.payload, vec![1]);
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let actual = set_game_mode(&mut client, &mut decoder, true, &tx)
            .expect("set game mode should succeed");

        assert!(actual);
        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_in_ear_detection() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_IN_EAR_DETECTION);
            assert_eq!(req.payload, vec![1]); // Disabled
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let actual = set_in_ear_detection(&mut client, &mut decoder, false, &tx)
            .expect("set in ear detection should succeed");

        assert!(!actual);
        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_air_s_session_initialization() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Firmware version 0x0309
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::FIRMWARE_VERSION);
            let mut fw_payload = vec![0x00];
            fw_payload.extend_from_slice(b"TW201_v0.1.2");
            send_mock_response(&mut device, req.command, fw_payload);

            // 2. Battery left 0x0306
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_LEFT);
            send_mock_response(&mut device, req.command, vec![0x00, 80]);

            // 3. Battery right 0x0307
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_RIGHT);
            send_mock_response(&mut device, req.command, vec![0x00, 85]);

            // Air S does not query BATTERY_CASE

            // 4. Ambient sound mode 0x0315
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AMBIENT_SOUND);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // ANC

            // Air S does not query ANC sub-mode

            // 5. Game mode 0x0313
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_GAME_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // Enabled

            // Air S does not query in-ear detection

            // 6. Dual device 0x0326
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECT_TWO_DEVICES);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // Enabled
        });

        // Test that custom device name does NOT affect identification:
        // product ID is derived from firmware code TW201
        let (snapshot, _) = initialize_earfun_session(&mut client, "Custom Air S Name", None, None)
            .expect("Air S session initialization should succeed");

        assert_eq!(snapshot.model, "EarFun Air S");
        assert_eq!(snapshot.product_id, EARFUN_AIR_S_PRODUCT_ID);
        assert_eq!(snapshot.firmware.left.as_deref(), Some("0.1.2"));
        assert_eq!(snapshot.battery.left.unwrap().percentage, 80);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 85);
        assert!(snapshot.battery.case.is_none());
        assert_eq!(snapshot.noise_mode, Some(1)); // Standard ANC
        assert_eq!(snapshot.noise_modes.len(), 3);
        assert!(!snapshot.supports_wear_detection);
        assert!(snapshot.game_mode_enabled.unwrap());
        assert!(snapshot.dual_device_enabled.unwrap());

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_free_pro_3_session_initialization() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Firmware version 0x0309
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::FIRMWARE_VERSION);
            let mut fw_payload = vec![0x00];
            fw_payload.extend_from_slice(b"TW400_v0.2.0");
            send_mock_response(&mut device, req.command, fw_payload);

            // 2. Battery left 0x0306
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_LEFT);
            send_mock_response(&mut device, req.command, vec![0x00, 95]);

            // 3. Battery right 0x0307
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_RIGHT);
            send_mock_response(&mut device, req.command, vec![0x00, 95]);

            // 4. Battery case 0x0317
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_CASE);
            send_mock_response(&mut device, req.command, vec![0x00, 90]);

            // 5. Ambient sound mode 0x0315
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AMBIENT_SOUND);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // ANC

            // 6. ANC submode 0x033B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_ANC_SUB_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 2]); // Strong ANC

            // 7. Game mode 0x0313
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_GAME_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0]); // Disabled

            // 8. Dual device 0x0326
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECT_TWO_DEVICES);
            send_mock_response(&mut device, req.command, vec![0x00, 1]); // Enabled
        });

        // Test that custom device name does NOT affect identification:
        // product ID is derived from firmware code TW400
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "My Renamed Headset", None, None)
                .expect("Free Pro 3 session initialization should succeed");

        assert_eq!(snapshot.model, "EarFun Free Pro 3");
        assert_eq!(snapshot.product_id, EARFUN_FREE_PRO_3_PRODUCT_ID);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 90);
        assert_eq!(snapshot.noise_mode, Some(4)); // Strong ANC
        assert_eq!(snapshot.noise_modes.len(), 10);
        assert!(!snapshot.supports_wear_detection);
        assert!(!snapshot.game_mode_enabled.unwrap());
        assert!(snapshot.dual_device_enabled.unwrap());

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_unknown_model_session_initialization() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Firmware version 0x0309 with unknown hardware code
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::FIRMWARE_VERSION);
            let mut fw_payload = vec![0x00];
            fw_payload.extend_from_slice(b"UNKNOWN_CHIP_v1.0");
            send_mock_response(&mut device, req.command, fw_payload);

            // Unknown model enters read-only mode and queries basic battery
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_LEFT);
            send_mock_response(&mut device, req.command, vec![0x00, 50]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_RIGHT);
            send_mock_response(&mut device, req.command, vec![0x00, 50]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_CASE);
            send_mock_response(&mut device, req.command, vec![0x00, 70]);
        });

        // Unknown model with no discovery product_id should be treated as read-only unknown device
        let (snapshot, _) = initialize_earfun_session(&mut client, "Generic EarFun", None, None)
            .expect("Unknown model session initialization should succeed in read-only mode");

        assert_eq!(snapshot.model, crate::t!("device-unknown-model"));
        assert!(!snapshot.is_known_model);
        assert!(snapshot.read_only);
        assert_eq!(snapshot.battery.left.unwrap().percentage, 50);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 50);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 70);
        assert!(snapshot.noise_mode.is_none());
        assert!(!snapshot.supports_wear_detection);
        assert!(!snapshot.supports_dual_device);
        assert!(!snapshot.supports_game_mode);
        assert!(!snapshot.supports_equalizer);

        device_thread.join().unwrap();
    }
}
