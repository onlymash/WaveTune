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
use gtk::{Align, Box as GtkBox, Label, Orientation, Scale, Switch};

use crate::{
    protocol::{PairedDevice, PairedDeviceAddress},
    ui::components::{ComboRowExtManual, combo_row, switch_row},
};

#[derive(Clone)]
pub(crate) struct FeatureTogglesWidgets {
    features_group: adw::PreferencesGroup,
    dual_row: adw::ActionRow,
    dual_switch: Switch,
    preferred_row: adw::ComboRow,
    in_ear_row: adw::ActionRow,
    in_ear_switch: Switch,
    sleep_row: adw::ActionRow,
    sleep_switch: Switch,
    prompt_volume_row: adw::ActionRow,
    prompt_volume_scale: Scale,
    prompt_volume_label: Label,
    dual_device_flag: crate::ui::ReentrancyFlag,
    in_ear_flag: crate::ui::ReentrancyFlag,
    sleep_flag: crate::ui::ReentrancyFlag,
    preferred_flag: crate::ui::ReentrancyFlag,
    prompt_volume_flag: crate::ui::ReentrancyFlag,
}

impl FeatureTogglesWidgets {
    pub fn set_supported(
        &self,
        supports_dual: bool,
        supports_wear: bool,
        supports_sleep: bool,
        supports_volume: bool,
    ) {
        self.features_group
            .set_visible(supports_dual || supports_wear || supports_sleep || supports_volume);
        self.dual_row.set_visible(supports_dual);
        self.dual_switch.set_sensitive(supports_dual);
        self.dual_row.set_sensitive(supports_dual);

        self.preferred_row.set_visible(supports_dual);
        self.preferred_row.set_sensitive(supports_dual);

        self.in_ear_row.set_visible(supports_wear);
        self.in_ear_switch.set_sensitive(supports_wear);
        self.in_ear_row.set_sensitive(supports_wear);

        self.sleep_row.set_visible(supports_sleep);
        self.sleep_switch.set_sensitive(supports_sleep);
        self.sleep_row.set_sensitive(supports_sleep);

        self.prompt_volume_row.set_visible(supports_volume);
    }

    pub fn set_dual_device(&self, enabled: bool) {
        let _guard = self.dual_device_flag.enter();
        self.dual_switch.set_state(enabled);
        self.dual_switch.set_active(enabled);
        self.dual_switch.set_sensitive(true);
        self.dual_row.set_sensitive(true);
        self.preferred_row.set_sensitive(enabled);
    }

    pub fn dual_switch(&self) -> &Switch {
        &self.dual_switch
    }

    pub fn revert_dual_switch(&self) {
        let _guard = self.dual_device_flag.enter();
        self.dual_switch.set_state(false);
        self.dual_switch.set_active(false);
    }

    pub fn set_dual_sensitive(&self, sensitive: bool) {
        self.dual_switch.set_sensitive(sensitive);
        self.dual_row.set_sensitive(sensitive);
    }

    pub fn set_in_ear_detection(&self, enabled: bool) {
        let _guard = self.in_ear_flag.enter();
        self.in_ear_switch.set_state(enabled);
        self.in_ear_switch.set_active(enabled);
        self.in_ear_switch.set_sensitive(true);
        self.in_ear_row.set_sensitive(true);
    }

    pub fn set_in_ear_sensitive(&self, sensitive: bool) {
        self.in_ear_switch.set_sensitive(sensitive);
        self.in_ear_row.set_sensitive(sensitive);
    }

    pub fn set_sleep_detection(&self, enabled: bool) {
        let _guard = self.sleep_flag.enter();
        self.sleep_switch.set_state(enabled);
        self.sleep_switch.set_active(enabled);
        self.sleep_switch.set_sensitive(true);
        self.sleep_row.set_sensitive(true);
    }

    pub fn set_sleep_sensitive(&self, sensitive: bool) {
        self.sleep_switch.set_sensitive(sensitive);
        self.sleep_row.set_sensitive(sensitive);
    }

    pub fn set_prompt_volume(&self, volume: u8) {
        let _guard = self.prompt_volume_flag.enter();
        self.prompt_volume_scale.set_value(volume as f64);
        self.prompt_volume_label.set_text(&volume.to_string());
        self.prompt_volume_scale.set_sensitive(true);
    }

    pub fn set_preferred_devices(
        &self,
        preferred: Option<PairedDeviceAddress>,
        devices: &[PairedDevice],
    ) {
        let _guard = self.preferred_flag.enter();
        let mut items: Vec<(u8, String)> = vec![(0, crate::t!("controls-preferred-device-auto"))];
        let mut selected_id = 0u8;

        for (i, dev) in devices.iter().enumerate() {
            let id = (i + 1) as u8;
            let label = if dev.name.is_empty() {
                dev.address.format_mac()
            } else {
                dev.name.clone()
            };
            items.push((id, label));
            if let Some(pref) = preferred {
                if dev.address == pref {
                    selected_id = id;
                }
            }
        }

        self.preferred_row.set_items(&items);
        self.preferred_row.set_selected_id(Some(selected_id));
        self.preferred_row.set_sensitive(true);
    }

    pub fn set_preferred_sensitive(&self, sensitive: bool) {
        self.preferred_row.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        self.features_group.set_visible(false);

        self.dual_switch.set_state(false);
        self.dual_switch.set_active(false);
        self.dual_switch.set_sensitive(false);
        self.dual_row.set_sensitive(false);

        self.in_ear_switch.set_state(false);
        self.in_ear_switch.set_active(false);
        self.in_ear_switch.set_sensitive(false);
        self.in_ear_row.set_sensitive(false);

        self.sleep_switch.set_state(false);
        self.sleep_switch.set_active(false);
        self.sleep_switch.set_sensitive(false);
        self.sleep_row.set_sensitive(false);
    }

    pub fn connect_dual_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let flag = self.dual_device_flag.clone();
        self.dual_switch.connect_state_set(move |_switch, state| {
            if flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_in_ear_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let flag = self.in_ear_flag.clone();
        self.in_ear_switch.connect_state_set(move |switch, state| {
            if flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            switch.set_sensitive(false);
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_sleep_toggled<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let flag = self.sleep_flag.clone();
        self.sleep_switch.connect_state_set(move |switch, state| {
            if flag.is_updating() {
                return gtk::glib::Propagation::Proceed;
            }
            switch.set_sensitive(false);
            callback(state);
            gtk::glib::Propagation::Stop
        });
    }

    pub fn connect_prompt_volume_changed<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let flag = self.prompt_volume_flag.clone();
        let label = self.prompt_volume_label.clone();
        self.prompt_volume_scale
            .connect_value_changed(move |scale| {
                let vol = scale.value().round() as u8;
                label.set_text(&vol.to_string());
                if flag.is_updating() {
                    return;
                }
                callback(vol);
            });
    }

    pub fn connect_preferred_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let flag = self.preferred_flag.clone();
        self.preferred_row.connect_selected_notify(move |row| {
            if flag.is_updating() {
                return;
            }
            let Some(id) = row.selected_id() else {
                return;
            };
            row.set_sensitive(false);
            callback(id);
        });
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl FeatureTogglesWidgets {
    pub fn is_dual_active(&self) -> bool {
        self.dual_switch.is_active()
    }

    pub fn is_dual_state(&self) -> bool {
        self.dual_switch.state()
    }

    pub fn is_dual_sensitive(&self) -> bool {
        self.dual_switch.is_sensitive()
    }

    pub fn is_dual_row_sensitive(&self) -> bool {
        self.dual_row.is_sensitive()
    }

    pub fn is_in_ear_active(&self) -> bool {
        self.in_ear_switch.is_active()
    }

    pub fn is_in_ear_state(&self) -> bool {
        self.in_ear_switch.state()
    }

    pub fn is_in_ear_sensitive(&self) -> bool {
        self.in_ear_switch.is_sensitive()
    }

    pub fn is_sleep_active(&self) -> bool {
        self.sleep_switch.is_active()
    }

    pub fn is_sleep_state(&self) -> bool {
        self.sleep_switch.state()
    }

    pub fn is_sleep_sensitive(&self) -> bool {
        self.sleep_switch.is_sensitive()
    }

    pub fn set_preferred_selected_id(&self, id: Option<u8>) {
        let _guard = self.preferred_flag.enter();
        self.preferred_row.set_selected_id(id);
    }
}

pub(crate) fn build_features_group() -> (adw::PreferencesGroup, FeatureTogglesWidgets) {
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

    let widgets = FeatureTogglesWidgets {
        features_group: features_group.clone(),
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
        dual_device_flag: crate::ui::ReentrancyFlag::new(),
        in_ear_flag: crate::ui::ReentrancyFlag::new(),
        sleep_flag: crate::ui::ReentrancyFlag::new(),
        preferred_flag: crate::ui::ReentrancyFlag::new(),
        prompt_volume_flag: crate::ui::ReentrancyFlag::new(),
    };

    (features_group, widgets)
}
