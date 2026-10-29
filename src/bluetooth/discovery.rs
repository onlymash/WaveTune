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

use std::{sync::mpsc::Sender, thread};

use bluer::{Session, Uuid};
use tokio::runtime::Builder;

use super::{DeviceBrand, DeviceChoice, UiUpdate};

pub(super) const CONTROL_SERVICE_CANDIDATES: &[&str] =
    crate::protocol::brands::oppo::OPPO_CONTROL_SERVICE_CANDIDATES;
pub(super) const ONEPLUS_COMPANY_ID: u16 = crate::protocol::brands::oppo::ONEPLUS_COMPANY_ID;
pub(super) const OPPO_COMPANY_ID: u16 = crate::protocol::brands::oppo::OPPO_COMPANY_ID;
pub(super) const REALME_COMPANY_ID: u16 = crate::protocol::brands::oppo::REALME_COMPANY_ID;

pub fn spawn_discovery(updates: Sender<UiUpdate>) -> std::io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("enco-discovery".to_string())
        .spawn(move || {
            let result = find_devices();
            let _ = updates.send(UiUpdate::Devices(result));
        })
}

pub(super) fn find_devices() -> Result<Vec<DeviceChoice>, String> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Unable to start BlueZ query runtime: {error}"))?;
    runtime.block_on(find_device_async())
}

async fn find_device_async() -> Result<Vec<DeviceChoice>, String> {
    let session = Session::new()
        .await
        .map_err(|error| format!("Unable to connect to BlueZ D-Bus: {error}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|error| format!("No Bluetooth adapter available: {error}"))?;
    let service_uuids: Vec<Uuid> = CONTROL_SERVICE_CANDIDATES
        .iter()
        .map(|candidate| {
            Uuid::parse_str(candidate)
                .map_err(|error| format!("Internal service UUID invalid: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut candidates = Vec::new();
    for address in adapter
        .device_addresses()
        .await
        .map_err(|error| format!("Unable to enumerate paired Bluetooth devices: {error}"))?
    {
        let device = adapter
            .device(address)
            .map_err(|error| format!("Unable to read Bluetooth device properties: {error}"))?;
        if !device
            .is_connected()
            .await
            .map_err(|error| format!("Unable to read device connection state: {error}"))?
        {
            continue;
        }

        let icon = device
            .icon()
            .await
            .map_err(|error| format!("Unable to read device type: {error}"))?;
        let class = device
            .class()
            .await
            .map_err(|error| format!("Unable to read device class: {error}"))?;
        if !is_headset(icon.as_deref(), class) {
            continue;
        }

        let manufacturer_data = device
            .manufacturer_data()
            .await
            .map_err(|error| format!("Unable to read device manufacturer data: {error}"))?
            .unwrap_or_default();
        let uuids = device
            .uuids()
            .await
            .map_err(|error| format!("Unable to read device service UUID: {error}"))?
            .unwrap_or_default();
        let control_service_uuids: Vec<Uuid> = service_uuids
            .iter()
            .copied()
            .filter(|candidate| uuids.contains(candidate))
            .collect();
        let has_control_service = !control_service_uuids.is_empty();
        let Some(brand) = identify_brand(&manufacturer_data, has_control_service) else {
            continue;
        };
        let alias = device
            .alias()
            .await
            .map_err(|error| format!("Unable to read Bluetooth device name: {error}"))?;
        candidates.push(DeviceChoice {
            address,
            name: alias,
            brand,
            has_control_service,
            control_service_uuids,
        });
    }
    candidates.sort_by(|left, right| {
        left.brand
            .label()
            .cmp(right.brand.label())
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(candidates)
}

pub(super) fn is_headset(icon: Option<&str>, class: Option<u32>) -> bool {
    let icon_matches = matches!(icon, Some("audio-headset" | "audio-headphones"));
    let class_matches = class.is_some_and(|class| {
        let major = (class >> 8) & 0x1f;
        let minor = (class >> 2) & 0x3f;
        major == 0x04 && matches!(minor, 0x01 | 0x02 | 0x06)
    });
    icon_matches || class_matches
}

pub(super) fn identify_brand(
    manufacturer_data: &std::collections::HashMap<u16, Vec<u8>>,
    has_control_service: bool,
) -> Option<DeviceBrand> {
    if manufacturer_data.contains_key(&ONEPLUS_COMPANY_ID) {
        Some(DeviceBrand::OnePlus)
    } else if manufacturer_data.contains_key(&REALME_COMPANY_ID) {
        Some(DeviceBrand::Realme)
    } else if manufacturer_data.contains_key(&OPPO_COMPANY_ID) || has_control_service {
        Some(DeviceBrand::Oppo)
    } else {
        None
    }
}
