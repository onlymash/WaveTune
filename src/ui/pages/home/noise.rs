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
use gtk::{Align, Box as GtkBox, Button, Label, Orientation};

use crate::{
    compatibility::{self, NoiseMode, NoiseModeKind},
    ui::{
        ReentrancyFlag,
        components::{ComboRowExtManual, combo_row, preferences_content_row},
    },
};

#[derive(Clone)]
pub(crate) struct PrimaryNoiseControl {
    pub(crate) index: u8,
    pub(crate) container: GtkBox,
    pub(crate) button: gtk::Button,
    pub(crate) label: Label,
}

#[derive(Clone, Copy)]
pub(crate) struct NoiseControlsViewState<'a> {
    pub supported_indices: &'a [u8],
    #[allow(dead_code)]
    pub support_note: &'a str,
    pub capability_enabled: bool,
    pub supports_wear_detection: bool,
    pub wear: &'a crate::protocol::WearState,
    pub current_mode: Option<u8>,
    pub modes: &'a [NoiseMode],
}

#[derive(Clone)]
pub(crate) struct QuickNoiseWidgets {
    mode_group: adw::PreferencesGroup,
    primary_noise_controls: Vec<PrimaryNoiseControl>,
    noise_option_row: adw::ComboRow,
    noise_option_flag: ReentrancyFlag,
}

impl QuickNoiseWidgets {
    pub fn render(&self, state: &NoiseControlsViewState) {
        let worn = !state.supports_wear_detection || state.wear.any_earbud_in_ear();
        self.refresh_controls(
            state.supported_indices,
            state.capability_enabled && worn,
            state.current_mode,
            state.modes,
        );
    }

    pub fn set_controls_sensitive(&self, sensitive: bool) {
        for control in &self.primary_noise_controls {
            control.button.set_sensitive(sensitive);
        }
        self.noise_option_row.set_sensitive(sensitive);
    }

    pub fn set_visible(&self, visible: bool) {
        self.mode_group.set_visible(visible);
    }

    pub fn connect_mode_clicked<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static + Clone,
    {
        for control in &self.primary_noise_controls {
            let protocol_index = control.index;
            let cb = callback.clone();
            control.button.connect_clicked(move |_| {
                cb(protocol_index);
            });
        }
    }

    pub fn connect_option_selected<F>(&self, callback: F)
    where
        F: Fn(u8) + 'static,
    {
        let flag = self.noise_option_flag.clone();
        self.noise_option_row.connect_selected_notify(move |row| {
            if flag.is_updating() {
                return;
            }
            if let Some(mode) = row.selected_id() {
                callback(mode);
            }
        });
    }

    fn refresh_controls(
        &self,
        supported_indices: &[u8],
        enabled: bool,
        current_mode: Option<u8>,
        modes: &[NoiseMode],
    ) {
        let current_group =
            current_mode.and_then(|index| compatibility::noise_mode_group_for(modes, index));
        let mut any_noise_mode_supported = false;

        for control in &self.primary_noise_controls {
            let index = control.index;
            let mode = compatibility::noise_mode_for(modes, index);
            let label_text = mode.map_or_else(
                || crate::t!("noise-mode-numbered", index),
                |mode| crate::t!(mode.kind.as_str()),
            );
            control.label.set_text(&label_text);
            control.button.set_tooltip_text(Some(&label_text));
            control.label.set_tooltip_text(Some(&label_text));
            if let Some(mode) = mode {
                let icon_name = noise_mode_icon_name(mode.kind);
                if let Some(child) = control.button.child() {
                    if let Some(img) = child.downcast_ref::<gtk::Image>() {
                        img.set_icon_name(Some(icon_name));
                    }
                }
            }
            let visible = mode.is_some_and(|mode| {
                mode.parent.is_none()
                    && compatibility::noise_mode_supported_for(index, supported_indices, modes)
            });
            control.container.set_visible(visible);
            if visible {
                any_noise_mode_supported = true;
            }
            let sensitive =
                enabled && visible && compatibility::noise_mode_for(modes, index).is_some();
            control.button.set_sensitive(sensitive);
            control.label.set_sensitive(sensitive);
            if current_group == Some(index) {
                control.button.add_css_class("suggested-action");
            } else {
                control.button.remove_css_class("suggested-action");
            }
        }

        self.mode_group.set_visible(any_noise_mode_supported);

        let visible_submodes: Vec<_> = current_group.map_or_else(Vec::new, |group| {
            modes
                .iter()
                .filter(|mode| {
                    mode.parent == Some(group)
                        && compatibility::noise_mode_supported_for(
                            mode.index,
                            supported_indices,
                            modes,
                        )
                })
                .collect()
        });
        let has_active_subgroup = visible_submodes.len() > 1;

        let _guard = self.noise_option_flag.enter();
        let prev_active = current_mode;
        self.noise_option_row.clear_items();

        if has_active_subgroup {
            if let Some(group) = current_group {
                self.noise_option_row
                    .set_title(&noise_options_title_for(group, modes));
            }
            let items: Vec<(u8, String)> = visible_submodes
                .iter()
                .map(|mode| (mode.index, crate::t!(mode.kind.as_str())))
                .collect();
            self.noise_option_row.set_items(&items);
            if let Some(id) = prev_active {
                self.noise_option_row.set_selected_id(Some(id));
            }
            if self.noise_option_row.selected_id().is_none() && !visible_submodes.is_empty() {
                self.noise_option_row.set_selected(0);
            }
            self.noise_option_row.set_sensitive(enabled);
            self.noise_option_row.set_visible(true);
        } else {
            self.noise_option_row.set_visible(false);
            self.noise_option_row.set_sensitive(false);
        }
    }
}

pub(crate) fn noise_mode_icon_name(kind: NoiseModeKind) -> &'static str {
    match kind {
        NoiseModeKind::Off => "audio-volume-muted-symbolic",
        NoiseModeKind::NoiseCancellation => "audio-headphones-symbolic",
        NoiseModeKind::Transparency => "audio-speakers-symbolic",
        NoiseModeKind::Adaptive => "starred-symbolic",
        NoiseModeKind::Smart => "weather-clear-symbolic",
        NoiseModeKind::Deep => "security-high-symbolic",
        NoiseModeKind::Moderate => "security-medium-symbolic",
        NoiseModeKind::Mild => "security-low-symbolic",
        NoiseModeKind::Standard => "audio-volume-medium-symbolic",
        NoiseModeKind::LessNoise => "audio-volume-low-symbolic",
        NoiseModeKind::MoreNoise => "audio-volume-high-symbolic",
        NoiseModeKind::VocalEnhancement => "audio-input-microphone-symbolic",
    }
}

pub(crate) fn noise_options_title_for(group: u8, modes: &[NoiseMode]) -> String {
    match group {
        1 => crate::t!("noise-mode-level"),
        11 => crate::t!("noise-mode-adaptive-preference"),
        2 => crate::t!("noise-mode-transparency-title"),
        _ => compatibility::noise_mode_for(modes, group)
            .map(|mode| crate::t!("common-options", crate::t!(mode.kind.as_str())))
            .unwrap_or_else(|| crate::t!("noise-mode-options")),
    }
}

pub(crate) fn build_quick_noise() -> (adw::PreferencesGroup, QuickNoiseWidgets) {
    let mode_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("noise-mode-title"))
        .build();
    mode_group.set_visible(false);

    let primary_modes = GtkBox::new(Orientation::Horizontal, 8);
    primary_modes.set_homogeneous(true);
    primary_modes.set_hexpand(true);
    primary_modes.set_halign(Align::Fill);
    primary_modes.set_valign(Align::Start);
    primary_modes.set_margin_top(14);
    primary_modes.set_margin_bottom(14);
    primary_modes.set_margin_start(12);
    primary_modes.set_margin_end(12);

    let mut primary_noise_controls = Vec::new();
    for index in compatibility::catalog_noise_indices() {
        let item_box = GtkBox::new(Orientation::Vertical, 6);
        item_box.set_halign(Align::Center);
        item_box.set_valign(Align::Start);
        item_box.set_hexpand(true);
        item_box.set_vexpand(false);
        item_box.set_visible(false);

        let button = Button::builder()
            .css_classes(["circular", "noise-mode-button"])
            .valign(Align::Start)
            .halign(Align::Center)
            .sensitive(false)
            .build();
        let icon = gtk::Image::new();
        icon.set_pixel_size(22);
        button.set_child(Some(&icon));

        let label = Label::builder()
            .label(&crate::t!("noise-mode-numbered", index))
            .css_classes(["caption", "noise-mode-label"])
            .halign(Align::Center)
            .valign(Align::Start)
            .justify(gtk::Justification::Center)
            .xalign(0.5)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .max_width_chars(14)
            .width_chars(1)
            .sensitive(false)
            .build();

        let click_gesture = gtk::GestureClick::new();
        let btn_clone = button.clone();
        click_gesture.connect_released(move |_, _, _, _| {
            if btn_clone.is_sensitive() {
                btn_clone.emit_clicked();
            }
        });
        label.add_controller(click_gesture);

        item_box.append(&button);
        item_box.append(&label);

        primary_noise_controls.push(PrimaryNoiseControl {
            index,
            container: item_box.clone(),
            button,
            label,
        });
        primary_modes.append(&item_box);
    }

    mode_group.add(&preferences_content_row(&primary_modes));

    let noise_option_row = combo_row(&crate::t!("noise-mode-level"), None);
    noise_option_row.set_visible(false);
    mode_group.add(&noise_option_row);

    let widgets = QuickNoiseWidgets {
        mode_group: mode_group.clone(),
        primary_noise_controls,
        noise_option_row,
        noise_option_flag: ReentrancyFlag::new(),
    };

    (mode_group, widgets)
}

#[cfg(test)]
#[allow(dead_code)]
impl QuickNoiseWidgets {
    pub fn mode_group(&self) -> &adw::PreferencesGroup {
        &self.mode_group
    }

    pub fn noise_option_row(&self) -> &adw::ComboRow {
        &self.noise_option_row
    }

    pub fn primary_noise_controls(&self) -> &[PrimaryNoiseControl] {
        &self.primary_noise_controls
    }

    pub fn reset(&self) {
        self.mode_group.set_visible(false);
        self.noise_option_row.set_visible(false);
        self.noise_option_row.set_sensitive(false);
        for control in &self.primary_noise_controls {
            control.container.set_visible(false);
            control.button.set_sensitive(false);
            control.label.set_sensitive(false);
            control.button.remove_css_class("suggested-action");
        }
    }
}
