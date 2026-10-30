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

use crate::{
    app::state::HeadsetSessionState,
    ui::{
        HomeWidgetRefs, PrimaryNoiseControl, ReentrancyFlag,
        home_state::{self, NoiseControlsViewState},
    },
};

#[derive(Clone)]
pub(crate) struct NoiseComponent {
    pub noise_option_row: adw::ComboRow,
    pub primary_noise_controls: Vec<PrimaryNoiseControl>,
    pub noise_option_flag: ReentrancyFlag,
    widget_refs: HomeWidgetRefs,
}

impl NoiseComponent {
    pub fn new(widget_refs: HomeWidgetRefs) -> Self {
        Self {
            noise_option_row: widget_refs.noise_option_row.clone(),
            primary_noise_controls: widget_refs.primary_noise_controls.clone(),
            noise_option_flag: widget_refs.noise_option_flag.clone(),
            widget_refs,
        }
    }

    pub fn set_controls_sensitive(&self, sensitive: bool) {
        for control in &self.primary_noise_controls {
            control.button.set_sensitive(sensitive);
        }
        self.noise_option_row.set_sensitive(sensitive);
    }

    pub fn render(&self, session: &HeadsetSessionState) {
        let view_state = NoiseControlsViewState {
            supported_indices: &session.noise.supported_indices,
            support_note: &session.noise.support_note,
            capability_enabled: session.noise.enabled && session.connected,
            supports_wear_detection: session.supports_wear_detection,
            wear: &session.wear,
            current_mode: session.noise.current_mode,
            modes: &session.noise.modes,
        };
        home_state::update_noise_controls(&self.widget_refs, &view_state);
    }
}
