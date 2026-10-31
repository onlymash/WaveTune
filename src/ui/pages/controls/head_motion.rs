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
use gtk::Switch;

use crate::ui::components::{ComboRowExtManual, combo_row, switch_row};

#[derive(Clone)]
pub(crate) struct HeadMotionWidgets {
    head_group: adw::PreferencesGroup,
    head_switch_row: adw::ActionRow,
    head_switch: Switch,
    head_motion_row: adw::ComboRow,
    head_motion_flag: crate::ui::ReentrancyFlag,
}

impl HeadMotionWidgets {
    pub fn set_supported(&self, supports_head: bool) {
        self.head_group.set_visible(supports_head);
        self.head_switch.set_sensitive(supports_head);
        self.head_switch_row.set_sensitive(supports_head);
    }

    pub fn set_switch_state(&self, enabled: bool) {
        let _guard = self.head_motion_flag.enter();
        self.head_switch.set_state(enabled);
        self.head_switch.set_active(enabled);
        self.head_switch.set_sensitive(true);
        self.head_switch_row.set_sensitive(true);
        self.head_motion_row.set_sensitive(enabled);
    }

    pub fn set_switch_sensitive(&self, sensitive: bool) {
        self.head_switch.set_sensitive(sensitive);
        self.head_switch_row.set_sensitive(sensitive);
    }

    pub fn set_motion_type(&self, motion_type: u8) {
        let _guard = self.head_motion_flag.enter();
        self.head_motion_row.set_selected_id(Some(motion_type));
        self.head_motion_row.set_sensitive(true);
    }

    pub fn set_motion_sensitive(&self, sensitive: bool) {
        self.head_motion_row.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        self.head_group.set_visible(false);
        self.head_switch.set_state(false);
        self.head_switch.set_active(false);
        self.head_switch.set_sensitive(false);
        self.head_switch_row.set_sensitive(false);
    }

    pub fn connect_switch_toggled<F>(&self, callback: F)
    where
        F: Fn(bool, u8) + 'static,
    {
        let head_motion_flag = self.head_motion_flag.clone();
        let motion_row = self.head_motion_row.clone();
        self.head_switch.connect_state_set(move |switch, state| {
            if head_motion_flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            let motion_type = if state {
                let sel = motion_row.selected_id().unwrap_or(0);
                if sel == 0 { 1 } else { sel }
            } else {
                0
            };
            switch.set_sensitive(false);
            callback(state, motion_type);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_motion_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let head_motion_flag = self.head_motion_flag.clone();
        let head_switch = self.head_switch.clone();
        self.head_motion_row.connect_selected_notify(move |row| {
            if head_motion_flag.is_updating() {
                return;
            }
            if let Some(motion_type) = row.selected_id() {
                if head_switch.is_active() && motion_type > 0 {
                    row.set_sensitive(false);
                    callback(motion_type);
                }
            }
        });
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl HeadMotionWidgets {
    pub fn is_switch_active(&self) -> bool {
        self.head_switch.is_active()
    }

    pub fn is_switch_state(&self) -> bool {
        self.head_switch.state()
    }

    pub fn is_switch_sensitive(&self) -> bool {
        self.head_switch.is_sensitive()
    }

    pub fn is_motion_sensitive(&self) -> bool {
        self.head_motion_row.is_sensitive()
    }

    pub fn selected_motion_id(&self) -> Option<u8> {
        self.head_motion_row.selected_id()
    }
}

pub(crate) fn build_head_motion_group() -> (adw::PreferencesGroup, HeadMotionWidgets) {
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

    let widgets = HeadMotionWidgets {
        head_group: head_group.clone(),
        head_switch_row,
        head_switch,
        head_motion_row,
        head_motion_flag: crate::ui::ReentrancyFlag::new(),
    };

    (head_group, widgets)
}
