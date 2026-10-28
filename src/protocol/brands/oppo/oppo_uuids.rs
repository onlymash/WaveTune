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

//! OPPO/OnePlus/realme Bluetooth constants and identifiers.

pub const OPPO_PRIMARY_CONTROL_SERVICE_UUID: &str = "00001107-d102-11e1-9b23-00025b00a5a5";
pub const OPPO_SECONDARY_CONTROL_SERVICE_UUID: &str = "0000079a-d102-11e1-9b23-00025b00a5a5";
pub const STANDARD_SPP_SERVICE_UUID: &str = "00001101-0000-1000-8000-00805f9b34fb";

/// Default / fallback control service UUID used by legacy references.
pub const OPPO_CONTROL_SERVICE_UUID: &str = OPPO_SECONDARY_CONTROL_SERVICE_UUID;

/// Candidate RFCOMM service UUIDs for the OPPO/OnePlus/realme control channel.
pub const OPPO_CONTROL_SERVICE_CANDIDATES: &[&str] = &[
    OPPO_PRIMARY_CONTROL_SERVICE_UUID,
    OPPO_SECONDARY_CONTROL_SERVICE_UUID,
];

pub fn is_candidate_control_service_uuid(uuid: &str) -> bool {
    OPPO_CONTROL_SERVICE_CANDIDATES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(uuid))
        || STANDARD_SPP_SERVICE_UUID.eq_ignore_ascii_case(uuid)
}

pub const ONEPLUS_COMPANY_ID: u16 = 1839;
pub const OPPO_COMPANY_ID: u16 = 1946;
pub const REALME_COMPANY_ID: u16 = 0x08a4;
