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

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use adw::prelude::*;

use super::component::ControlsComponent;
use crate::{
    app::{
        events::{SettingDomain, UiUpdate, WorkerCommand},
        features::FeatureContext,
    },
    protocol::{EarSide, PairedDevice},
    ui::components::ComboRowExtManual,
};

#[derive(Clone)]
pub(crate) struct ControlsController {
    component: ControlsComponent,
    context: FeatureContext,
    selected_ear: Rc<Cell<EarSide>>,
    cached_paired_devices: Rc<RefCell<Vec<PairedDevice>>>,
    is_finding_earbuds: Rc<Cell<bool>>,
    fit_test_running: Rc<Cell<bool>>,
    prompt_volume_debounce_source: Rc<RefCell<Option<gtk::glib::SourceId>>>,
}

impl ControlsController {
    pub fn new(
        component: ControlsComponent,
        context: FeatureContext,
        cached_paired_devices: Rc<RefCell<Vec<PairedDevice>>>,
    ) -> Self {
        Self {
            component,
            context,
            selected_ear: Rc::new(Cell::new(EarSide::Left)),
            cached_paired_devices,
            is_finding_earbuds: Rc::new(Cell::new(false)),
            fit_test_running: Rc::new(Cell::new(false)),
            prompt_volume_debounce_source: Rc::new(RefCell::new(None)),
        }
    }

    pub fn bind_events(&self) {
        self.bind_ear_selection_events();
        self.bind_gesture_combo_events();
        self.bind_noise_cycle_events();
        self.bind_find_and_fit_events();
        self.bind_prompt_volume_events();
        self.bind_head_motion_events();
        self.bind_dual_device_events();
        self.bind_detection_events();
        self.bind_preferred_device_events();
    }

    fn bind_ear_selection_events(&self) {
        let selected_ear = self.selected_ear.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .ear_left_btn
            .connect_toggled(move |btn| {
                if btn.is_active() {
                    selected_ear.set(EarSide::Left);
                    let st = context.session_state.borrow();
                    component.render_key_functions(
                        &st.controls.key_functions,
                        EarSide::Left,
                        &st.controls.gesture_rows,
                    );
                    component.render_noise_cycle(st.controls.noise_cycle, EarSide::Left);
                }
            });

        let selected_ear = self.selected_ear.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .ear_right_btn
            .connect_toggled(move |btn| {
                if btn.is_active() {
                    selected_ear.set(EarSide::Right);
                    let st = context.session_state.borrow();
                    component.render_key_functions(
                        &st.controls.key_functions,
                        EarSide::Right,
                        &st.controls.gesture_rows,
                    );
                    component.render_noise_cycle(st.controls.noise_cycle, EarSide::Right);
                }
            });
    }

    fn bind_gesture_combo_events(&self) {
        for (row_def, combo) in &self.component.widgets.gesture_combos {
            let row_def = row_def.clone();
            let combo = combo.clone();
            let selected_ear = self.selected_ear.clone();
            let gesture_combos_flag = self.component.widgets.gesture_combos_flag.clone();
            let component = self.component.clone();
            let context = self.context.clone();

            combo.connect_selected_notify(move |c| {
                if gesture_combos_flag.is_updating() {
                    return;
                }
                if let Some(func_id) = c.selected_id() {
                    let side = selected_ear.get();
                    let button = row_def.button;
                    let action = row_def.action;
                    let function = func_id;

                    c.set_sensitive(false);
                    let cmd = WorkerCommand::SetGesture {
                        side,
                        button,
                        action,
                        function,
                    };
                    match context.send_command(cmd) {
                        Ok(()) => {}
                        Err(err) => {
                            context.send_status(crate::t!("controls-gesture-send-failed", err));
                            let st = context.session_state.borrow();
                            component.render_key_functions(
                                &st.controls.key_functions,
                                selected_ear.get(),
                                &st.controls.gesture_rows,
                            );
                            c.set_sensitive(true);
                        }
                    }
                }
            });
        }
    }

    fn bind_noise_cycle_events(&self) {
        for check in [
            &self.component.widgets.cycle_check_anc,
            &self.component.widgets.cycle_check_transparency,
            &self.component.widgets.cycle_check_off,
        ] {
            let component = self.component.clone();
            let selected_ear = self.selected_ear.clone();
            let noise_cycle_flag = self.component.widgets.noise_cycle_flag.clone();
            let context = self.context.clone();

            check.connect_toggled(move |_| {
                if noise_cycle_flag.is_updating() {
                    return;
                }
                let anc = component.widgets.cycle_check_anc.is_active();
                let trans = component.widgets.cycle_check_transparency.is_active();
                let off = component.widgets.cycle_check_off.is_active();
                let count = (anc as u8) + (trans as u8) + (off as u8);
                if count < 2 {
                    context.send_status(crate::t!("controls-noise-cycle-min-warning"));
                    let cur = context.session_state.borrow().controls.noise_cycle;
                    component.render_noise_cycle(cur, selected_ear.get());
                    return;
                }
                let mut mask = 0u8;
                if off {
                    mask |= 1 << 0;
                }
                if anc {
                    mask |= 1 << 1;
                }
                if trans {
                    mask |= 1 << 2;
                }
                let side = selected_ear.get();
                let cmd = WorkerCommand::SetNoiseCycle {
                    side,
                    mask: mask as u32,
                    is_shared: false,
                };
                match context.send_command(cmd) {
                    Ok(()) => {
                        component.widgets.cycle_check_anc.set_sensitive(false);
                        component
                            .widgets
                            .cycle_check_transparency
                            .set_sensitive(false);
                        component.widgets.cycle_check_off.set_sensitive(false);
                    }
                    Err(err) => {
                        context.send_status(crate::t!("controls-noise-cycle-send-failed", err));
                        let cur = context.session_state.borrow().controls.noise_cycle;
                        component.render_noise_cycle(cur, selected_ear.get());
                    }
                }
            });
        }
    }

    fn bind_find_and_fit_events(&self) {
        // 1. Find earbuds
        let context = self.context.clone();
        let component = self.component.clone();
        let is_finding = self.is_finding_earbuds.clone();

        self.component.widgets.find_btn.connect_clicked(move |_| {
            if is_finding.get() {
                component.widgets.find_btn.set_sensitive(false);
                if let Err(err) = context.send_command(WorkerCommand::FindEarbuds(false)) {
                    context.send_status(crate::t!("controls-find-send-failed", err));
                    component.widgets.find_btn.set_sensitive(true);
                }
            } else {
                let in_ear = context.session_state.borrow().wear.any_earbud_in_ear();
                let context_find = context.clone();
                let find_btn_find = component.widgets.find_btn.clone();
                let do_play = move || {
                    find_btn_find.set_sensitive(false);
                    if let Err(err) = context_find.send_command(WorkerCommand::FindEarbuds(true)) {
                        context_find.send_status(crate::t!("controls-find-send-failed", err));
                        find_btn_find.set_sensitive(true);
                    }
                };

                if in_ear {
                    let alert = adw::AlertDialog::builder()
                        .heading(&crate::t!("controls-find-warning-title"))
                        .body(&crate::t!("controls-find-warning-body"))
                        .build();
                    alert.add_response("cancel", &crate::t!("common-cancel"));
                    alert.add_response("play", &crate::t!("controls-find-play"));
                    alert.set_response_appearance("play", adw::ResponseAppearance::Destructive);
                    alert.choose(
                        Some(&component.widgets.find_btn),
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "play" {
                                do_play();
                            }
                        },
                    );
                } else {
                    do_play();
                }
            }
        });

        // 2. Fit test
        let context = self.context.clone();
        let component = self.component.clone();

        self.component.widgets.fit_btn.connect_clicked(move |_| {
            component.render_fit_test_running(true);
            if let Err(err) = context.send_command(WorkerCommand::RunFitTest) {
                context.send_status(crate::t!("controls-fit-test-send-failed", err));
                component.render_fit_test_running(false);
            }
        });
    }

    fn bind_prompt_volume_events(&self) {
        let prompt_volume_flag = self.component.widgets.prompt_volume_flag.clone();
        let debounce = self.prompt_volume_debounce_source.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .prompt_volume_scale
            .connect_value_changed(move |slider| {
                let rounded = slider.value().round();
                if (slider.value() - rounded).abs() > 0.001 {
                    slider.set_value(rounded);
                    return;
                }
                let value = rounded as u8;
                component
                    .widgets
                    .prompt_volume_label
                    .set_text(&value.to_string());
                if prompt_volume_flag.is_updating() {
                    return;
                }
                if let Some(source_id) = debounce.borrow_mut().take() {
                    source_id.remove();
                }

                let debounce_clone = debounce.clone();
                let context = context.clone();

                let source_id =
                    gtk::glib::timeout_add_local(Duration::from_millis(500), move || {
                        *debounce_clone.borrow_mut() = None;
                        if let Err(err) =
                            context.send_command(WorkerCommand::SetPromptVolume(value))
                        {
                            context
                                .send_status(crate::t!("controls-prompt-volume-send-failed", err));
                        }
                        gtk::glib::ControlFlow::Break
                    });
                *debounce.borrow_mut() = Some(source_id);
            });
    }

    fn bind_head_motion_events(&self) {
        // 1. Head switch
        let head_motion_flag = self.component.widgets.head_motion_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .head_switch
            .connect_state_set(move |switch, state| {
                if head_motion_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                let motion_type = if state {
                    let sel = component.widgets.head_motion_row.selected_id().unwrap_or(0);
                    if sel == 0 { 1 } else { sel }
                } else {
                    0
                };

                switch.set_sensitive(false);
                let cmd = WorkerCommand::SetHeadMotionType(motion_type);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("controls-head-motion-send-failed", err));
                        let cur = context.session_state.borrow().controls.head_motion;
                        component.render_head_motion(cur);
                    }
                }
                gtk::glib::Propagation::Stop
            });

        // 2. Head motion row
        let head_motion_flag = self.component.widgets.head_motion_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .head_motion_row
            .connect_selected_notify(move |row| {
                if head_motion_flag.is_updating() {
                    return;
                }
                if let Some(motion_type) = row.selected_id() {
                    if component.widgets.head_switch.is_active() && motion_type > 0 {
                        row.set_sensitive(false);
                        let cmd = WorkerCommand::SetHeadMotionType(motion_type);
                        match context.send_command(cmd) {
                            Ok(()) => {}
                            Err(err) => {
                                context.send_status(crate::t!(
                                    "controls-head-motion-send-failed",
                                    err
                                ));
                                let cur = context.session_state.borrow().controls.head_motion;
                                component.render_head_motion(cur);
                            }
                        }
                    }
                }
            });
    }

    fn bind_dual_device_events(&self) {
        let dual_device_flag = self.component.widgets.dual_device_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .dual_switch
            .connect_state_set(move |switch, state| {
                if dual_device_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }

                // If user enables dual device while LDAC is active on EarFun:
                let is_ldac = context.session_state.borrow().sound.current_audio_codec
                    == Some(crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8());

                if state && is_ldac {
                    let switch_for_dialog = switch.clone();
                    let switch_for_action = switch.clone();
                    let updating_dual_confirm = dual_device_flag.clone();
                    let context_dialog = context.clone();

                    let dialog = adw::AlertDialog::builder()
                        .heading(&crate::t!("dialog-reboot-required-title"))
                        .body(&crate::t!("dialog-dual-device-conflict-message"))
                        .build();
                    dialog.add_response("cancel", &crate::t!("common-cancel"));
                    dialog.add_response("confirm", &crate::t!("common-confirm"));
                    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);

                    dialog.choose(
                        Some(&switch_for_dialog),
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "confirm" {
                                switch_for_action.set_sensitive(false);
                                let _ = context_dialog
                                    .send_command(WorkerCommand::SetDualDeviceConnection(true));
                            } else {
                                let _guard = updating_dual_confirm.enter();
                                switch_for_action.set_state(false);
                                switch_for_action.set_active(false);
                            }
                        },
                    );
                    return gtk::glib::Propagation::Stop;
                }

                switch.set_sensitive(false);
                let cmd = WorkerCommand::SetDualDeviceConnection(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("dual-device-send-failed", err));
                        let cur = context
                            .session_state
                            .borrow()
                            .dual_device
                            .current_dual_device;
                        component.render_dual_device(cur);
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    fn bind_detection_events(&self) {
        // 1. In-ear detection
        let in_ear_flag = self.component.widgets.in_ear_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .in_ear_switch
            .connect_state_set(move |switch, state| {
                if in_ear_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                switch.set_sensitive(false);
                let cmd = WorkerCommand::SetInEarDetection(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("wear-send-failed", err));
                        let cur = context.session_state.borrow().controls.in_ear_detection;
                        component.render_in_ear(cur);
                    }
                }
                gtk::glib::Propagation::Stop
            });

        // 2. Sleep detection
        let sleep_flag = self.component.widgets.sleep_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .sleep_switch
            .connect_state_set(move |switch, state| {
                if sleep_flag.is_updating() {
                    return gtk::glib::Propagation::Proceed;
                }
                switch.set_sensitive(false);
                let cmd = WorkerCommand::SetSleepDetection(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("sleep-detection-send-failed", err));
                        let cur = context.session_state.borrow().controls.sleep_detection;
                        component.render_sleep(cur);
                    }
                }
                gtk::glib::Propagation::Stop
            });
    }

    fn bind_preferred_device_events(&self) {
        let preferred_flag = self.component.widgets.preferred_flag.clone();
        let component = self.component.clone();
        let context = self.context.clone();
        let cached_devices = self.cached_paired_devices.clone();

        self.component
            .widgets
            .preferred_row
            .connect_selected_notify(move |row| {
                if preferred_flag.is_updating() {
                    return;
                }
                if let Some(id) = row.selected_id() {
                    let target_mac = if id == 0 {
                        None
                    } else {
                        let idx = (id - 1) as usize;
                        cached_devices.borrow().get(idx).map(|d| d.address)
                    };
                    row.set_sensitive(false);
                    let cmd = WorkerCommand::SetPreferredDevice(target_mac);
                    match context.send_command(cmd) {
                        Ok(()) => {}
                        Err(err) => {
                            context.send_status(crate::t!(
                                "controls-preferred-device-send-failed",
                                err
                            ));
                            let pref = context.session_state.borrow().controls.preferred_device;
                            component.render_preferred_devices(pref, &cached_devices.borrow());
                        }
                    }
                }
            });
    }

    pub fn on_update(&self, update: &UiUpdate) -> bool {
        match update {
            UiUpdate::ProductId { product_id, .. } => {
                let profile = crate::compatibility::profile_for_product_id(*product_id);
                let supports_dual_device =
                    profile.is_some_and(|profile| profile.supports_dual_device());
                self.context
                    .session_state
                    .borrow_mut()
                    .dual_device
                    .supports_dual_device = supports_dual_device;
                self.render_features();
                true
            }
            UiUpdate::KeyFunctions(functions) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.key_functions = functions.clone();
                }
                let st = self.context.session_state.borrow();
                self.component.render_key_functions(
                    functions,
                    self.selected_ear.get(),
                    &st.controls.gesture_rows,
                );
                for (_, combo) in &self.component.widgets.gesture_combos {
                    combo.set_sensitive(true);
                }
                true
            }
            UiUpdate::NoiseCycle(cycle) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.noise_cycle = Some(*cycle);
                }
                self.component
                    .render_noise_cycle(Some(*cycle), self.selected_ear.get());
                self.component.widgets.cycle_check_anc.set_sensitive(true);
                self.component
                    .widgets
                    .cycle_check_transparency
                    .set_sensitive(true);
                self.component.widgets.cycle_check_off.set_sensitive(true);
                true
            }
            UiUpdate::IsFindingEarbuds(finding) => {
                self.is_finding_earbuds.set(*finding);
                self.component.render_finding_earbuds(*finding);
                true
            }
            UiUpdate::FitTestRunning(running) => {
                self.fit_test_running.set(*running);
                self.component.render_fit_test_running(*running);
                true
            }
            UiUpdate::FitTestResult(result) => {
                self.fit_test_running.set(false);
                self.component.render_fit_test_running(false);
                self.component.render_fit_test_result(*result);
                true
            }
            UiUpdate::PromptVolume(volume) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.prompt_volume = Some(*volume);
                }
                self.component.render_prompt_volume(Some(*volume));
                true
            }
            UiUpdate::HeadMotionType(motion_type) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.head_motion = Some(*motion_type);
                }
                self.component.render_head_motion(Some(*motion_type));
                true
            }
            UiUpdate::DualDeviceConnectionEnabled(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.dual_device.current_dual_device = Some(*enabled);
                }
                self.component.render_dual_device(Some(*enabled));
                true
            }
            UiUpdate::InEarDetectionEnabled(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.in_ear_detection = Some(*enabled);
                }
                self.component.render_in_ear(Some(*enabled));
                true
            }
            UiUpdate::SleepDetectionEnabled(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.sleep_detection = Some(*enabled);
                }
                self.component.render_sleep(Some(*enabled));
                true
            }
            UiUpdate::PreferredDevice(address) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.preferred_device = *address;
                }
                self.component
                    .render_preferred_devices(*address, &self.cached_paired_devices.borrow());
                true
            }
            UiUpdate::ControlsFeatures {
                supports_touch_customization,
                supports_dual_device,
                supports_wear_detection,
                supports_sleep_detection,
                supports_head_motion,
                supports_find_device,
                supports_fit_detection,
                supports_prompt_volume,
                gesture_rows,
            } => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.supports_touch_customization = *supports_touch_customization;
                    st.dual_device.supports_dual_device = *supports_dual_device;
                    st.supports_wear_detection = *supports_wear_detection;
                    st.controls.supports_sleep_detection = *supports_sleep_detection;
                    st.controls.supports_head_motion = *supports_head_motion;
                    st.controls.supports_find_device = *supports_find_device;
                    st.controls.supports_fit_detection = *supports_fit_detection;
                    st.controls.supports_prompt_volume = *supports_prompt_volume;
                    st.controls.gesture_rows = gesture_rows.clone();
                }
                self.render_features();
                true
            }
            UiUpdate::SettingError {
                domain: SettingDomain::Controls,
                ..
            } => {
                self.component.re_enable_controls();
                true
            }
            _ => false,
        }
    }

    pub fn render_features(&self) {
        let st = self.context.session_state.borrow();
        self.component.render_features(
            &st.controls,
            st.dual_device.supports_dual_device,
            st.supports_wear_detection,
        );
    }

    pub fn reset(&self) {
        if let Some(source_id) = self.prompt_volume_debounce_source.borrow_mut().take() {
            source_id.remove();
        }
        self.component.reset();
        self.selected_ear.set(EarSide::Left);
        self.component.widgets.ear_left_btn.set_active(true);
        self.component.widgets.ear_right_btn.set_active(false);
    }
}
