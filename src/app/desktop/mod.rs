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

pub(crate) mod dbus_service;
pub(crate) mod tray_service;

use std::rc::Rc;

pub(crate) use dbus_service::{
    DesktopSnapshotInput, QuickSettingsService, build_snapshot, is_non_off_writable_mode,
    resolve_toggle_target,
};
pub(crate) use tray_service::TrayService;

use super::state::HeadsetSessionState;

#[derive(Clone, Default)]
pub(crate) struct DesktopServices {
    pub quick_settings: Option<Rc<QuickSettingsService>>,
    pub tray: Option<Rc<TrayService>>,
}

impl DesktopServices {
    #[allow(dead_code)]
    pub fn new(
        quick_settings: Option<Rc<QuickSettingsService>>,
        tray: Option<Rc<TrayService>>,
    ) -> Self {
        Self {
            quick_settings,
            tray,
        }
    }

    pub fn wire_handlers(
        &self,
        session_state: std::rc::Rc<std::cell::RefCell<HeadsetSessionState>>,
        send_mode: impl Fn(u8) -> Result<(), String> + Clone + 'static,
        present_window: crate::ui::PresentWindowFn,
        quit_app: crate::ui::QuitAppFn,
    ) {
        if let Some(ref svc) = self.quick_settings {
            let send_mode_for_set = send_mode.clone();
            let send_mode_for_toggle = send_mode.clone();
            let session_state_for_toggle = std::rc::Rc::clone(&session_state);
            let present_window = std::rc::Rc::clone(&present_window);

            svc.set_handlers(
                move |mode_idx| send_mode_for_set(mode_idx),
                move || {
                    let target = {
                        let st = session_state_for_toggle.borrow();
                        resolve_toggle_target(
                            st.noise.current_mode,
                            st.noise.last_non_off_mode,
                            &st.noise.supported_indices,
                            &st.noise.modes,
                        )
                    };
                    match target {
                        Some(idx) => send_mode_for_toggle(idx),
                        None => Err(crate::t!("noise-mode-blocked")),
                    }
                },
                move || present_window(None),
            );
        }

        if let Some(ref tray_service) = self.tray {
            let send_mode_for_tray = send_mode;
            let present_window = std::rc::Rc::clone(&present_window);
            let quit_app = std::rc::Rc::clone(&quit_app);
            tray_service.set_handlers(
                move |activation_token| present_window(activation_token),
                move || quit_app(),
                move |mode_idx| send_mode_for_tray(mode_idx),
            );
        }
    }

    pub fn sync(&self, state: &HeadsetSessionState) {
        if self.quick_settings.is_none() && self.tray.is_none() {
            return;
        }
        let snap = build_snapshot(&DesktopSnapshotInput {
            connected: state.connected,
            device_name: &state.device_name,
            noise_controls_enabled: state.noise.enabled,
            supports_wear_detection: state.supports_wear_detection,
            wear: &state.wear,
            battery: &state.battery,
            case_battery_stale: state.case_battery_is_stale,
            current_noise_mode: state.noise.current_mode,
            supported_indices: &state.noise.supported_indices,
            modes: &state.noise.modes,
        });
        if let Some(service) = &self.quick_settings {
            service.update(snap.clone());
        }
        if let Some(tray) = &self.tray {
            tray.update(snap);
        }
    }
}
