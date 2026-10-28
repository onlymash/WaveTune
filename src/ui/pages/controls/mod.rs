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
use gtk::{
    Align, Box as GtkBox, Button, CheckButton, Label, Orientation, Scale, Spinner, Switch,
    ToggleButton,
};

use crate::{
    compatibility::{GestureRow, STANDARD_GESTURE_ROWS, gesture_function_label_key},
    ui::components::{
        ComboRowExtManual, combo_row, page_container, preferences_content_row, switch_row,
    },
};

pub mod state;

#[derive(Clone)]
pub(crate) struct ControlsWidgets {
    // Feature Toggles Group
    pub(crate) features_group: adw::PreferencesGroup,
    pub(crate) dual_row: adw::ActionRow,
    pub(crate) dual_switch: Switch,
    pub(crate) preferred_row: adw::ComboRow,
    pub(crate) in_ear_row: adw::ActionRow,
    pub(crate) in_ear_switch: Switch,
    pub(crate) sleep_row: adw::ActionRow,
    pub(crate) sleep_switch: Switch,
    pub(crate) prompt_volume_row: adw::ActionRow,
    pub(crate) prompt_volume_scale: Scale,
    pub(crate) prompt_volume_label: Label,

    // Head Motion Group
    pub(crate) head_group: adw::PreferencesGroup,
    pub(crate) head_switch_row: adw::ActionRow,
    pub(crate) head_switch: Switch,
    pub(crate) head_motion_row: adw::ComboRow,

    // Gestures Group
    pub(crate) gesture_group: adw::PreferencesGroup,
    pub(crate) ear_selector: GtkBox,
    pub(crate) ear_left_btn: ToggleButton,
    pub(crate) ear_right_btn: ToggleButton,
    pub(crate) gesture_combos: Vec<(GestureRow, adw::ComboRow)>,
    pub(crate) cycle_section: adw::ActionRow,
    pub(crate) cycle_check_anc: CheckButton,
    pub(crate) cycle_check_transparency: CheckButton,
    pub(crate) cycle_check_off: CheckButton,
    pub(crate) call_gesture_group: adw::PreferencesGroup,

    // Utilities Group
    pub(crate) utility_group: adw::PreferencesGroup,
    pub(crate) find_card: adw::ActionRow,
    pub(crate) find_btn: Button,
    pub(crate) find_status_label: Label,
    pub(crate) fit_card: adw::ActionRow,
    pub(crate) fit_btn: Button,
    pub(crate) fit_spinner: Spinner,
    pub(crate) fit_result_label: Label,
}

pub(crate) fn build_controls_page() -> (GtkBox, ControlsWidgets) {
    let page = page_container();

    // ==========================================
    // 1. Feature Settings Group
    // ==========================================
    let features_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-feature-settings"))
        .build();

    let (dual_row, dual_switch) = switch_row(
        &crate::t!("dual-device-title"),
        Some(&crate::t!("dual-device-desc")),
    );
    features_group.add(&dual_row);

    let preferred_row = combo_row(
        &crate::t!("controls-preferred-device-title"),
        Some(&crate::t!("controls-preferred-device-desc")),
    );
    features_group.add(&preferred_row);

    let (in_ear_row, in_ear_switch) = switch_row(
        &crate::t!("wear-detection"),
        Some(&crate::t!("wear-auto-pause-resume-desc")),
    );
    features_group.add(&in_ear_row);

    let (sleep_row, sleep_switch) = switch_row(
        &crate::t!("controls-sleep-detection-title"),
        Some(&crate::t!("controls-sleep-detection-desc")),
    );
    features_group.add(&sleep_row);

    // Prompt Volume Row
    let prompt_volume_row = adw::ActionRow::builder()
        .title(&crate::t!("controls-prompt-volume-title"))
        .subtitle(&crate::t!("controls-prompt-volume-desc"))
        .build();
    let volume_box = GtkBox::new(Orientation::Horizontal, 10);
    volume_box.set_valign(Align::Center);
    let prompt_volume_scale = Scale::with_range(Orientation::Horizontal, 1.0, 10.0, 1.0);
    prompt_volume_scale.set_width_request(140);
    prompt_volume_scale.set_sensitive(false);
    let prompt_volume_label = Label::new(Some("7"));
    prompt_volume_label.set_width_chars(2);
    volume_box.append(&prompt_volume_scale);
    volume_box.append(&prompt_volume_label);
    prompt_volume_row.add_suffix(&volume_box);
    features_group.add(&prompt_volume_row);

    page.append(&features_group);

    // ==========================================
    // 2. Earbud Gestures Group
    // ==========================================
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

    page.append(&gesture_group);

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
    page.append(&call_gesture_group);

    // ==========================================
    // 3. Head Motion Control Group
    // ==========================================
    let head_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-head-gestures-title"))
        .description(&crate::t!("controls-head-gestures-desc"))
        .build();

    let (head_switch_row, head_switch) =
        switch_row(&crate::t!("controls-head-gestures-title"), None);
    head_group.add(&head_switch_row);

    let head_motion_row = combo_row(&crate::t!("controls-head-gestures-title"), None);
    head_motion_row.set_items(&[
        (0, crate::t!("controls-head-nod-answers")),
        (1, crate::t!("controls-head-shake-answers")),
    ]);
    head_motion_row.set_selected_id(Some(0));
    head_group.add(&head_motion_row);

    page.append(&head_group);

    // ==========================================
    // 4. Headset Utilities Group
    // ==========================================
    let utility_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-headset-utilities"))
        .description(&crate::t!("controls-find-and-seal-desc"))
        .build();

    // Find My Earbuds ActionRow
    let find_card = adw::ActionRow::builder()
        .title(&crate::t!("controls-find-earbuds"))
        .subtitle(&crate::t!("controls-find-and-seal-desc"))
        .build();
    let find_box = GtkBox::new(Orientation::Vertical, 4);
    find_box.set_valign(Align::Center);
    let find_btn = Button::with_label(&crate::t!("controls-find-play"));
    find_btn.set_sensitive(false);
    let find_status_label = Label::new(None);
    find_status_label.set_visible(false);
    find_status_label.add_css_class("caption");
    find_status_label.add_css_class("accent");
    find_box.append(&find_btn);
    find_box.append(&find_status_label);
    find_card.add_suffix(&find_box);
    utility_group.add(&find_card);

    // Fit Test ActionRow
    let fit_card = adw::ActionRow::builder()
        .title(&crate::t!("controls-fit-test-title"))
        .subtitle(&crate::t!("controls-fit-test-desc"))
        .build();
    let fit_box = GtkBox::new(Orientation::Horizontal, 8);
    fit_box.set_valign(Align::Center);
    let fit_spinner = Spinner::new();
    fit_spinner.set_visible(false);
    let fit_btn = Button::with_label(&crate::t!("controls-fit-test-start"));
    fit_btn.set_sensitive(false);
    let fit_result_label = Label::new(None);
    fit_result_label.add_css_class("dim-label");
    fit_box.append(&fit_result_label);
    fit_box.append(&fit_spinner);
    fit_box.append(&fit_btn);
    fit_card.add_suffix(&fit_box);
    utility_group.add(&fit_card);

    page.append(&utility_group);

    let widgets = ControlsWidgets {
        features_group,
        dual_row,
        dual_switch,
        preferred_row,
        in_ear_row,
        in_ear_switch,
        sleep_row,
        sleep_switch,
        prompt_volume_row,
        prompt_volume_scale,
        prompt_volume_label,
        head_group,
        head_switch_row,
        head_switch,
        head_motion_row,
        gesture_group,
        ear_selector,
        ear_left_btn,
        ear_right_btn,
        gesture_combos,
        cycle_section,
        cycle_check_anc,
        cycle_check_transparency,
        cycle_check_off,
        call_gesture_group,
        utility_group,
        find_card,
        find_btn,
        find_status_label,
        fit_card,
        fit_btn,
        fit_spinner,
        fit_result_label,
    };

    (page, widgets)
}
