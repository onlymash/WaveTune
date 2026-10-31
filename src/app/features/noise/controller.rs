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
            let worn = !st.supports_wear_detection || st.wear.any_earbud_in_ear();
            if !worn {
                return Err(crate::t!("wear-earbuds-not-in-ear"));
            }
            if !st.noise.enabled {
                return Err(crate::t!("noise-mode-blocked"));
            }
        }

        let cmd = WorkerCommand::SetNoiseMode(protocol_index);
        match self.context.send_command(cmd) {
            Ok(()) => {
                self.component.set_controls_sensitive(false);
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
                let profile = crate::compatibility::profile_for_product_id(*product_id);
                let supports_noise = profile.is_some_and(|p| p.supports_noise_control());
                self.component.widgets.set_visible(supports_noise);
                true
            }
            UiUpdate::NoiseMode(index) => {
                {
                    let mut st = self.context.session_state.borrow_mut();
                    st.noise.current_mode = Some(*index);
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
