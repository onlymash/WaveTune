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

use crate::{
    app::state::HeadsetSessionState,
    ui::pages::home::noise::{NoiseControlsViewState, QuickNoiseWidgets},
};

#[derive(Clone)]
pub(crate) struct NoiseComponent {
    pub widgets: QuickNoiseWidgets,
}

impl NoiseComponent {
    pub fn new(widgets: QuickNoiseWidgets) -> Self {
        Self { widgets }
    }

    pub fn set_controls_sensitive(&self, sensitive: bool) {
        self.widgets.set_controls_sensitive(sensitive);
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
        self.widgets.render(&view_state);
    }
}
