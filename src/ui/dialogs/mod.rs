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

pub mod about;
pub mod custom_eq;
pub mod paired_device;
pub mod preferences;

#[allow(unused_imports)]
pub use about::show_about_dialog;
#[allow(unused_imports)]
pub use custom_eq::show_custom_eq_dialog;
#[allow(unused_imports)]
pub use paired_device::{
    paired_device_icon_name, paired_device_type_name, show_paired_device_dialog,
};
#[allow(unused_imports)]
pub use preferences::show_preferences_dialog;
