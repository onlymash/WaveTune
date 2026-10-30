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
    use crate::protocol::brands::earfun::{
        EarFunAudioCodec, EarFunCommands, EarFunFrameDecoder, EarFunPacket,
    };

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

    fn send_mock_032c(
        device: &mut UnixStream,
        slot: u8,
        name: &[u8],
        lap: [u8; 3],
        connected: bool,
    ) {
        let mut payload = vec![0x00, slot];
        let mut name_buf = [0u8; 21];
        let len = name.len().min(21);
        name_buf[..len].copy_from_slice(&name[..len]);
        payload.extend_from_slice(&name_buf);
        payload.extend_from_slice(&lap);
        payload.push(if connected { 1 } else { 0 });
        let pkt = EarFunPacket {
            version: 0x03,
            flags: 0x00,
            vendor_id: 0x000A,
            command: EarFunCommands::GET_CONNECTED_DEVICES,
            payload,
        };
        device.write_all(&pkt.encode()).unwrap();
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

            // 10. Paired devices 0x032C (slots 0..10)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECTED_DEVICES);
            send_mock_032c(&mut device, 0, b"PC-Work", [0x01, 0x02, 0x03], true);
            send_mock_032c(&mut device, 1, b"Phone", [0x04, 0x05, 0x06], false);
            for slot in 2..10 {
                send_mock_032c(&mut device, slot, b"", [0, 0, slot], false);
            }

            // 11. Audio codec 0x032F
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AUDIO_CODEC);
            send_mock_response(&mut device, req.command, vec![0x00, 19]); // aptX Lossless
        });

        let (snapshot, _) =
            initialize_earfun_session(&mut client, "EarFun Air Pro 4", None, None, None)
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
        assert!(snapshot.supports_paired_devices);
        assert!(snapshot.supports_audio_codec);
        assert_eq!(snapshot.audio_codec, Some(19));

        let devs = snapshot.paired_devices.unwrap();
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].name, "PC-Work");
        assert!(devs[0].connected);
        assert_eq!(devs[1].name, "Phone");
        assert!(!devs[1].connected);
        assert!(snapshot.paired_device_controls_enabled.is_ok());

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_session_initialization_with_offline_case_battery() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. FW query 0x0309
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
            send_mock_response(&mut device, req.command, vec![0x00, 95]);

            // 4. Battery case 0x0317 returns 0xFF (255 = offline/disconnected sentinel)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::BATTERY_CASE);
            send_mock_response(&mut device, req.command, vec![0x00, 0xFF]);

            // 5. Ambient sound mode 0x0315
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AMBIENT_SOUND);
            send_mock_response(&mut device, req.command, vec![0x00, 0]); // Off

            // 6. Game mode 0x0313
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_GAME_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0]);

            // 7. In-ear detection 0x034A
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_IN_EAR_DETECTION);
            send_mock_response(&mut device, req.command, vec![0x00, 0]);

            // 8. Dual device 0x0326
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECT_TWO_DEVICES);
            send_mock_response(&mut device, req.command, vec![0x00, 0]);

            // 9. Paired devices 0x032C
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECTED_DEVICES);
            for slot in 0..10 {
                send_mock_032c(&mut device, slot, b"", [0, 0, slot], false);
            }

            // 10. Audio codec 0x032F
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AUDIO_CODEC);
            send_mock_response(&mut device, req.command, vec![0x00, 8]); // LDAC
        });

        let (snapshot, _) =
            initialize_earfun_session(&mut client, "EarFun Air Pro 4", None, None, None)
                .expect("session initialization should succeed");

        assert_eq!(snapshot.battery.left.unwrap().percentage, 90);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 95);
        // Case battery reporting 0xFF (255) must be parsed as None (unavailable/unreported), not 255%
        assert!(snapshot.battery.case.is_none());
        assert_eq!(snapshot.audio_codec, Some(8));

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
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "Custom Air S Name", None, None, None)
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

            // 9. Paired devices 0x032C (slots 0..10)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECTED_DEVICES);
            for slot in 0..10 {
                send_mock_032c(&mut device, slot, b"", [0, 0, slot], false);
            }

            // 10. Audio codec 0x032F
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_AUDIO_CODEC);
            send_mock_response(&mut device, req.command, vec![0x00, 3]); // aptX Adaptive
        });

        // Test that custom device name does NOT affect identification:
        // product ID is derived from firmware code TW400
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "My Renamed Headset", None, None, None)
                .expect("Free Pro 3 session initialization should succeed");

        assert_eq!(snapshot.model, "EarFun Free Pro 3");
        assert_eq!(snapshot.product_id, EARFUN_FREE_PRO_3_PRODUCT_ID);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 90);
        assert_eq!(snapshot.noise_mode, Some(4)); // Strong ANC
        assert_eq!(snapshot.noise_modes.len(), 10);
        assert!(!snapshot.supports_wear_detection);
        assert!(!snapshot.game_mode_enabled.unwrap());
        assert!(snapshot.dual_device_enabled.unwrap());
        assert!(snapshot.supports_audio_codec);
        assert_eq!(snapshot.audio_codec, Some(3));

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
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "Generic EarFun", None, None, None)
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

    #[test]
    fn test_earfun_read_paired_devices() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECTED_DEVICES);
            send_mock_032c(&mut device, 0, b"Arch-Desktop", [0x11, 0x22, 0x33], false);
            send_mock_032c(&mut device, 1, b"Pixel-Phone", [0x44, 0x55, 0x66], true);
            for slot in 2..10 {
                send_mock_032c(&mut device, slot, b"", [0, 0, slot], false);
            }
        });

        let mut decoder = EarFunFrameDecoder::default();
        let devices = read_earfun_paired_devices(&mut client, &mut decoder, None).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Pixel-Phone");
        assert!(devices[0].connected);
        assert_eq!(devices[0].device_type, 0);
        assert_eq!(devices[1].name, "Arch-Desktop");
        assert!(!devices[1].connected);
        assert_eq!(devices[1].device_type, 0);

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_audio_codec_ldac_conflict_and_reboot() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Air Pro 4 receives disable dual device 0x0327 [0] because LDAC was requested and dual device was ON
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_CONNECT_TWO_DEVICES);
            assert_eq!(req.payload, vec![0x00]);

            // 2. Headset receives set codec LDAC 0x032E [0x08]
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_AUDIO_CODEC);
            assert_eq!(req.payload, vec![0x08]);

            // 3. Headset receives reboot command 0x0308
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::REBOOT);
            assert!(req.payload.is_empty());
        });

        let (tx, rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let actual = set_audio_codec(
            &mut client,
            &mut decoder,
            EARFUN_AIR_PRO_4_PRODUCT_ID,
            EarFunAudioCodec::Ldac.as_u8(),
            Some(true), // dual device was ON
            &tx,
        )
        .expect("set audio codec should succeed");

        assert_eq!(actual, EarFunAudioCodec::Ldac.as_u8());

        // Verify updates sent: DualDeviceConnectionEnabled(false) and DeviceRebooting
        let updates: Vec<crate::bluetooth::UiUpdate> = rx.try_iter().collect();
        assert!(updates.iter().any(|u| matches!(
            u,
            crate::bluetooth::UiUpdate::DualDeviceConnectionEnabled(false)
        )));
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, crate::bluetooth::UiUpdate::DeviceRebooting))
        );

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_dual_device_ldac_conflict_and_reboot() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // 1. Because LDAC was active, enabling dual device causes fallback to aptX Adaptive (0x03)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_AUDIO_CODEC);
            assert_eq!(req.payload, vec![EarFunAudioCodec::AptXAdaptive.as_u8()]);

            // 2. Dual device is enabled 0x0327 [0x01]
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_CONNECT_TWO_DEVICES);
            assert_eq!(req.payload, vec![0x01]);

            // 3. Headset receives reboot command 0x0308
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::REBOOT);
            assert!(req.payload.is_empty());
        });

        let (tx, rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let actual = set_dual_device(
            &mut client,
            &mut decoder,
            EARFUN_AIR_PRO_4_PRODUCT_ID,
            true,
            Some(EarFunAudioCodec::Ldac.as_u8()), // current codec is LDAC
            &tx,
        )
        .expect("set dual device should succeed");

        assert!(actual);

        let updates: Vec<crate::bluetooth::UiUpdate> = rx.try_iter().collect();
        assert!(updates.iter().any(|u| matches!(u, crate::bluetooth::UiUpdate::AudioCodec(c) if *c == EarFunAudioCodec::AptXAdaptive.as_u8())));
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, crate::bluetooth::UiUpdate::DeviceRebooting))
        );

        device_thread.join().unwrap();
    }
}
