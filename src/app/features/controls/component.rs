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

use gtk::prelude::*;

use crate::{
    app::state::ControlsState,
    compatibility::GestureRow,
    protocol::{
        EarSide, FitTestResult, KeyFunction, NoiseCycleState, PairedDevice, PairedDeviceAddress,
    },
    ui::{
        ControlsWidgets,
        controls_state::{self, ControlsFeaturesViewState},
    },
};

#[derive(Clone)]
pub(crate) struct ControlsComponent {
    pub widgets: ControlsWidgets,
}

impl ControlsComponent {
    pub fn new(widgets: ControlsWidgets) -> Self {
        Self { widgets }
    }

    pub fn render_features(
        &self,
        state: &ControlsState,
        supports_dual_device: bool,
        supports_wear_detection: bool,
    ) {
        controls_state::update_controls_features(
            &self.widgets,
            &ControlsFeaturesViewState {
                supports_touch: state.supports_touch_customization,
                supports_dual: supports_dual_device,
                supports_wear: supports_wear_detection,
                supports_sleep: state.supports_sleep_detection,
                supports_head: state.supports_head_motion,
                supports_find: state.supports_find_device,
                supports_fit: state.supports_fit_detection,
                supports_volume: state.supports_prompt_volume,
                gesture_rows: &state.gesture_rows,
            },
        );
    }

    pub fn render_key_functions(
        &self,
        functions: &[KeyFunction],
        side: EarSide,
        rows: &[GestureRow],
    ) {
        controls_state::update_key_functions(&self.widgets, functions, side, rows);
    }

    pub fn render_noise_cycle(&self, cycle: Option<NoiseCycleState>, side: EarSide) {
        if let Some(cycle) = cycle {
            controls_state::update_noise_cycle(&self.widgets, cycle, side);
        }
    }

    pub fn render_prompt_volume(&self, volume: Option<u8>) {
        if let Some(vol) = volume {
            controls_state::update_prompt_volume(&self.widgets, vol);
        }
    }

    pub fn render_head_motion(&self, motion_type: Option<u8>) {
        if let Some(m) = motion_type {
            controls_state::update_head_switch(&self.widgets, m > 0);
            controls_state::update_head_motion(&self.widgets, m);
        }
    }

    pub fn render_dual_device(&self, enabled: Option<bool>) {
        if let Some(en) = enabled {
            controls_state::update_dual_device(&self.widgets, en);
        }
    }

    pub fn render_in_ear(&self, enabled: Option<bool>) {
        if let Some(en) = enabled {
            controls_state::update_in_ear_detection(&self.widgets, en);
        }
    }

    pub fn render_sleep(&self, enabled: Option<bool>) {
        if let Some(en) = enabled {
            controls_state::update_sleep_detection(&self.widgets, en);
        }
    }

    pub fn render_preferred_devices(
        &self,
        preferred: Option<PairedDeviceAddress>,
        devices: &[PairedDevice],
    ) {
        controls_state::update_preferred_devices(&self.widgets, preferred, devices);
    }

    pub fn render_finding_earbuds(&self, finding: bool) {
        controls_state::update_finding_earbuds(&self.widgets, finding);
    }

    pub fn render_fit_test_running(&self, running: bool) {
        controls_state::update_fit_test_running(&self.widgets, running);
    }

    pub fn render_fit_test_result(&self, result: FitTestResult) {
        controls_state::update_fit_test_result(&self.widgets, result);
    }

    pub fn reset(&self) {
        controls_state::reset_controls_page(&self.widgets);
    }

    pub fn re_enable_controls(&self) {
        for (_, combo) in &self.widgets.gesture_combos {
            combo.set_sensitive(true);
        }
        self.widgets.cycle_check_anc.set_sensitive(true);
        self.widgets.cycle_check_transparency.set_sensitive(true);
        self.widgets.cycle_check_off.set_sensitive(true);
        self.widgets.find_btn.set_sensitive(true);
        self.widgets.fit_btn.set_sensitive(true);
        self.widgets.dual_switch.set_sensitive(true);
        self.widgets.dual_row.set_sensitive(true);
        self.widgets.in_ear_switch.set_sensitive(true);
        self.widgets.in_ear_row.set_sensitive(true);
        self.widgets.sleep_switch.set_sensitive(true);
        self.widgets.sleep_row.set_sensitive(true);
        self.widgets.head_switch.set_sensitive(true);
        self.widgets.head_switch_row.set_sensitive(true);
        self.widgets.head_motion_row.set_sensitive(true);
        self.widgets.preferred_row.set_sensitive(true);
        self.widgets.prompt_volume_scale.set_sensitive(true);
    }
}
