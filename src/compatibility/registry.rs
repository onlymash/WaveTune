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

//! Multi-brand device profile registry, inspired by EncoDeck's BuiltInProfiles.

use super::{
    DeviceProfile, brands::earfun::EARFUN_DEVICE_PROFILES, brands::oppo::OPPO_DEVICE_PROFILES,
};

/// Returns an iterator over all built-in device profiles across all supported brands.
#[allow(dead_code)]
pub fn builtin_profiles() -> impl Iterator<Item = &'static DeviceProfile> {
    OPPO_DEVICE_PROFILES
        .iter()
        .chain(EARFUN_DEVICE_PROFILES.iter())
}
