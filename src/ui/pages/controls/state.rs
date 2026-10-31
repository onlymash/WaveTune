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

use super::ControlsWidgets;
use crate::{
    compatibility::GestureRow,
    protocol::{
        EarSide, FitTestResult, KeyFunction, NoiseCycleState, PairedDevice, PairedDeviceAddress,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct ControlsFeaturesViewState<'a> {
    pub supports_touch: bool,
    pub supports_dual: bool,
    pub supports_wear: bool,
    pub supports_sleep: bool,
    pub supports_head: bool,
    pub supports_find: bool,
    pub supports_fit: bool,
    pub supports_volume: bool,
    pub gesture_rows: &'a [GestureRow],
}

pub(crate) fn update_controls_features(
    widgets: &ControlsWidgets,
    state: &ControlsFeaturesViewState,
) {
    widgets
        .gestures
        .set_supported(state.supports_touch, state.gesture_rows);

    widgets.features.set_supported(
        state.supports_dual,
        state.supports_wear,
        state.supports_sleep,
        state.supports_volume,
    );

    widgets.head_motion.set_supported(state.supports_head);

    widgets
        .utilities
        .set_supported(state.supports_find, state.supports_fit);
}

pub(crate) fn update_key_functions(
    widgets: &ControlsWidgets,
    key_functions: &[KeyFunction],
    selected_ear: EarSide,
    gesture_rows: &[GestureRow],
) {
    widgets
        .gestures
        .update_key_functions(key_functions, selected_ear, gesture_rows);
}

pub(crate) fn update_noise_cycle(
    widgets: &ControlsWidgets,
    cycle: NoiseCycleState,
    selected_ear: EarSide,
) {
    widgets.gestures.update_noise_cycle(cycle, selected_ear);
}

pub(crate) fn update_finding_earbuds(widgets: &ControlsWidgets, finding: bool) {
    widgets.utilities.set_finding(finding);
}

pub(crate) fn update_fit_test_running(widgets: &ControlsWidgets, running: bool) {
    widgets.utilities.set_fit_running(running);
}

pub(crate) fn update_fit_test_result(widgets: &ControlsWidgets, result: FitTestResult) {
    widgets.utilities.set_fit_result(result);
}

pub(crate) fn update_prompt_volume(widgets: &ControlsWidgets, volume: u8) {
    widgets.features.set_prompt_volume(volume);
}

pub(crate) fn update_head_motion(widgets: &ControlsWidgets, motion_type: u8) {
    widgets.head_motion.set_motion_type(motion_type);
}

pub(crate) fn update_preferred_devices(
    widgets: &ControlsWidgets,
    preferred: Option<PairedDeviceAddress>,
    devices: &[PairedDevice],
) {
    widgets.features.set_preferred_devices(preferred, devices);
}

pub(crate) fn update_dual_device(widgets: &ControlsWidgets, enabled: bool) {
    widgets.features.set_dual_device(enabled);
}

pub(crate) fn update_in_ear_detection(widgets: &ControlsWidgets, enabled: bool) {
    widgets.features.set_in_ear_detection(enabled);
}

pub(crate) fn update_sleep_detection(widgets: &ControlsWidgets, enabled: bool) {
    widgets.features.set_sleep_detection(enabled);
}

pub(crate) fn update_head_switch(widgets: &ControlsWidgets, enabled: bool) {
    widgets.head_motion.set_switch_state(enabled);
}

pub(crate) fn reset_controls_page(widgets: &ControlsWidgets) {
    widgets.gestures.reset();
    widgets.features.reset();
    widgets.head_motion.reset();
    widgets.utilities.reset();
}
