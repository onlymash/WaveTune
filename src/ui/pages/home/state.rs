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

pub(crate) use crate::ui::pages::home::noise::{noise_mode_icon_name, noise_options_title_for};

use crate::{
    compatibility,
    protocol::{BatteryState, WearPart, WearState},
};

#[cfg(test)]
use crate::{
    compatibility::EqualizerPreset,
    protocol::CustomEqualizer,
    ui::pages::home::{HomeWidgets, noise::NoiseControlsViewState},
};

#[cfg(test)]
pub(crate) fn update_noise_controls(widgets: &HomeWidgets, state: &NoiseControlsViewState) {
    widgets.noise.render(state);
}

#[cfg(test)]
pub(crate) fn update_equalizer_items(
    widgets: &HomeWidgets,
    supports_equalizer: bool,
    presets: &[EqualizerPreset],
    custom_equalizers: &[CustomEqualizer],
    selected_id: Option<u8>,
) {
    widgets.sound.update_equalizer_items(
        supports_equalizer,
        presets,
        custom_equalizers,
        selected_id,
    );
}

pub(crate) fn merge_battery(current: &mut BatteryState, update: BatteryState) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case.is_some() {
        current.case = update.case;
    }
}

pub(crate) fn merge_wear(current: &mut WearState, update: WearState) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case_open.is_some() {
        current.case_open = update.case_open;
    }
}

pub(crate) fn wear_part_name(name: &str, part: Option<WearPart>) -> String {
    match part {
        Some(part) if part.in_ear => crate::t!("wear-removed-in-ear", name),
        Some(part) if part.out_of_case => crate::t!("wear-removed-out-of-ear", name),
        Some(_) => crate::t!("wear-in-case-status", name),
        None => crate::t!("wear-status-unknown", name),
    }
}

#[allow(dead_code)]
pub(crate) fn noise_mode_name(index: u8, modes: &[compatibility::NoiseMode]) -> String {
    compatibility::noise_mode_for(modes, index)
        .map(|mode| crate::t!(mode.kind.as_str()))
        .unwrap_or_else(|| crate::t!("noise-mode-protocol-mode", index))
}
