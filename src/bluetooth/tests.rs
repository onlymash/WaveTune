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

use super::discovery::{
    ONEPLUS_COMPANY_ID, OPPO_COMPANY_ID, REALME_COMPANY_ID, identify_brand, is_headset,
};
use super::session::{
    set_bass_engine_enabled, set_bass_engine_level, set_equalizer, set_game_mode, set_hi_res,
    set_noise_mode, set_paired_device, set_spatial_audio,
};
use super::*;
use crate::protocol::{
    AppPacket, FrameDecoder, PairedDeviceAction, WearState, encode_frame,
    noise_mode_write_payload_for, parse_paired_devices_response,
};
use std::sync::mpsc;
use std::{
    collections::HashMap,
    io::{Read, Write},
    mem::size_of,
    os::unix::net::UnixStream,
    thread,
    time::Duration,
};

#[test]
fn detects_headsets_from_bluez_icon_or_class_of_device() {
    assert!(is_headset(Some("audio-headset"), None));
    assert!(is_headset(None, Some((0x04 << 8) | (0x06 << 2))));
    assert!(!is_headset(None, Some((0x04 << 8) | (0x05 << 2))));
    assert!(!is_headset(Some("audio-speakers"), None));
}

#[test]
fn identifies_oppo_oneplus_and_realme_from_company_identifiers() {
    let empty_uuids = std::collections::HashSet::new();
    let mut manufacturer_data = HashMap::new();
    manufacturer_data.insert(OPPO_COMPANY_ID, vec![]);
    assert_eq!(
        identify_brand(&manufacturer_data, false, "", &empty_uuids),
        Some(DeviceBrand::Oppo)
    );

    manufacturer_data.clear();
    assert_eq!(
        identify_brand(&manufacturer_data, true, "", &empty_uuids),
        Some(DeviceBrand::Oppo)
    );
    assert_eq!(
        identify_brand(&manufacturer_data, false, "", &empty_uuids),
        None
    );

    manufacturer_data.insert(ONEPLUS_COMPANY_ID, vec![]);
    assert_eq!(
        identify_brand(&manufacturer_data, false, "", &empty_uuids),
        Some(DeviceBrand::OnePlus)
    );

    manufacturer_data.clear();
    manufacturer_data.insert(REALME_COMPANY_ID, vec![]);
    assert_eq!(
        identify_brand(&manufacturer_data, false, "", &empty_uuids),
        Some(DeviceBrand::Realme)
    );

    manufacturer_data.clear();
    manufacturer_data.insert(0x5A70, vec![]);
    assert_eq!(
        identify_brand(&manufacturer_data, false, "", &empty_uuids),
        Some(DeviceBrand::EarFun)
    );

    manufacturer_data.clear();
    assert_eq!(
        identify_brand(&manufacturer_data, false, "EarFun Air Pro 4", &empty_uuids),
        Some(DeviceBrand::EarFun)
    );
}

#[test]
fn extracts_earfun_product_id_from_manufacturer_data() {
    use crate::bluetooth::discovery::extract_earfun_product_id;
    let mac = bluer::Address([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    let mut manufacturer_data = HashMap::new();
    // [0x01, 0x02, mac[2..6], pid[0], pid[1]]
    let data = vec![0x01, 0x02, 0x33, 0x44, 0x55, 0x66, 0x45, 0x35];
    manufacturer_data.insert(0x5A70, data);
    assert_eq!(
        extract_earfun_product_id(&mac, &manufacturer_data),
        Some(0x004535)
    );

    // Mismatched MAC suffix returns None
    let other_mac = bluer::Address([0x11, 0x22, 0x99, 0x88, 0x77, 0x66]);
    assert_eq!(
        extract_earfun_product_id(&other_mac, &manufacturer_data),
        None
    );
}

#[test]
fn unsupported_models_stop_after_read_only_identification() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let device_thread = thread::spawn(move || {
        let handshake = read_mock_request(&mut device);
        assert_eq!(handshake.command, 0x0100);
        send_mock_response(&mut device, &handshake, vec![0, 0]);

        let product = read_mock_request(&mut device);
        assert_eq!(product.command, 0x0103);
        send_mock_response(&mut device, &product, vec![0, 0x56, 0x34, 0x12]);

        let color = read_mock_request(&mut device);
        assert_eq!(color.command, 0x010b);
        send_mock_response(&mut device, &color, vec![0, 1]);

        device
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut extra = [0];
        assert!(device.read(&mut extra).is_err());
    });

    let (snapshot, _) = super::session::initialize_session(&mut client, "test headset", None)
        .expect("unknown models should connect in read-only identification mode");
    assert_eq!(snapshot.identity.product_id, 0x123456);
    assert_eq!(snapshot.identity.color_id, Some(1));
    assert!(!snapshot.identity.is_known_model);
    assert!(snapshot.identity.read_only);
    assert!(snapshot.noise.supported_indices.is_empty());
    assert!(snapshot.dual_device.paired_devices.is_err());
    device_thread.join().unwrap();
}

#[test]
fn catalog_model_uses_runtime_capabilities_without_requiring_hardware_verification() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let device_thread = thread::spawn(move || {
        let handshake = read_mock_request(&mut device);
        send_mock_response(&mut device, &handshake, vec![0, 0]);
        let product = read_mock_request(&mut device);
        send_mock_response(&mut device, &product, vec![0, 0x10, 0x74, 0x06]);
        let color = read_mock_request(&mut device);
        assert_eq!(color.command, 0x010b);
        send_mock_response(&mut device, &color, vec![0, 2]);
        let events = read_mock_request(&mut device);
        assert_eq!(events.command, 0x0200);
        send_mock_response(&mut device, &events, vec![0, 0]);
        let battery = read_mock_request(&mut device);
        assert_eq!(battery.command, 0x0106);
        send_mock_response(&mut device, &battery, vec![0, 0]);
        let firmware = read_mock_request(&mut device);
        assert_eq!(firmware.command, 0x0105);
        send_mock_response(
            &mut device,
            &firmware,
            vec![0, 1, b'1', b',', b'0', b',', b'v'],
        );
        let wear = read_mock_request(&mut device);
        assert_eq!(wear.command, 0x0109);
        send_mock_response(&mut device, &wear, vec![0, 0]);

        let noise = read_mock_request(&mut device);
        assert_eq!(noise.command, 0x010c);
        send_mock_response(&mut device, &noise, vec![0, 1, 1]);

        let feature = read_mock_request(&mut device);
        assert_eq!(feature.command, 0x010d);
        send_mock_response(&mut device, &feature, vec![0, 1, 17, 1]);

        let paired = read_mock_request(&mut device);
        assert_eq!(paired.command, 0x0112);
        send_mock_response(&mut device, &paired, vec![0, 0]);

        let eq = read_mock_request(&mut device);
        assert_eq!(eq.command, 0x010f);
        send_mock_response(&mut device, &eq, vec![0, 7]);

        let spatial = read_mock_request(&mut device);
        assert_eq!(spatial.command, 0x012a);
        send_mock_response(&mut device, &spatial, vec![0, 1]);

        let game = read_mock_request(&mut device);
        assert_eq!(game.command, 0x010d);
        send_mock_response(&mut device, &game, vec![0, 1, 6, 1]);

        let hi_res = read_mock_request(&mut device);
        assert_eq!(hi_res.command, 0x010d);
        assert_eq!(hi_res.payload, vec![1, 24]);
        send_mock_response(&mut device, &hi_res, vec![0, 1, 24, 1]);

        let touch = read_mock_request(&mut device);
        assert_eq!(touch.command, 0x0108);
        assert_eq!(touch.payload, vec![2, 3, 1]);
        send_mock_response(&mut device, &touch, vec![0, 0]);

        let cycle = read_mock_request(&mut device);
        assert_eq!(cycle.command, 0x010c);
        assert_eq!(cycle.payload, vec![2, 1]);
        send_mock_response(&mut device, &cycle, vec![0, 1, 6]);

        let priority = read_mock_request(&mut device);
        assert_eq!(priority.command, 0x0132);
        assert_eq!(priority.payload, vec![2]);
        send_mock_response(&mut device, &priority, vec![0, 0]);

        let in_ear = read_mock_request(&mut device);
        assert_eq!(in_ear.command, 0x010d);
        assert_eq!(in_ear.payload, vec![1, 4]);
        send_mock_response(&mut device, &in_ear, vec![0, 1, 4, 1]);

        device
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut extra = [0];
        assert!(device.read(&mut extra).is_err());
    });

    let (snapshot, _) = super::session::initialize_session(&mut client, "Enco X3", None)
        .expect("catalog models use model profiles even without write capability bits");
    assert_eq!(snapshot.identity.product_id, 0x067410);
    assert_eq!(snapshot.identity.color_id, Some(2));
    assert_eq!(snapshot.identity.model, "OPPO Enco X3");
    assert!(!snapshot.identity.read_only);
    assert!(!snapshot.noise.supported_indices.is_empty());
    assert!(
        !snapshot
            .diagnostics
            .capabilities
            .iter()
            .any(|byte| *byte != 0)
    );
    assert!(snapshot.wear.supports_wear_detection);
    assert!(snapshot.dual_device.supports_dual_device);
    assert!(snapshot.dual_device.paired_devices.is_ok());
    assert!(snapshot.sound.supports_equalizer);
    assert_eq!(snapshot.sound.equalizer_preset, Some(7));
    assert!(snapshot.sound.supports_spatial_audio);
    assert_eq!(snapshot.sound.spatial_audio_mode, Some(1));
    assert!(snapshot.sound.supports_game_mode);
    assert_eq!(snapshot.sound.game_mode_enabled, Some(true));
    assert!(snapshot.sound.supports_hi_res);
    assert_eq!(snapshot.sound.hi_res_enabled, Some(true));
    assert!(!snapshot.sound.supports_bass_engine);
    assert_eq!(snapshot.sound.bass_engine_enabled, None);
    assert!(snapshot.controls.supports_touch_customization);
    assert_eq!(snapshot.controls.in_ear_detection_enabled, Some(true));
    device_thread.join().unwrap();
}

#[test]
fn read_only_model_detects_and_displays_supported_queries() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let device_thread = thread::spawn(move || {
        let events = read_mock_request(&mut device);
        assert_eq!(events.command, 0x0200);
        send_mock_response(&mut device, &events, vec![0, 2, 1, 2]);

        let subscribe = read_mock_request(&mut device);
        assert_eq!(subscribe.command, 0x0205);
        assert_eq!(subscribe.payload, vec![2, 1, 2]);
        send_mock_response(&mut device, &subscribe, vec![0]);

        let battery = read_mock_request(&mut device);
        assert_eq!(battery.command, 0x0106);
        assert_eq!(battery.sequence, 0xf0);
        send_mock_response(&mut device, &battery, vec![0, 2, 1, 0x46, 2, 0x3c]);

        let firmware = read_mock_request(&mut device);
        assert_eq!(firmware.command, 0x0105);
        send_mock_response(&mut device, &firmware, vec![0x0e]);

        let wear = read_mock_request(&mut device);
        assert_eq!(wear.command, 0x0109);
        send_mock_response(&mut device, &wear, vec![0, 2, 1, 3, 2, 1]);
    });

    let (battery, firmware, wear, features, errors) =
        super::session::probe_read_only_features(&mut client, &mut FrameDecoder::default(), None);
    assert_eq!(battery.left.unwrap().percentage, 0x46);
    assert_eq!(battery.right.unwrap().percentage, 0x3c);
    assert!(firmware.left.is_none());
    assert!(wear.left.unwrap().in_ear);
    assert_eq!(features.len(), 4);
    assert_eq!(errors.len(), 1);
    device_thread.join().unwrap();
}

#[test]
fn socket_address_matches_linux_rfcomm_layout() {
    #[repr(C)]
    struct SockAddrRc {
        family: libc::sa_family_t,
        address: [u8; 6],
        channel: u8,
    }
    assert_eq!(size_of::<SockAddrRc>(), 10);
}

#[test]
fn noise_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let device_thread = thread::spawn(move || {
        let setting = read_mock_request(&mut device);
        assert_eq!(setting.command, 0x0404);
        assert_eq!(setting.payload, vec![1, 1, 0x10]);
        send_mock_response(&mut device, &setting, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x010c);
        assert_eq!(readback.payload, vec![1, 1]);
        send_mock_response(&mut device, &readback, vec![0, 1, 1, 0x10]);
    });

    let (updates, _) = mpsc::channel();
    let modes = crate::compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    let actual = set_noise_mode(
        &mut client,
        &mut FrameDecoder::default(),
        &[4],
        modes,
        &mut worn_state(),
        true,
        4,
        &updates,
    );
    assert_eq!(actual, Ok(4));
    device_thread.join().unwrap();
}

#[test]
fn paired_device_write_waits_for_ack_and_matching_readback() {
    for (action, initially_connected, final_connected) in [
        (PairedDeviceAction::Connect, false, true),
        (PairedDeviceAction::Disconnect, true, false),
    ] {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();

        let listed_devices = paired_device_response(initially_connected);
        let devices = parse_paired_devices_response(&listed_devices).unwrap();
        let address = devices[0].address;
        let device_thread = thread::spawn(move || {
            let command = read_mock_request(&mut device);
            assert_eq!(command.command, 0x0429);
            assert_eq!(command.payload, action.payload(address));
            send_mock_response(&mut device, &command, vec![0]);

            let readback = read_mock_request(&mut device);
            assert_eq!(readback.command, 0x0112);
            send_mock_response(
                &mut device,
                &readback,
                paired_device_response(final_connected),
            );
        });

        let (updates, _) = mpsc::channel();
        let result = set_paired_device(
            &mut client,
            &mut FrameDecoder::default(),
            &Ok(true),
            &Ok(devices),
            address,
            action,
            &updates,
        )
        .unwrap();
        assert!(result.1);
        assert_eq!(result.0[0].connected, final_connected);
        device_thread.join().unwrap();
    }
}

#[test]
fn paired_device_write_is_blocked_when_dual_connection_is_disabled() {
    let (mut client, _device) = UnixStream::pair().unwrap();
    let listed_devices = paired_device_response(false);
    let devices = parse_paired_devices_response(&listed_devices).unwrap();
    let result = set_paired_device(
        &mut client,
        &mut FrameDecoder::default(),
        &Ok(false),
        &Ok(devices.clone()),
        devices[0].address,
        PairedDeviceAction::Connect,
        &mpsc::channel().0,
    );
    assert!(result.is_err());
}

#[test]
fn paired_device_write_rejects_a_mismatched_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let devices = parse_paired_devices_response(&paired_device_response(false)).unwrap();
    let address = devices[0].address;
    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        send_mock_response(&mut device, &readback, paired_device_response(false));
    });

    let result = set_paired_device(
        &mut client,
        &mut FrameDecoder::default(),
        &Ok(true),
        &Ok(devices),
        address,
        PairedDeviceAction::Connect,
        &mpsc::channel().0,
    );
    assert!(result.is_err());
    device_thread.join().unwrap();
}

#[test]
fn paired_device_remove_waits_for_ack_and_verifies_absence() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let listed_devices = paired_device_response(false);
    let devices = parse_paired_devices_response(&listed_devices).unwrap();
    let address = devices[0].address;
    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0429);
        assert_eq!(command.payload, PairedDeviceAction::Remove.payload(address));
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x0112);
        send_mock_response(&mut device, &readback, vec![0, 0]);
    });

    let (updates, _) = mpsc::channel();
    let result = set_paired_device(
        &mut client,
        &mut FrameDecoder::default(),
        &Ok(true),
        &Ok(devices),
        address,
        PairedDeviceAction::Remove,
        &updates,
    )
    .unwrap();
    assert!(result.1);
    assert!(result.0.is_empty());
    device_thread.join().unwrap();
}

#[test]
fn paired_device_remove_is_rejected_when_device_is_connected() {
    let (mut client, _device) = UnixStream::pair().unwrap();
    let listed_devices = paired_device_response(true);
    let devices = parse_paired_devices_response(&listed_devices).unwrap();
    let result = set_paired_device(
        &mut client,
        &mut FrameDecoder::default(),
        &Ok(true),
        &Ok(devices.clone()),
        devices[0].address,
        PairedDeviceAction::Remove,
        &mpsc::channel().0,
    );
    assert!(result.is_err());
}

#[test]
fn paired_device_remove_never_deletes_the_current_host() {
    let (mut client, _device) = UnixStream::pair().unwrap();
    let mut listed_devices = paired_device_response(false);
    listed_devices[10] |= 0x01;
    let devices = parse_paired_devices_response(&listed_devices).unwrap();
    let result = set_paired_device(
        &mut client,
        &mut FrameDecoder::default(),
        &Ok(true),
        &Ok(devices.clone()),
        devices[0].address,
        PairedDeviceAction::Remove,
        &mpsc::channel().0,
    );
    assert!(result.is_err());
}

fn paired_device_response(connected: bool) -> Vec<u8> {
    let mut response = vec![0, 1, 6, 5, 4, 3, 2, 1, 8, u8::from(connected), 0x08, 5];
    response.extend_from_slice(b"Phone");
    response
}

#[test]
fn noise_write_uses_ack_and_readback_for_all_profile_modes() {
    let modes = crate::compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    for (requested, actual) in [(5, 5), (7, 7), (8, 8), (11, 13), (12, 12), (14, 14)] {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let device_thread = thread::spawn(move || {
            let setting = read_mock_request(&mut device);
            assert_eq!(
                setting.payload,
                noise_mode_write_payload_for(requested, modes).unwrap()
            );
            send_mock_response(&mut device, &setting, vec![0]);

            let query = read_mock_request(&mut device);
            assert_eq!(query.command, 0x010c);
            let mut response_payload = vec![0, 1, 1];
            let byte_count = usize::from(actual / 8) + 1;
            response_payload.resize(3 + byte_count, 0);
            response_payload[3 + usize::from(actual / 8)] = 1 << (actual % 8);
            send_mock_response(&mut device, &query, response_payload);
        });

        let (updates, _) = mpsc::channel();
        let result = set_noise_mode(
            &mut client,
            &mut FrameDecoder::default(),
            &crate::compatibility::fallback_noise_mode_indices_for(modes),
            modes,
            &mut worn_state(),
            true,
            requested,
            &updates,
        );
        assert_eq!(result, Ok(actual), "requested index {requested}");
        device_thread.join().unwrap();
    }
}

#[test]
fn noise_write_preserves_failure_on_device_rejection_or_bad_readback() {
    for (ack, readback) in [(vec![0x0e], None), (vec![0], Some(vec![0, 1, 1, 0x40]))] {
        let (mut client, mut device) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        device
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let device_thread = thread::spawn(move || {
            let setting = read_mock_request(&mut device);
            send_mock_response(&mut device, &setting, ack);
            if let Some(readback_payload) = readback {
                let query = read_mock_request(&mut device);
                send_mock_response(&mut device, &query, readback_payload);
            }
        });

        let (updates, _) = mpsc::channel();
        let modes = crate::compatibility::profile_for_product_id(0x06ec10)
            .unwrap()
            .noise_modes;
        let result = set_noise_mode(
            &mut client,
            &mut FrameDecoder::default(),
            &[4],
            modes,
            &mut worn_state(),
            true,
            4,
            &updates,
        );
        assert!(result.is_err());
        device_thread.join().unwrap();
    }
}

#[test]
fn noise_write_does_not_send_modes_missing_from_api_list() {
    let (mut client, _device) = UnixStream::pair().unwrap();
    let (updates, _) = mpsc::channel();
    let modes = crate::compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    let result = set_noise_mode(
        &mut client,
        &mut FrameDecoder::default(),
        &[6],
        modes,
        &mut worn_state(),
        true,
        4,
        &updates,
    );
    assert!(result.is_err());
}

#[test]
fn noise_write_does_not_send_when_no_earbud_is_worn() {
    let (mut client, _device) = UnixStream::pair().unwrap();
    let (updates, _) = mpsc::channel();
    let modes = crate::compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    let result = set_noise_mode(
        &mut client,
        &mut FrameDecoder::default(),
        &[4],
        modes,
        &mut WearState::default(),
        true,
        4,
        &updates,
    );
    assert!(result.is_err());
}

#[test]
fn noise_write_allows_unworn_when_wear_detection_is_not_supported() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let device_thread = thread::spawn(move || {
        let setting = read_mock_request(&mut device);
        assert_eq!(setting.command, 0x0404);
        send_mock_response(&mut device, &setting, vec![0]);

        let query = read_mock_request(&mut device);
        assert_eq!(query.command, 0x010c);
        send_mock_response(&mut device, &query, vec![0, 1, 1, 0x10]);
    });

    let (updates, _) = mpsc::channel();
    let modes = crate::compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    let result = set_noise_mode(
        &mut client,
        &mut FrameDecoder::default(),
        &[4],
        modes,
        &mut WearState::default(),
        false,
        4,
        &updates,
    );
    assert_eq!(result, Ok(4));
    device_thread.join().unwrap();
}

#[test]
fn equalizer_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0406);
        assert_eq!(command.payload, vec![2]);
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x010f);
        send_mock_response(&mut device, &readback, vec![0, 2]);
    });

    let (updates, _) = mpsc::channel();
    let result = set_equalizer(&mut client, &mut FrameDecoder::default(), 2, &updates).unwrap();
    assert_eq!(result, 2);
    device_thread.join().unwrap();
}

#[test]
fn spatial_audio_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0422);
        assert_eq!(command.payload, vec![1]);
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x012a);
        send_mock_response(&mut device, &readback, vec![0, 1]);
    });

    let (updates, _) = mpsc::channel();
    let result = set_spatial_audio(&mut client, &mut FrameDecoder::default(), 1, &updates).unwrap();
    assert_eq!(result, 1);
    device_thread.join().unwrap();
}

#[test]
fn game_mode_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0403);
        assert_eq!(command.payload, vec![0x06, 1]);
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x010d);
        assert_eq!(readback.payload, vec![1, 0x06]);
        send_mock_response(&mut device, &readback, vec![0, 1, 0x06, 1]);
    });

    let (updates, _) = mpsc::channel();
    let result = set_game_mode(
        &mut client,
        &mut FrameDecoder::default(),
        0x06,
        true,
        &updates,
    )
    .unwrap();
    assert!(result);
    device_thread.join().unwrap();
}

#[test]
fn bass_engine_toggle_and_level_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        // Toggle write: 0x0403 [29, 1]
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0403);
        assert_eq!(command.payload, vec![29, 1]);
        send_mock_response(&mut device, &command, vec![0]);

        // Toggle readback: 0x010d [1, 29]
        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x010d);
        assert_eq!(readback.payload, vec![1, 29]);
        send_mock_response(&mut device, &readback, vec![0, 1, 29, 1]);

        // Level write: 0x041b [min, max, cur]
        // -5 as u8 is 0xFB, 5 is 0x05, 2 is 0x02
        let level_cmd = read_mock_request(&mut device);
        assert_eq!(level_cmd.command, 0x041b);
        assert_eq!(level_cmd.payload, vec![0xFB, 0x05, 0x02]);
        send_mock_response(&mut device, &level_cmd, vec![0]);

        // Level readback: 0x0124 []
        let level_rb = read_mock_request(&mut device);
        assert_eq!(level_rb.command, 0x0124);
        assert!(level_rb.payload.is_empty());
        send_mock_response(&mut device, &level_rb, vec![0, 0xFB, 0x05, 0x02]);
    });

    let (updates, _) = mpsc::channel();
    let toggle_result =
        set_bass_engine_enabled(&mut client, &mut FrameDecoder::default(), true, &updates).unwrap();
    assert!(toggle_result);

    let level_result = set_bass_engine_level(
        &mut client,
        &mut FrameDecoder::default(),
        -5,
        5,
        2,
        &updates,
    )
    .unwrap();
    assert_eq!(level_result.min, -5);
    assert_eq!(level_result.max, 5);
    assert_eq!(level_result.current, 2);

    device_thread.join().unwrap();
}

#[test]
fn hi_res_write_waits_for_ack_and_matching_readback() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        let command = read_mock_request(&mut device);
        assert_eq!(command.command, 0x0403);
        assert_eq!(command.payload, vec![24, 1]);
        send_mock_response(&mut device, &command, vec![0]);

        let readback = read_mock_request(&mut device);
        assert_eq!(readback.command, 0x010d);
        assert_eq!(readback.payload, vec![1, 24]);
        send_mock_response(&mut device, &readback, vec![0, 1, 24, 1]);
    });

    let (updates, _) = mpsc::channel();
    let result = set_hi_res(&mut client, &mut FrameDecoder::default(), true, &updates).unwrap();
    assert!(result);
    device_thread.join().unwrap();
}

fn worn_state() -> WearState {
    WearState {
        left: Some(crate::protocol::WearPart {
            out_of_case: true,
            in_ear: true,
            raw_flags: 3,
        }),
        ..WearState::default()
    }
}

fn read_mock_request(stream: &mut UnixStream) -> AppPacket {
    let mut decoder = FrameDecoder::default();
    let mut buffer = [0; 1024];
    loop {
        let length = stream.read(&mut buffer).unwrap();
        for frame in decoder.push(&buffer[..length]).unwrap() {
            if let Ok(packet) = AppPacket::decode(&frame) {
                return packet;
            }
        }
    }
}

fn send_mock_response(stream: &mut UnixStream, request: &AppPacket, payload: Vec<u8>) {
    let response = AppPacket {
        command: request.command | 0x8000,
        sequence: request.sequence,
        payload,
    };
    let frame = encode_frame(&response.encode().unwrap()).unwrap();
    stream.write_all(&frame).unwrap();
}

#[test]
fn sdp_uuid128_round_trips_through_libbluetooth() {
    use super::transport::{create_sdp_uuid128, format_sdp_uuid};

    for uuid_str in [
        "0000079a-d102-11e1-9b23-00025b00a5a5",
        "00001107-d102-11e1-9b23-00025b00a5a5",
    ] {
        let uuid = bluer::Uuid::parse_str(uuid_str).unwrap();
        let sdp_uuid = create_sdp_uuid128(uuid).expect("sdp_uuid128_create should succeed");
        let formatted = format_sdp_uuid(&sdp_uuid).expect("sdp_uuid2strn should format");
        assert_eq!(
            formatted.to_ascii_lowercase(),
            uuid_str.to_ascii_lowercase()
        );
    }
}

#[test]
fn candidate_service_uuids_prioritizes_device_uuids() {
    use super::brands::oppo::oppo_candidate_service_uuids;

    let device_uuid = bluer::Uuid::parse_str("0000079a-d102-11e1-9b23-00025b00a5a5").unwrap();
    let candidates = oppo_candidate_service_uuids(&[device_uuid]);
    assert_eq!(candidates.first(), Some(&device_uuid));
    assert!(candidates.len() >= 2);
}

#[test]
fn prioritize_rfcomm_channels_handles_t200_lite_and_fallbacks() {
    use super::transport::prioritize_rfcomm_channels;

    let primary_names = &["elnk", "enco link", "encolink"];
    let secondary_names = &["jl_spp"];

    // 1. realme Buds T200 Lite: 128-bit UUID on channel 11 + two JL_SPP records on channel 1
    let spp_t200 = vec![
        (Some("JL_SPP".to_string()), 1),
        (Some("JL_SPP".to_string()), 1),
    ];
    let channels =
        prioritize_rfcomm_channels(&[11], &spp_t200, primary_names, secondary_names, Some(15))
            .unwrap();
    assert_eq!(channels, vec![11, 1]);

    // 2. realme Buds T200 Lite when SDP UUID channel query is empty: picks JL_SPP on channel 1
    let channels_no_uuid =
        prioritize_rfcomm_channels(&[], &spp_t200, primary_names, secondary_names, Some(15))
            .unwrap();
    assert_eq!(channels_no_uuid, vec![1]);

    // 3. Standard OPPO/OnePlus: ELNK on channel 15 takes precedence over JL_SPP
    let mixed_spp = vec![
        (Some("JL_SPP".to_string()), 1),
        (Some("ELNK".to_string()), 15),
    ];
    let channels_elnk =
        prioritize_rfcomm_channels(&[], &mixed_spp, primary_names, secondary_names, Some(15))
            .unwrap();
    assert_eq!(channels_elnk, vec![15]);

    // 4. Conflicting ELNK channels without UUID return error
    let conflicting_elnk = vec![
        (Some("ELNK".to_string()), 15),
        (Some("ELNK".to_string()), 16),
    ];
    assert!(
        prioritize_rfcomm_channels(
            &[],
            &conflicting_elnk,
            primary_names,
            secondary_names,
            Some(15)
        )
        .is_err()
    );

    // 5. Unambiguous single SPP channel fallback when no names match
    let unknown_spp = vec![(Some("SerialPort".to_string()), 5)];
    let channels_unknown =
        prioritize_rfcomm_channels(&[], &unknown_spp, primary_names, secondary_names, Some(15))
            .unwrap();
    assert_eq!(channels_unknown, vec![5]);

    // 6. Fallback channel 15 when no SDP records exist
    let channels_empty =
        prioritize_rfcomm_channels(&[], &[], primary_names, secondary_names, Some(15)).unwrap();
    assert_eq!(channels_empty, vec![15]);
}

#[test]
fn probe_read_only_features_populates_detected_list() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        let events = read_mock_request(&mut device);
        assert_eq!(events.command, 0x0200);
        send_mock_response(&mut device, &events, vec![0, 1, 1]);

        let subscribe = read_mock_request(&mut device);
        assert_eq!(subscribe.command, 0x0205);
        send_mock_response(&mut device, &subscribe, vec![0]);

        let battery = read_mock_request(&mut device);
        assert_eq!(battery.command, 0x0106);
        send_mock_response(&mut device, &battery, vec![0, 3, 1, 80, 2, 80, 3, 0]);

        let fw = read_mock_request(&mut device);
        assert_eq!(fw.command, 0x0105);
        let mut fw_payload = vec![0, 3];
        fw_payload.extend_from_slice(b"1,1,1.1.0,2,1,1.1.0,3,1,019");
        send_mock_response(&mut device, &fw, fw_payload);

        let wear = read_mock_request(&mut device);
        assert_eq!(wear.command, 0x0109);
        send_mock_response(&mut device, &wear, vec![0, 1, 1, 3]);
    });

    let (battery, firmware, wear, detected_features, probe_errors) =
        super::session::probe_read_only_features(&mut client, &mut FrameDecoder::default(), None);

    device_thread.join().unwrap();

    assert!(battery.left.is_some());
    assert!(battery.right.is_some());
    assert!(battery.case.is_none()); // 0x00 case battery treated as None
    assert!(firmware.left.is_some());
    assert!(wear.left.is_some());
    assert!(probe_errors.is_empty());
    assert_eq!(detected_features.len(), 5);
    for feature in &detected_features {
        assert!(!feature.is_empty());
    }
}

#[test]
fn device_originated_requests_are_answered_during_queries() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        // Client sends an initial request, e.g. 0x0106 battery
        let initial_query = read_mock_request(&mut device);
        assert_eq!(initial_query.command, 0x0106);

        // Before answering the query, device sends unsolicited TimeRequest (0x0500)
        let time_req = AppPacket {
            command: 0x0500,
            sequence: 0x12,
            payload: vec![],
        };
        let frame = encode_frame(&time_req.encode().unwrap()).unwrap();
        device.write_all(&frame).unwrap();

        // Host client must reply 0x8500 with unix timestamp
        let time_reply = read_mock_request(&mut device);
        assert_eq!(time_reply.command, 0x8500);
        assert_eq!(time_reply.sequence, 0x12);
        assert_eq!(time_reply.payload.len(), 5);
        assert_eq!(time_reply.payload[0], 0x00);

        // Device sends AppCapabilitiesRequest (0x051C)
        let caps_req = AppPacket {
            command: 0x051C,
            sequence: 0x13,
            payload: vec![],
        };
        let frame = encode_frame(&caps_req.encode().unwrap()).unwrap();
        device.write_all(&frame).unwrap();

        // Host client must reply 0x851C [0, 0, 0]
        let caps_reply = read_mock_request(&mut device);
        assert_eq!(caps_reply.command, 0x851C);
        assert_eq!(caps_reply.sequence, 0x13);
        assert_eq!(caps_reply.payload, vec![0x00, 0x00, 0x00]);

        // Device sends PhoneSpatialRequest (0x051D)
        let spatial_req = AppPacket {
            command: 0x051D,
            sequence: 0x14,
            payload: vec![],
        };
        let frame = encode_frame(&spatial_req.encode().unwrap()).unwrap();
        device.write_all(&frame).unwrap();

        let spatial_reply = read_mock_request(&mut device);
        assert_eq!(spatial_reply.command, 0x851D);
        assert_eq!(spatial_reply.sequence, 0x14);
        assert_eq!(spatial_reply.payload, vec![0x01]);

        // Device sends ConnectionHistory event (0x0204 [0xF2])
        let history_event = AppPacket {
            command: 0x0204,
            sequence: 0x15,
            payload: vec![0xF2, 0x01, 0x02],
        };
        let frame = encode_frame(&history_event.encode().unwrap()).unwrap();
        device.write_all(&frame).unwrap();

        let history_reply = read_mock_request(&mut device);
        assert_eq!(history_reply.command, 0x8204);
        assert_eq!(history_reply.sequence, 0x15);
        assert_eq!(history_reply.payload, vec![0x00, 0xF2]);

        // Device sends Telemetry event (0x0204 [0xF4, ...])
        let telemetry_event = AppPacket {
            command: 0x0204,
            sequence: 0x16,
            payload: [&[0xF4][..], br#"{"cmd":"device_heartbeat","status":"ok"}"#].concat(),
        };
        let frame = encode_frame(&telemetry_event.encode().unwrap()).unwrap();
        device.write_all(&frame).unwrap();

        let telemetry_reply = read_mock_request(&mut device);
        assert_eq!(telemetry_reply.command, 0x8204);
        assert_eq!(telemetry_reply.sequence, 0x16);
        assert_eq!(telemetry_reply.payload[0], 0x00);
        assert_eq!(telemetry_reply.payload[1], 0xF4);
        let reply_json: serde_json::Value =
            serde_json::from_slice(&telemetry_reply.payload[2..]).unwrap();
        assert_eq!(reply_json["cmd"], "device_heartbeat");

        // Now device answers the original query
        send_mock_response(&mut device, &initial_query, vec![0, 0]);
    });

    let (updates, _) = mpsc::channel();
    let res = super::brands::oppo::oppo_session::request(
        &mut client,
        &mut FrameDecoder::default(),
        0x0106,
        0,
        vec![],
        Some(&updates),
    );
    assert!(res.is_ok());
    device_thread.join().unwrap();
}

#[test]
fn write_readback_settle_delay_retries_on_slow_firmware() {
    let (mut client, mut device) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    device
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    let device_thread = thread::spawn(move || {
        // Setting game mode
        let write_cmd = read_mock_request(&mut device);
        assert_eq!(write_cmd.command, 0x0403);
        assert_eq!(write_cmd.payload, vec![6, 1]);
        send_mock_response(&mut device, &write_cmd, vec![0]);

        // First readback immediately after write: returns old state 0 (not settled yet)
        let readback1 = read_mock_request(&mut device);
        assert_eq!(readback1.command, 0x010d);
        assert_eq!(readback1.payload, vec![1, 6]);
        send_mock_response(&mut device, &readback1, vec![0, 1, 6, 0]);

        // Second readback after settle delay (250ms): returns new state 1 (settled)
        let readback2 = read_mock_request(&mut device);
        assert_eq!(readback2.command, 0x010d);
        assert_eq!(readback2.payload, vec![1, 6]);
        send_mock_response(&mut device, &readback2, vec![0, 1, 6, 1]);
    });

    let (updates, _) = mpsc::channel();
    let res =
        super::session::set_game_mode(&mut client, &mut FrameDecoder::default(), 6, true, &updates);
    assert_eq!(res, Ok(true));
    device_thread.join().unwrap();
}
