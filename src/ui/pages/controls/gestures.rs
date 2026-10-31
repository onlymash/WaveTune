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
use gtk::{Align, Box as GtkBox, CheckButton, Orientation, ToggleButton};

use crate::{
    compatibility::{GestureRow, STANDARD_GESTURE_ROWS, gesture_function_label_key},
    protocol::{EarSide, KeyFunction, NoiseCycleState},
    ui::components::{ComboRowExtManual, combo_row, preferences_content_row},
};

#[derive(Clone)]
pub(crate) struct GestureSettingsWidgets {
    gesture_group: adw::PreferencesGroup,
    call_gesture_group: adw::PreferencesGroup,
    ear_selector: GtkBox,
    ear_left_btn: ToggleButton,
    ear_right_btn: ToggleButton,
    gesture_combos: Vec<(GestureRow, adw::ComboRow)>,
    cycle_section: adw::ActionRow,
    cycle_check_anc: CheckButton,
    cycle_check_transparency: CheckButton,
    cycle_check_off: CheckButton,
    gesture_combos_flag: crate::ui::ReentrancyFlag,
    noise_cycle_flag: crate::ui::ReentrancyFlag,
}

impl GestureSettingsWidgets {
    pub fn set_supported(&self, supports_touch: bool, gesture_rows: &[GestureRow]) {
        self.gesture_group
            .set_visible(supports_touch && !gesture_rows.is_empty());
        self.call_gesture_group
            .set_visible(supports_touch && gesture_rows.iter().any(|r| r.is_call_control));
        self.ear_selector.set_visible(supports_touch);
        self.set_gesture_combos_sensitive(supports_touch);
        self.set_noise_cycle_sensitive(supports_touch);
    }

    pub fn update_key_functions(
        &self,
        key_functions: &[KeyFunction],
        selected_ear: EarSide,
        gesture_rows: &[GestureRow],
    ) {
        let _guard = self.gesture_combos_flag.enter();
        for (row_def, combo) in &self.gesture_combos {
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
        self.set_gesture_combos_sensitive(true);

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

        self.cycle_section.set_visible(hold_controls_noise);
    }

    pub fn update_noise_cycle(&self, cycle: NoiseCycleState, selected_ear: EarSide) {
        let _guard = self.noise_cycle_flag.enter();
        let mask = if selected_ear == EarSide::Right {
            cycle.right_mask
        } else {
            cycle.left_mask
        };

        // Bit 1: Noise Cancellation, Bit 2: Transparency, Bit 0: Off
        self.cycle_check_anc.set_active((mask & (1 << 1)) != 0);
        self.cycle_check_transparency
            .set_active((mask & (1 << 2)) != 0);
        self.cycle_check_off.set_active((mask & (1 << 0)) != 0);
    }

    pub fn set_gesture_combos_sensitive(&self, sensitive: bool) {
        for (_, combo) in &self.gesture_combos {
            combo.set_sensitive(sensitive);
        }
    }

    #[cfg(test)]
    pub fn is_gesture_combos_sensitive(&self) -> bool {
        self.gesture_combos.iter().all(|(_, c)| c.is_sensitive())
    }

    pub fn set_noise_cycle_sensitive(&self, sensitive: bool) {
        self.cycle_check_anc.set_sensitive(sensitive);
        self.cycle_check_transparency.set_sensitive(sensitive);
        self.cycle_check_off.set_sensitive(sensitive);
    }

    pub fn set_ear_selection(&self, ear: EarSide) {
        self.ear_left_btn.set_active(ear == EarSide::Left);
        self.ear_right_btn.set_active(ear == EarSide::Right);
    }

    pub fn reset(&self) {
        self.gesture_group.set_visible(false);
        self.call_gesture_group.set_visible(false);
        self.ear_selector.set_visible(false);
        self.cycle_section.set_visible(false);
        self.ear_left_btn.set_active(true);
        self.ear_right_btn.set_active(false);
    }

    pub fn connect_ear_selected<F>(&self, callback: F)
    where
        F: Fn(EarSide) + 'static + Clone,
    {
        let cb_left = callback.clone();
        self.ear_left_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                cb_left(EarSide::Left);
            }
        });

        self.ear_right_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                callback(EarSide::Right);
            }
        });
    }

    pub fn connect_gesture_selected<F>(&self, callback: F)
    where
        F: Fn(GestureRow, u8, adw::ComboRow) + 'static + Clone,
    {
        for (row_def, combo) in &self.gesture_combos {
            let row_def = row_def.clone();
            let combo = combo.clone();
            let flag = self.gesture_combos_flag.clone();
            let cb = callback.clone();

            combo.connect_selected_notify(move |c| {
                if flag.is_updating() {
                    return;
                }
                if let Some(func_id) = c.selected_id() {
                    cb(row_def.clone(), func_id, c.clone());
                }
            });
        }
    }

    pub fn connect_noise_cycle_toggled<F>(&self, callback: F)
    where
        F: Fn(bool, bool, bool) + 'static + Clone,
    {
        for check in [
            &self.cycle_check_anc,
            &self.cycle_check_transparency,
            &self.cycle_check_off,
        ] {
            let flag = self.noise_cycle_flag.clone();
            let anc_btn = self.cycle_check_anc.clone();
            let trans_btn = self.cycle_check_transparency.clone();
            let off_btn = self.cycle_check_off.clone();
            let cb = callback.clone();

            check.connect_toggled(move |_| {
                if flag.is_updating() {
                    return;
                }
                cb(
                    anc_btn.is_active(),
                    trans_btn.is_active(),
                    off_btn.is_active(),
                );
            });
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl GestureSettingsWidgets {
    pub fn is_anc_active(&self) -> bool {
        self.cycle_check_anc.is_active()
    }

    pub fn is_transparency_active(&self) -> bool {
        self.cycle_check_transparency.is_active()
    }

    pub fn is_off_active(&self) -> bool {
        self.cycle_check_off.is_active()
    }

    pub fn is_ear_left_active(&self) -> bool {
        self.ear_left_btn.is_active()
    }

    pub fn is_ear_right_active(&self) -> bool {
        self.ear_right_btn.is_active()
    }

    pub fn cycle_checks(&self) -> (bool, bool, bool) {
        (
            self.cycle_check_anc.is_active(),
            self.cycle_check_transparency.is_active(),
            self.cycle_check_off.is_active(),
        )
    }
}

pub(crate) fn build_gesture_groups() -> (
    adw::PreferencesGroup,
    adw::PreferencesGroup,
    GestureSettingsWidgets,
) {
    let gesture_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-touch-gestures"))
        .description(&crate::t!("controls-gestures-desc"))
        .build();

    // Ear selector (Left / Right)
    let ear_selector = GtkBox::new(Orientation::Horizontal, 0);
    ear_selector.set_halign(Align::Center);
    ear_selector.add_css_class("linked");
    ear_selector.set_margin_top(4);
    ear_selector.set_margin_bottom(12);

    let ear_left_btn = ToggleButton::with_label(&crate::t!("controls-ear-left"));
    ear_left_btn.set_active(true);
    let ear_right_btn = ToggleButton::with_label(&crate::t!("controls-ear-right"));
    ear_right_btn.set_group(Some(&ear_left_btn));

    ear_selector.append(&ear_left_btn);
    ear_selector.append(&ear_right_btn);
    gesture_group.add(&preferences_content_row(&ear_selector));

    let mut gesture_combos = Vec::new();
    let rows = STANDARD_GESTURE_ROWS;

    // Regular gestures
    for row_def in rows.iter().filter(|r| !r.is_call_control) {
        let title = crate::t!(row_def.label_key);
        let combo = combo_row(&title, None);
        let items: Vec<(u8, String)> = row_def
            .allowed_functions
            .iter()
            .map(|&f| (f, crate::t!(gesture_function_label_key(f))))
            .collect();
        combo.set_items(&items);
        if let Some(def_f) = row_def.default_function {
            combo.set_selected_id(Some(def_f));
        }
        gesture_group.add(&combo);
        gesture_combos.push((row_def.clone(), combo));
    }

    // Noise Cycle Section (shown when squeeze and hold is noise control)
    let cycle_section = adw::ActionRow::builder()
        .title(&crate::t!("controls-noise-cycle-title"))
        .subtitle(&crate::t!("controls-noise-cycle-note"))
        .visible(false)
        .build();
    let cycle_box = GtkBox::new(Orientation::Horizontal, 12);
    cycle_box.set_valign(Align::Center);

    let cycle_check_anc = CheckButton::with_label(&crate::t!("controls-noise-cycle-anc"));
    cycle_check_anc.set_active(true);
    let cycle_check_transparency =
        CheckButton::with_label(&crate::t!("controls-noise-cycle-transparency"));
    cycle_check_transparency.set_active(true);
    let cycle_check_off = CheckButton::with_label(&crate::t!("controls-noise-cycle-off"));
    cycle_check_off.set_active(false);

    cycle_box.append(&cycle_check_anc);
    cycle_box.append(&cycle_check_transparency);
    cycle_box.append(&cycle_check_off);
    cycle_section.add_suffix(&cycle_box);
    gesture_group.add(&cycle_section);

    // Call Gestures Group
    let call_gesture_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-call-gestures-title"))
        .build();

    for row_def in rows.iter().filter(|r| r.is_call_control) {
        let title = crate::t!(row_def.label_key);
        let combo = combo_row(&title, None);
        let items: Vec<(u8, String)> = row_def
            .allowed_functions
            .iter()
            .map(|&f| (f, crate::t!(gesture_function_label_key(f))))
            .collect();
        combo.set_items(&items);
        if let Some(def_f) = row_def.default_function {
            combo.set_selected_id(Some(def_f));
        }
        call_gesture_group.add(&combo);
        gesture_combos.push((row_def.clone(), combo));
    }

    let widgets = GestureSettingsWidgets {
        gesture_group: gesture_group.clone(),
        call_gesture_group: call_gesture_group.clone(),
        ear_selector,
        ear_left_btn,
        ear_right_btn,
        gesture_combos,
        cycle_section,
        cycle_check_anc,
        cycle_check_transparency,
        cycle_check_off,
        gesture_combos_flag: crate::ui::ReentrancyFlag::new(),
        noise_cycle_flag: crate::ui::ReentrancyFlag::new(),
    };

    (gesture_group, call_gesture_group, widgets)
}
