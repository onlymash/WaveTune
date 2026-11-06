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

use super::component::NoiseComponent;
use crate::app::{
    desktop::is_non_off_writable_mode,
    events::{SettingDomain, UiUpdate, WorkerCommand},
    features::FeatureContext,
    state::HeadsetSessionState,
};

#[derive(Clone)]
pub(crate) struct NoiseController {
    component: NoiseComponent,
    context: FeatureContext,
}

impl NoiseController {
    pub fn new(component: NoiseComponent, context: FeatureContext) -> Self {
        Self { component, context }
    }

    pub fn send_mode(&self, protocol_index: u8) -> Result<(), String> {
        {
            let st = self.context.session_state.borrow();
            if !st.connected {
                return Err(crate::t!("channel-not-connected"));
            }
            let worn = !st.supports_wear_status || st.wear.any_earbud_in_ear();
            if !worn {
                return Err(crate::t!("wear-earbuds-not-in-ear"));
            }
            if !st.noise.enabled {
                return Err(crate::t!("noise-mode-blocked"));
            }
            // Only one noise-mode write may be in flight at a time. A repeated
            // request for the mode that is already pending is a no-op, so a
            // spurious widget notification can never re-send the same write.
            if let Some(pending) = st.noise.pending_mode {
                if pending == protocol_index {
                    return Ok(());
                }
                return Err(crate::t!("noise-mode-write-in-progress"));
            }
        }

        let cmd = WorkerCommand::SetNoiseMode(protocol_index);
        match self.context.send_command(cmd) {
            Ok(()) => {
                self.context.session_state.borrow_mut().noise.pending_mode = Some(protocol_index);
                self.component.set_controls_sensitive(false);
                // Defer the optimistic re-render to the next main-loop iteration.
                // `send_mode` is commonly invoked from inside the combo row's
                // `notify::selected` handler; rebuilding that row's model while the
                // notification is still being dispatched leaks a second `selected`
                // notification outside the reentrancy guard, which re-enters
                // `send_mode` and makes the headset switch modes repeatedly.
                let component = self.component.clone();
                let session_state = self.context.session_state.clone();
                gtk::glib::idle_add_local_once(move || {
                    component.render(&session_state.borrow());
                });
                Ok(())
            }
            Err(error) => {
                let msg = crate::t!("noise-mode-send-failed", error);
                self.context.send_status(msg.clone());
                Err(msg)
            }
        }
    }

    pub fn bind_events(&self) {
        let this = self.clone();
        self.component
            .widgets
            .connect_mode_clicked(move |protocol_index| {
                let _ = this.send_mode(protocol_index);
            });

        let this = self.clone();
        self.component.widgets.connect_option_selected(move |mode| {
            // Ignore selection notifications while a write is in flight; reporting
            // them as a `SettingError` would clear the pending write state.
            if this
                .context
                .session_state
                .borrow()
                .noise
                .pending_mode
                .is_some()
            {
                return;
            }
            if let Err(msg) = this.send_mode(mode) {
                this.context
                    .send_setting_error(SettingDomain::NoiseMode, msg);
            }
        });
    }

    pub fn on_update(&self, update: &UiUpdate) -> bool {
        match update {
            UiUpdate::InitialSession(snapshot) => {
                let supports_noise = !snapshot.noise.modes.is_empty();
                self.component.widgets.set_visible(supports_noise);
                self.component.render(&self.context.session_state.borrow());
                true
            }
            UiUpdate::ProductId { product_id, .. } => {
                let profile = crate::compatibility::profile_for_identity(
                    *product_id,
                    self.context
                        .session_state
                        .borrow()
                        .firmware_model_code
                        .as_deref(),
                );
                let supports_noise = profile.is_some_and(|p| p.supports_noise_control());
                self.component.widgets.set_visible(supports_noise);
                true
            }
            UiUpdate::NoiseMode(index) => {
                let mut st = self.context.session_state.borrow_mut();
                if st.noise.observe_mode(*index) {
                    if is_non_off_writable_mode(*index, &st.noise.modes) {
                        st.noise.last_non_off_mode = Some(*index);
                    }
                }
                drop(st);
                self.component.render(&self.context.session_state.borrow());
                true
            }
            UiUpdate::NoiseModeWriteCompleted(index) | UiUpdate::NoiseModeConfirmed(index) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.noise.complete_write(*index);
                    if is_non_off_writable_mode(*index, &st.noise.modes) {
                        st.noise.last_non_off_mode = Some(*index);
                    }
                }
                self.component.render(&self.context.session_state.borrow());
                true
            }
            UiUpdate::NoiseControlsEnabled(enabled) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.noise.enabled = *enabled && st.connected;
                }
                self.component.render(&self.context.session_state.borrow());
                true
            }
            UiUpdate::NoiseModeOptions {
                indices,
                note,
                modes,
            } => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.noise.supported_indices = indices.clone();
                    st.noise.modes = modes.clone();
                    st.noise.support_note = note.clone();
                    if let Some(idx) = st.noise.current_mode
                        && is_non_off_writable_mode(idx, &st.noise.modes)
                    {
                        st.noise.last_non_off_mode = Some(idx);
                    }
                }
                self.component.render(&self.context.session_state.borrow());
                true
            }
            UiUpdate::SettingError {
                domain: SettingDomain::NoiseMode,
                ..
            } => {
                self.context.session_state.borrow_mut().noise.pending_mode = None;
                self.component.render(&self.context.session_state.borrow());
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, session: &HeadsetSessionState) {
        self.component.render(session);
    }
}
