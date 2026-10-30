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

use adw::prelude::*;

use super::ControlsWidgets;
use crate::{
    compatibility::GestureRow,
    protocol::{
        EarSide, FitQuality, FitTestResult, KeyFunction, NoiseCycleState, PairedDevice,
        PairedDeviceAddress,
    },
    ui::components::ComboRowExtManual,
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
        .gesture_group
        .set_visible(state.supports_touch && !state.gesture_rows.is_empty());
    widgets
        .call_gesture_group
        .set_visible(state.supports_touch && state.gesture_rows.iter().any(|r| r.is_call_control));
    widgets.ear_selector.set_visible(state.supports_touch);

    widgets.features_group.set_visible(
        state.supports_dual || state.supports_wear || state.supports_sleep || state.supports_volume,
    );
    widgets.dual_row.set_visible(state.supports_dual);
    widgets.dual_switch.set_sensitive(state.supports_dual);
    widgets.dual_row.set_sensitive(state.supports_dual);

    widgets.preferred_row.set_visible(state.supports_dual);
    widgets.preferred_row.set_sensitive(state.supports_dual);

    widgets.in_ear_row.set_visible(state.supports_wear);
    widgets.in_ear_switch.set_sensitive(state.supports_wear);
    widgets.in_ear_row.set_sensitive(state.supports_wear);

    widgets.sleep_row.set_visible(state.supports_sleep);
    widgets.sleep_switch.set_sensitive(state.supports_sleep);
    widgets.sleep_row.set_sensitive(state.supports_sleep);

    widgets.prompt_volume_row.set_visible(state.supports_volume);

    widgets.head_group.set_visible(state.supports_head);
    widgets.head_switch.set_sensitive(state.supports_head);
    widgets.head_switch_row.set_sensitive(state.supports_head);

    widgets
        .utility_group
        .set_visible(state.supports_find || state.supports_fit);
    widgets.find_card.set_visible(state.supports_find);
    widgets.fit_card.set_visible(state.supports_fit);
}

pub(crate) fn update_key_functions(
    widgets: &ControlsWidgets,
    key_functions: &[KeyFunction],
    selected_ear: EarSide,
    gesture_rows: &[GestureRow],
) {
    let _guard = widgets.gesture_combos_flag.enter();
    for (row_def, combo) in &widgets.gesture_combos {
        let current_func = key_functions
            .iter()
            .find(|k| {
                (k.side == selected_ear || k.side == EarSide::Both)
                    && k.button == row_def.button
                    && k.action == row_def.action
            })
            .map(|k| k.function)
            .or(row_def.default_function);

        if let Some(func) = current_func {
            combo.set_selected_id(Some(func as u8));
        }
    }

    // Check if long-press is currently noise control
    let hold_controls_noise = gesture_rows
        .iter()
        .find(|r| r.edits_noise_cycle)
        .map(|r| {
            let func = key_functions
                .iter()
                .find(|k| {
                    (k.side == selected_ear || k.side == EarSide::Both)
                        && k.button == r.button
                        && k.action == r.action
                })
                .map(|k| k.function)
                .or(r.default_function);
            func == Some(8) // 8 = Noise control
        })
        .unwrap_or(false);

    widgets.cycle_section.set_visible(hold_controls_noise);
}

pub(crate) fn update_noise_cycle(
    widgets: &ControlsWidgets,
    cycle: NoiseCycleState,
    selected_ear: EarSide,
) {
    let _guard = widgets.noise_cycle_flag.enter();
    let mask = if selected_ear == EarSide::Right {
        cycle.right_mask
    } else {
        cycle.left_mask
    };

    // Bit 1: Noise Cancellation, Bit 2: Transparency, Bit 0: Off
    widgets.cycle_check_anc.set_active((mask & (1 << 1)) != 0);
    widgets
        .cycle_check_transparency
        .set_active((mask & (1 << 2)) != 0);
    widgets.cycle_check_off.set_active((mask & (1 << 0)) != 0);
}

pub(crate) fn update_finding_earbuds(widgets: &ControlsWidgets, finding: bool) {
    if finding {
        widgets.find_btn.set_label(&crate::t!("controls-find-stop"));
        widgets.find_btn.add_css_class("destructive-action");
        widgets
            .find_status_label
            .set_text(&crate::t!("controls-find-playing-note"));
        widgets.find_status_label.set_visible(true);
    } else {
        widgets.find_btn.set_label(&crate::t!("controls-find-play"));
        widgets.find_btn.remove_css_class("destructive-action");
        widgets.find_status_label.set_visible(false);
    }
    widgets.find_btn.set_sensitive(true);
}

pub(crate) fn update_fit_test_running(widgets: &ControlsWidgets, running: bool) {
    if running {
        widgets.fit_btn.set_sensitive(false);
        widgets.fit_spinner.set_spinning(true);
        widgets.fit_spinner.set_visible(true);
        widgets
            .fit_result_label
            .set_text(&crate::t!("controls-fit-test-running"));
    } else {
        widgets.fit_btn.set_sensitive(true);
        widgets.fit_spinner.set_spinning(false);
        widgets.fit_spinner.set_visible(false);
    }
}

pub(crate) fn update_fit_test_result(widgets: &ControlsWidgets, result: FitTestResult) {
    let describe_quality = |q: FitQuality| match q {
        FitQuality::Good => crate::t!("controls-fit-test-good"),
        FitQuality::Average => crate::t!("controls-fit-test-average"),
        FitQuality::Poor => crate::t!("controls-fit-test-poor"),
        _ => crate::t!("controls-fit-test-failed"),
    };

    let text = if result.left == FitQuality::Good && result.right == FitQuality::Good {
        crate::t!("controls-fit-test-good")
    } else if result.left == FitQuality::Failed || result.right == FitQuality::Failed {
        crate::t!("controls-fit-test-failed")
    } else {
        format!(
            "{}: {} · {}: {}",
            crate::t!("controls-ear-left"),
            describe_quality(result.left),
            crate::t!("controls-ear-right"),
            describe_quality(result.right)
        )
    };
    widgets.fit_result_label.set_text(&text);
}

pub(crate) fn update_prompt_volume(widgets: &ControlsWidgets, volume: u8) {
    let _guard = widgets.prompt_volume_flag.enter();
    widgets.prompt_volume_scale.set_value(volume as f64);
    widgets.prompt_volume_label.set_text(&volume.to_string());
    widgets.prompt_volume_scale.set_sensitive(true);
}

pub(crate) fn update_head_motion(widgets: &ControlsWidgets, motion_type: u8) {
    let _guard = widgets.head_motion_flag.enter();
    widgets.head_motion_row.set_selected_id(Some(motion_type));
    widgets.head_motion_row.set_sensitive(true);
}

pub(crate) fn update_preferred_devices(
    widgets: &ControlsWidgets,
    preferred: Option<PairedDeviceAddress>,
    devices: &[PairedDevice],
) {
    let _guard = widgets.preferred_flag.enter();
    let mut items: Vec<(u8, String)> = vec![(0, crate::t!("controls-preferred-device-auto"))];
    let mut selected_id = 0u8;

    for (i, dev) in devices.iter().enumerate() {
        let id = (i + 1) as u8;
        let label = if dev.name.is_empty() {
            dev.address.format_mac()
        } else {
            dev.name.clone()
        };
        items.push((id, label));
        if let Some(pref) = preferred {
            if dev.address == pref {
                selected_id = id;
            }
        }
    }

    widgets.preferred_row.set_items(&items);
    widgets.preferred_row.set_selected_id(Some(selected_id));
    widgets.preferred_row.set_sensitive(true);
}

pub(crate) fn update_dual_device(widgets: &ControlsWidgets, enabled: bool) {
    let _guard = widgets.dual_device_flag.enter();
    widgets.dual_switch.set_state(enabled);
    widgets.dual_switch.set_active(enabled);
    widgets.dual_switch.set_sensitive(true);
    widgets.dual_row.set_sensitive(true);
    widgets.preferred_row.set_sensitive(enabled);
}

pub(crate) fn update_in_ear_detection(widgets: &ControlsWidgets, enabled: bool) {
    let _guard = widgets.in_ear_flag.enter();
    widgets.in_ear_switch.set_state(enabled);
    widgets.in_ear_switch.set_active(enabled);
    widgets.in_ear_switch.set_sensitive(true);
    widgets.in_ear_row.set_sensitive(true);
}

pub(crate) fn update_sleep_detection(widgets: &ControlsWidgets, enabled: bool) {
    let _guard = widgets.sleep_flag.enter();
    widgets.sleep_switch.set_state(enabled);
    widgets.sleep_switch.set_active(enabled);
    widgets.sleep_switch.set_sensitive(true);
    widgets.sleep_row.set_sensitive(true);
}

pub(crate) fn update_head_switch(widgets: &ControlsWidgets, enabled: bool) {
    let _guard = widgets.head_motion_flag.enter();
    widgets.head_switch.set_state(enabled);
    widgets.head_switch.set_active(enabled);
    widgets.head_switch.set_sensitive(true);
    widgets.head_switch_row.set_sensitive(true);
    widgets.head_motion_row.set_sensitive(enabled);
}

pub(crate) fn reset_controls_page(widgets: &ControlsWidgets) {
    widgets.gesture_group.set_visible(false);
    widgets.call_gesture_group.set_visible(false);
    widgets.ear_selector.set_visible(false);
    widgets.features_group.set_visible(false);
    widgets.head_group.set_visible(false);
    widgets.utility_group.set_visible(false);
    widgets.cycle_section.set_visible(false);
    widgets.find_status_label.set_visible(false);
    widgets.fit_spinner.set_visible(false);
    widgets.fit_spinner.set_spinning(false);
    widgets.fit_result_label.set_text("");

    widgets.dual_switch.set_state(false);
    widgets.dual_switch.set_active(false);
    widgets.dual_switch.set_sensitive(false);
    widgets.dual_row.set_sensitive(false);

    widgets.in_ear_switch.set_state(false);
    widgets.in_ear_switch.set_active(false);
    widgets.in_ear_switch.set_sensitive(false);
    widgets.in_ear_row.set_sensitive(false);

    widgets.sleep_switch.set_state(false);
    widgets.sleep_switch.set_active(false);
    widgets.sleep_switch.set_sensitive(false);
    widgets.sleep_row.set_sensitive(false);

    widgets.head_switch.set_state(false);
    widgets.head_switch.set_active(false);
    widgets.head_switch.set_sensitive(false);
    widgets.head_switch_row.set_sensitive(false);
}
