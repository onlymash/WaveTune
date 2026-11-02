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

    fn send_mock_gestures_030b(device: &mut UnixStream) {
        let gestures_payload = vec![
            0x00, // Status
            0x00, 0x02, // L single: Vol Down (2)
            0x00, 0x01, // R single: Vol Up (1)
            0x00, 0x04, // L double: Play/Pause (4)
            0x00, 0x04, // R double: Play/Pause (4)
            0x00, 0x05, // L triple: Previous (5)
            0x00, 0x03, // R triple: Next (3)
            0x00, 0x09, // L long: Ambient (9)
            0x00, 0x06, // R long: Voice Assistant (6)
        ];
        send_mock_response(device, EarFunCommands::GET_TOUCH_ACTION, gestures_payload);
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

            // 12. Touch gestures 0x030B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_ACTION);
            send_mock_gestures_030b(&mut device);

            // 13. Touch lock 0x0335
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x00]); // Disabled

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_FIND_DEVICE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x00]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_VOICE_PROMPT_VOLUME);
            send_mock_response(&mut device, req.command, vec![0x00, 0x02]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_MICROPHONE_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x01]);
        });

        let (snapshot, _) =
            initialize_earfun_session(&mut client, "EarFun Air Pro 4", None, None, None)
                .expect("session initialization should succeed");

        assert_eq!(snapshot.identity.model, "EarFun Air Pro 4");
        assert_eq!(snapshot.identity.firmware.left.as_deref(), Some("0.4.1"));
        assert_eq!(snapshot.battery.left.unwrap().percentage, 90);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 100);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 80);
        assert_eq!(snapshot.noise.current_mode, Some(3)); // AI Environment Adaptive
        assert_eq!(snapshot.sound.game_mode_enabled, Some(true));
        assert_eq!(snapshot.controls.in_ear_detection_enabled, Some(true));
        assert_eq!(snapshot.dual_device.dual_device_enabled, Some(true));
        assert!(snapshot.dual_device.supports_dual_device);
        assert!(snapshot.sound.supports_audio_codec);
        assert_eq!(snapshot.sound.audio_codec, Some(19));
        assert_eq!(snapshot.controls.key_functions.len(), 8);
        assert!(snapshot.controls.supports_touch_lock);
        assert_eq!(snapshot.controls.touch_lock_enabled, Some(false));
        assert!(snapshot.controls.supports_find_device);
        assert!(!snapshot.controls.is_finding_earbuds);
        assert!(snapshot.controls.supports_prompt_volume);
        assert_eq!(snapshot.controls.prompt_volume, Some(2));
        assert_eq!(snapshot.controls.prompt_volume_range, Some((0, 4)));
        assert!(snapshot.controls.supports_microphone_mode);
        assert_eq!(snapshot.controls.microphone_mode, Some(1));

        let devs = snapshot.dual_device.paired_devices.unwrap();
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].name, "PC-Work");
        assert!(devs[0].connected);
        assert_eq!(devs[1].name, "Phone");
        assert!(!devs[1].connected);
        assert!(snapshot.dual_device.controls_enabled.is_ok());

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

            // 11. Touch gestures 0x030B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_ACTION);
            send_mock_gestures_030b(&mut device);

            // 12. Touch lock 0x0335
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x01]); // Enabled

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_FIND_DEVICE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x03]);
        });

        let (snapshot, _) =
            initialize_earfun_session(&mut client, "EarFun Air Pro 4", None, None, None)
                .expect("session initialization should succeed");

        assert_eq!(snapshot.battery.left.unwrap().percentage, 90);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 95);
        // Case battery reporting 0xFF (255) must be parsed as None (unavailable/unreported), not 255%
        assert!(snapshot.battery.case.is_none());
        assert_eq!(snapshot.sound.audio_codec, Some(8));
        assert_eq!(snapshot.controls.touch_lock_enabled, Some(true));
        assert!(snapshot.controls.is_finding_earbuds);
        assert!(snapshot.controls.supports_prompt_volume);
        assert_eq!(snapshot.controls.prompt_volume, None);
        assert!(snapshot.controls.supports_microphone_mode);

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

            // 7. Paired devices 0x032C (slots 0..10)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_CONNECTED_DEVICES);
            for slot in 0..10 {
                send_mock_032c(&mut device, slot, b"", [0, 0, slot], false);
            }

            // 8. Touch gestures 0x030B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_ACTION);
            send_mock_gestures_030b(&mut device);
        });

        // Test that custom device name does NOT affect identification:
        // product ID is derived from firmware code TW201
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "Custom Air S Name", None, None, None)
                .expect("Air S session initialization should succeed");

        assert_eq!(snapshot.identity.model, "EarFun Air S");
        assert_eq!(snapshot.identity.product_id, EARFUN_AIR_S_PRODUCT_ID);
        assert_eq!(snapshot.identity.firmware.left.as_deref(), Some("0.1.2"));
        assert_eq!(snapshot.battery.left.unwrap().percentage, 80);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 85);
        assert!(snapshot.battery.case.is_none());
        assert_eq!(snapshot.noise.current_mode, Some(1)); // Standard ANC
        assert_eq!(snapshot.noise.modes.len(), 3);
        assert!(!snapshot.wear.supports_wear_detection);
        assert!(snapshot.sound.game_mode_enabled.unwrap());
        assert!(snapshot.dual_device.dual_device_enabled.unwrap());
        assert_eq!(snapshot.controls.key_functions.len(), 8);
        assert!(!snapshot.controls.supports_touch_lock);
        assert_eq!(snapshot.controls.touch_lock_enabled, None);

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

            // 11. Touch gestures 0x030B
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_ACTION);
            send_mock_gestures_030b(&mut device);

            // 12. Touch lock 0x0335
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_TOUCH_MODE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x00]); // Disabled

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_FIND_DEVICE);
            send_mock_response(&mut device, req.command, vec![0x00, 0x00]);
        });

        // Test that custom device name does NOT affect identification:
        // product ID is derived from firmware code TW400
        let (snapshot, _) =
            initialize_earfun_session(&mut client, "My Renamed Headset", None, None, None)
                .expect("Free Pro 3 session initialization should succeed");

        assert_eq!(snapshot.identity.model, "EarFun Free Pro 3");
        assert_eq!(snapshot.identity.product_id, EARFUN_FREE_PRO_3_PRODUCT_ID);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 90);
        assert_eq!(snapshot.noise.current_mode, Some(4)); // Strong ANC
        assert_eq!(snapshot.noise.modes.len(), 10);
        assert!(!snapshot.wear.supports_wear_detection);
        assert!(!snapshot.sound.game_mode_enabled.unwrap());
        assert!(snapshot.dual_device.dual_device_enabled.unwrap());
        assert!(snapshot.sound.supports_audio_codec);
        assert_eq!(snapshot.sound.audio_codec, Some(3));
        assert_eq!(snapshot.controls.key_functions.len(), 8);
        assert!(snapshot.controls.supports_touch_lock);
        assert_eq!(snapshot.controls.touch_lock_enabled, Some(false));
        assert!(snapshot.controls.supports_find_device);
        assert!(!snapshot.controls.supports_prompt_volume);
        assert!(!snapshot.controls.supports_microphone_mode);

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

        assert_eq!(snapshot.identity.model, crate::t!("device-unknown-model"));
        assert!(!snapshot.identity.is_known_model);
        assert!(snapshot.identity.read_only);
        assert_eq!(snapshot.battery.left.unwrap().percentage, 50);
        assert_eq!(snapshot.battery.right.unwrap().percentage, 50);
        assert_eq!(snapshot.battery.case.unwrap().percentage, 70);
        assert!(snapshot.noise.current_mode.is_none());
        assert!(!snapshot.wear.supports_wear_detection);
        assert!(!snapshot.dual_device.supports_dual_device);
        assert!(!snapshot.sound.supports_game_mode);
        assert!(!snapshot.sound.supports_equalizer);

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
        let devices = read_earfun_paired_devices(&mut client, &mut decoder, None, None).unwrap();
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

    #[test]
    fn test_earfun_set_gesture() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // SET_TOUCH_ACTION 0x030A: payload [interaction=0x11 (Long Press), function=9 (Ambient Sound), 0x00, 0x00] for Left earbud
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_TOUCH_ACTION);
            assert_eq!(req.payload, vec![0x11, 0x09, 0x00, 0x00]);
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        set_touch_action(
            &mut client,
            &mut decoder,
            crate::protocol::core::types::EarSide::Left,
            crate::protocol::core::types::GestureAction::SqueezeAndHold,
            9,
            Some(&tx),
        )
        .expect("set touch action should succeed");

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_set_touch_lock() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            // SET_TOUCH_MODE 0x0334: payload [0x01] (lock enabled)
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_TOUCH_MODE);
            assert_eq!(req.payload, vec![0x01]);
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        let res = set_touch_lock(&mut client, &mut decoder, true, Some(&tx))
            .expect("set touch lock should succeed");
        assert!(res);

        device_thread.join().unwrap();
    }

    #[test]
    fn test_earfun_apply_equalizer_preset() {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            for i in 0..10 {
                let req = read_mock_request(&mut device);
                assert_eq!(req.command, EarFunCommands::SET_EQUALIZER_BAND);
                assert_eq!(req.payload[0], 0xA1 + i as u8);
                // Reply with ACK 0x0F81 [0x01]
                let ack_pkt = EarFunPacket::new_v3(0x0F81, vec![0x01]);
                device.write_all(&ack_pkt.encode()).unwrap();
            }
        });

        let (tx, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        apply_equalizer_preset(
            &mut client,
            &mut decoder,
            EARFUN_AIR_PRO_4_PRODUCT_ID,
            2,
            Some(&tx),
        )
        .expect("apply_equalizer_preset should succeed");

        device_thread.join().unwrap();

        // Unknown product or preset should fail without writing to device
        assert!(apply_equalizer_preset(&mut client, &mut decoder, 0x999999, 0, Some(&tx)).is_err());
        assert!(
            apply_equalizer_preset(
                &mut client,
                &mut decoder,
                EARFUN_AIR_PRO_4_PRODUCT_ID,
                99,
                Some(&tx)
            )
            .is_err()
        );
    }

    #[test]
    fn test_earfun_custom_equalizer_driver_workflow() {
        use crate::app::events::{UiUpdate, WorkerCommand};
        use crate::bluetooth::driver::HeadsetDriver;

        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let mut driver = EarFunDriver::new();
        // Set a mock snapshot for product ID
        driver.set_snapshot_for_test(EARFUN_AIR_PRO_4_PRODUCT_ID);

        let (tx, rx) = mpsc::channel();

        // 1. Add Custom Equalizer - local only, no bytes on wire
        let bands = vec![
            (31, 1),
            (62, 2),
            (125, 3),
            (250, 2),
            (500, 1),
            (1000, 0),
            (2000, -1),
            (4000, -2),
            (8000, -3),
            (16000, -4),
        ];
        driver
            .handle_command(
                &mut client,
                WorkerCommand::AddCustomEqualizer {
                    name: "Jazz".to_string(),
                    bands: bands.clone(),
                },
                &tx,
            )
            .expect("AddCustomEqualizer should succeed");

        let mut got_custom = false;
        while let Ok(msg) = rx.try_recv() {
            if let UiUpdate::CustomEqualizers(list) = msg {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0].name, "Jazz");
                assert_eq!(list[0].id, 100);
                assert!(!list[0].is_selected);
                got_custom = true;
            }
        }
        assert!(got_custom, "Should emit CustomEqualizers on add");

        // 2. Select Custom Equalizer - writes 10 frames to headset
        let device_thread = thread::spawn(move || {
            for i in 0..10 {
                let req = read_mock_request(&mut device);
                assert_eq!(req.command, EarFunCommands::SET_EQUALIZER_BAND);
                assert_eq!(req.payload[0], 0xA1 + i as u8);
                let ack_pkt = EarFunPacket::new_v3(0x0F81, vec![0x01]);
                device.write_all(&ack_pkt.encode()).unwrap();
            }
        });

        driver
            .handle_command(&mut client, WorkerCommand::SelectCustomEqualizer(100), &tx)
            .expect("SelectCustomEqualizer should succeed");

        device_thread.join().unwrap();

        let mut selected_eq = false;
        while let Ok(msg) = rx.try_recv() {
            if let UiUpdate::Equalizer(id) = msg {
                if id == 100 {
                    selected_eq = true;
                }
            }
        }
        assert!(
            selected_eq,
            "Should emit Equalizer(100) when custom selected"
        );
    }

    #[test]
    fn test_earfun_controls_require_ack_and_matching_readback() {
        use crate::bluetooth::brands::earfun::features::controls::{
            set_find_mode, set_microphone_mode, set_prompt_volume,
        };
        use crate::protocol::brands::earfun::{EarFunFindMode, EarFunMicrophoneMode};

        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let device_thread = thread::spawn(move || {
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_FIND_DEVICE);
            assert_eq!(req.payload, vec![3]);
            send_mock_response(&mut device, req.command, vec![0]);
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_FIND_DEVICE);
            send_mock_response(&mut device, req.command, vec![0, 3]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_VOICE_PROMPT_VOLUME);
            assert_eq!(req.payload, vec![1]); // UI level 3 maps to wire level 1.
            send_mock_response(&mut device, req.command, vec![0]);
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_VOICE_PROMPT_VOLUME);
            send_mock_response(&mut device, req.command, vec![0, 1]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_MICROPHONE_MODE);
            assert_eq!(req.payload, vec![2]);
            send_mock_response(&mut device, req.command, vec![0]);
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_MICROPHONE_MODE);
            send_mock_response(&mut device, req.command, vec![0, 2]);

            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::SET_MICROPHONE_MODE);
            assert_eq!(req.payload, vec![1]);
            send_mock_response(&mut device, req.command, vec![0]);
            let req = read_mock_request(&mut device);
            assert_eq!(req.command, EarFunCommands::GET_MICROPHONE_MODE);
            send_mock_response(&mut device, req.command, vec![0, 0]);
        });

        let (updates, _rx) = mpsc::channel();
        let mut decoder = EarFunFrameDecoder::default();
        assert_eq!(
            set_find_mode(
                &mut client,
                &mut decoder,
                EarFunFindMode::Both,
                Some(&updates)
            )
            .unwrap(),
            EarFunFindMode::Both
        );
        assert_eq!(
            set_prompt_volume(&mut client, &mut decoder, 3, Some(&updates)).unwrap(),
            3
        );
        assert_eq!(
            set_microphone_mode(
                &mut client,
                &mut decoder,
                EarFunMicrophoneMode::Right,
                Some(&updates)
            )
            .unwrap(),
            EarFunMicrophoneMode::Right
        );
        assert!(
            set_microphone_mode(
                &mut client,
                &mut decoder,
                EarFunMicrophoneMode::Left,
                Some(&updates)
            )
            .is_err(),
            "mismatching readback must not be reported as success"
        );

        device_thread.join().unwrap();
    }
}
