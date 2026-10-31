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
            .gestures
            .connect_ear_selected(move |side| {
                selected_ear.set(side);
                let st = context.session_state.borrow();
                component.render_key_functions(
                    &st.controls.key_functions,
                    side,
                    &st.controls.gesture_rows,
                );
                component.render_noise_cycle(st.controls.noise_cycle, side);
            });
    }

    fn bind_gesture_combo_events(&self) {
        let selected_ear = self.selected_ear.clone();
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .gestures
            .connect_gesture_selected(move |row_def, func_id, combo| {
                let side = selected_ear.get();
                let button = row_def.button;
                let action = row_def.action;
                let function = func_id;

                combo.set_sensitive(false);
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
                        combo.set_sensitive(true);
                    }
                }
            });
    }

    fn bind_noise_cycle_events(&self) {
        let component = self.component.clone();
        let selected_ear = self.selected_ear.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .gestures
            .connect_noise_cycle_toggled(move |anc, trans, off| {
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
                        component.widgets.gestures.set_noise_cycle_sensitive(false);
                    }
                    Err(err) => {
                        context.send_status(crate::t!("controls-noise-cycle-send-failed", err));
                        let cur = context.session_state.borrow().controls.noise_cycle;
                        component.render_noise_cycle(cur, selected_ear.get());
                    }
                }
            });
    }

    fn bind_find_and_fit_events(&self) {
        // 1. Find earbuds
        let context = self.context.clone();
        let component = self.component.clone();
        let is_finding = self.is_finding_earbuds.clone();

        self.component
            .widgets
            .utilities
            .connect_find_clicked(move || {
                if is_finding.get() {
                    component.widgets.utilities.set_find_sensitive(false);
                    if let Err(err) = context.send_command(WorkerCommand::FindEarbuds(false)) {
                        context.send_status(crate::t!("controls-find-send-failed", err));
                        component.widgets.utilities.set_find_sensitive(true);
                    }
                } else {
                    let in_ear = context.session_state.borrow().wear.any_earbud_in_ear();
                    let context_find = context.clone();
                    let utilities_find = component.widgets.utilities.clone();
                    let do_play = move || {
                        utilities_find.set_find_sensitive(false);
                        if let Err(err) =
                            context_find.send_command(WorkerCommand::FindEarbuds(true))
                        {
                            context_find.send_status(crate::t!("controls-find-send-failed", err));
                            utilities_find.set_find_sensitive(true);
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
                            Some(component.widgets.utilities.find_button()),
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

        self.component
            .widgets
            .utilities
            .connect_fit_clicked(move || {
                component.render_fit_test_running(true);
                if let Err(err) = context.send_command(WorkerCommand::RunFitTest) {
                    context.send_status(crate::t!("controls-fit-test-send-failed", err));
                    component.render_fit_test_running(false);
                }
            });
    }

    fn bind_prompt_volume_events(&self) {
        let debounce = self.prompt_volume_debounce_source.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .features
            .connect_prompt_volume_changed(move |value| {
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
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .head_motion
            .connect_switch_toggled(move |_state, motion_type| {
                let cmd = WorkerCommand::SetHeadMotionType(motion_type);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("controls-head-motion-send-failed", err));
                        let cur = context.session_state.borrow().controls.head_motion;
                        component.render_head_motion(cur);
                    }
                }
            });

        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .head_motion
            .connect_motion_selected(move |motion_type| {
                let cmd = WorkerCommand::SetHeadMotionType(motion_type);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("controls-head-motion-send-failed", err));
                        let cur = context.session_state.borrow().controls.head_motion;
                        component.render_head_motion(cur);
                    }
                }
            });
    }

    fn bind_dual_device_events(&self) {
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .features
            .connect_dual_toggled(move |state| {
                // If user enables dual device while LDAC is active on EarFun:
                let is_ldac = context.session_state.borrow().sound.current_audio_codec
                    == Some(crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8());

                if state && is_ldac {
                    let switch_for_dialog = component.widgets.features.dual_switch().clone();
                    let features_for_revert = component.widgets.features.clone();
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
                                features_for_revert.set_dual_sensitive(false);
                                let _ = context_dialog
                                    .send_command(WorkerCommand::SetDualDeviceConnection(true));
                            } else {
                                features_for_revert.revert_dual_switch();
                            }
                        },
                    );
                    return;
                }

                component.widgets.features.set_dual_sensitive(false);
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
            });
    }

    fn bind_detection_events(&self) {
        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .features
            .connect_in_ear_toggled(move |state| {
                let cmd = WorkerCommand::SetInEarDetection(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("wear-send-failed", err));
                        let cur = context.session_state.borrow().controls.in_ear_detection;
                        component.render_in_ear(cur);
                    }
                }
            });

        let component = self.component.clone();
        let context = self.context.clone();

        self.component
            .widgets
            .features
            .connect_sleep_toggled(move |state| {
                let cmd = WorkerCommand::SetSleepDetection(state);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context.send_status(crate::t!("sleep-detection-send-failed", err));
                        let cur = context.session_state.borrow().controls.sleep_detection;
                        component.render_sleep(cur);
                    }
                }
            });
    }

    fn bind_preferred_device_events(&self) {
        let component = self.component.clone();
        let context = self.context.clone();
        let cached_devices = self.cached_paired_devices.clone();

        self.component
            .widgets
            .features
            .connect_preferred_selected(move |id| {
                let target_mac = if id == 0 {
                    None
                } else {
                    let idx = (id - 1) as usize;
                    cached_devices.borrow().get(idx).map(|d| d.address)
                };
                let cmd = WorkerCommand::SetPreferredDevice(target_mac);
                match context.send_command(cmd) {
                    Ok(()) => {}
                    Err(err) => {
                        context
                            .send_status(crate::t!("controls-preferred-device-send-failed", err));
                        let pref = context.session_state.borrow().controls.preferred_device;
                        component.render_preferred_devices(pref, &cached_devices.borrow());
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
                self.component
                    .widgets
                    .gestures
                    .set_gesture_combos_sensitive(true);
                true
            }
            UiUpdate::NoiseCycle(cycle) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.controls.noise_cycle = Some(*cycle);
                }
                self.component
                    .render_noise_cycle(Some(*cycle), self.selected_ear.get());
                self.component
                    .widgets
                    .gestures
                    .set_noise_cycle_sensitive(true);
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
        self.component
            .widgets
            .gestures
            .set_ear_selection(EarSide::Left);
    }
}
