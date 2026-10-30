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
use gtk::{Box as GtkBox, Button, Label, Switch};

use crate::{
    compatibility::{self, EqualizerPreset},
    protocol::{BatteryPart, BatteryState, CustomEqualizer, noise_mode_write_payload_for},
};

use crate::ui::{
    HomeWidgets,
    components::{BatteryIndicator, ComboRowExtManual},
    scene3d::DeviceSceneView,
};

#[derive(Debug)]
pub(crate) struct TurntableState {
    pub(crate) tick_id: Option<gtk::TickCallbackId>,
    pub(crate) is_dragging: bool,
    pub(crate) last_interaction: std::time::Instant,
    pub(crate) last_tick_time: Option<std::time::Instant>,
    pub(crate) accumulator: f64,
}

impl Default for TurntableState {
    fn default() -> Self {
        Self {
            tick_id: None,
            is_dragging: false,
            last_interaction: std::time::Instant::now(),
            last_tick_time: None,
            accumulator: 0.0,
        }
    }
}

pub(crate) struct HomeWidgetRefs {
    pub(crate) model_stack: gtk::Stack,
    pub(crate) model_picture: gtk::Picture,
    pub(crate) scene_view: DeviceSceneView,
    pub(crate) model_placeholder: gtk::Image,
    pub(crate) model_firmware: Label,
    pub(crate) model_card: GtkBox,
    pub(crate) active_turntable: std::rc::Rc<
        std::cell::RefCell<Option<std::rc::Rc<crate::resources::turntable::TurntableModel>>>,
    >,
    pub(crate) current_frame: std::rc::Rc<std::cell::Cell<usize>>,
    pub(crate) turntable_state: std::rc::Rc<std::cell::RefCell<TurntableState>>,
    pub(crate) device_name: Label,
    pub(crate) connection_tag: Label,
    pub(crate) device_menu_button: gtk::MenuButton,
    pub(crate) device_popover: gtk::Popover,
    pub(crate) device_list: gtk::ListBox,
    pub(crate) device_list_empty: Label,
    pub(crate) refresh_devices: Button,
    pub(crate) left_battery: BatteryIndicator,
    pub(crate) right_battery: BatteryIndicator,
    pub(crate) case_battery: BatteryIndicator,
    pub(crate) mode_group: adw::PreferencesGroup,
    pub(crate) noise_option_row: adw::ComboRow,
    pub(crate) paired_devices_section: GtkBox,
    pub(crate) paired_devices: gtk::ListBox,
    pub(crate) primary_noise_controls: Vec<super::PrimaryNoiseControl>,
    pub(crate) sound_card_section: GtkBox,
    pub(crate) eq_row: adw::ComboRow,
    pub(crate) spatial_row: adw::ComboRow,
    pub(crate) game_mode_section: adw::ActionRow,
    pub(crate) game_mode_switch: Switch,
}

pub(crate) fn widgets_ref(widgets: &HomeWidgets) -> HomeWidgetRefs {
    HomeWidgetRefs {
        model_stack: widgets.model_stack.clone(),
        model_picture: widgets.model_picture.clone(),
        scene_view: widgets.scene_view.clone(),
        model_placeholder: widgets.model_placeholder.clone(),
        model_firmware: widgets.model_firmware.clone(),
        model_card: widgets.model_card.clone(),
        active_turntable: std::rc::Rc::clone(&widgets.active_turntable),
        current_frame: std::rc::Rc::clone(&widgets.current_frame),
        turntable_state: std::rc::Rc::clone(&widgets.turntable_state),
        device_name: widgets.device_name.clone(),
        connection_tag: widgets.connection_tag.clone(),
        device_menu_button: widgets.device_menu_button.clone(),
        device_popover: widgets.device_popover.clone(),
        device_list: widgets.device_list.clone(),
        device_list_empty: widgets.device_list_empty.clone(),
        refresh_devices: widgets.refresh_devices.clone(),
        left_battery: widgets.left_battery.clone(),
        right_battery: widgets.right_battery.clone(),
        case_battery: widgets.case_battery.clone(),
        mode_group: widgets.mode_group.clone(),
        noise_option_row: widgets.noise_option_row.clone(),
        paired_devices_section: widgets.paired_devices_section.clone(),
        paired_devices: widgets.paired_devices.clone(),
        primary_noise_controls: widgets.primary_noise_controls.clone(),
        sound_card_section: widgets.sound_card_section.clone(),
        eq_row: widgets.eq_row.clone(),
        spatial_row: widgets.spatial_row.clone(),
        game_mode_section: widgets.game_mode_section.clone(),
        game_mode_switch: widgets.game_mode_switch.clone(),
    }
}

pub(crate) fn ensure_turntable_tick(
    model_card: &GtkBox,
    model_stack: &gtk::Stack,
    model_picture: &gtk::Picture,
    active_turntable: &std::rc::Rc<
        std::cell::RefCell<Option<std::rc::Rc<crate::resources::turntable::TurntableModel>>>,
    >,
    current_frame: &std::rc::Rc<std::cell::Cell<usize>>,
    turntable_state: &std::rc::Rc<std::cell::RefCell<TurntableState>>,
) {
    let mut st = turntable_state.borrow_mut();
    if st.tick_id.is_none() && active_turntable.borrow().is_some() {
        st.last_tick_time = Some(std::time::Instant::now());
        st.accumulator = 0.0;
        let active_turntable = std::rc::Rc::clone(active_turntable);
        let current_frame = std::rc::Rc::clone(current_frame);
        let turntable_state = std::rc::Rc::clone(turntable_state);
        let model_picture = model_picture.clone();
        let model_stack = model_stack.clone();

        let id = model_card.add_tick_callback(move |widget, _| {
            if !widget.is_mapped() || model_stack.visible_child_name().as_deref() != Some("picture")
            {
                turntable_state.borrow_mut().tick_id = None;
                return gtk::glib::ControlFlow::Break;
            }

            let tt_guard = active_turntable.borrow();
            let Some(ref tt) = *tt_guard else {
                turntable_state.borrow_mut().tick_id = None;
                return gtk::glib::ControlFlow::Break;
            };

            let total = tt.frame_count();
            if total == 0 {
                turntable_state.borrow_mut().tick_id = None;
                return gtk::glib::ControlFlow::Break;
            }

            let now = std::time::Instant::now();
            let mut st = turntable_state.borrow_mut();
            let dt = st
                .last_tick_time
                .map_or(0.016, |prev| now.duration_since(prev).as_secs_f64())
                .min(0.2);
            st.last_tick_time = Some(now);

            if st.is_dragging {
                st.accumulator = 0.0;
                return gtk::glib::ControlFlow::Continue;
            }

            if now.duration_since(st.last_interaction).as_millis() < 1200 {
                st.accumulator = 0.0;
                return gtk::glib::ControlFlow::Continue;
            }

            let duration_ms = tt.frame_duration_ms(current_frame.get());
            let frame_duration = (duration_ms as f64 / 1000.0).max(0.025);
            st.accumulator += dt;
            if st.accumulator >= frame_duration {
                let steps = (st.accumulator / frame_duration).floor() as usize;
                st.accumulator %= frame_duration;
                // Reverse rotation direction to match natural turntable spin
                let next = (current_frame.get() as isize - steps as isize)
                    .rem_euclid(total as isize) as usize;
                current_frame.set(next);
                if let Some(tex) = tt.get_texture(next) {
                    model_picture.set_paintable(Some(&tex));
                }
                // Pre-warm upcoming frame in cache to eliminate decode lag
                let upcoming = (next as isize - 1).rem_euclid(total as isize) as usize;
                let _ = tt.get_texture(upcoming);
            }

            gtk::glib::ControlFlow::Continue
        });

        st.tick_id = Some(id);
    }
}

pub(crate) fn stop_turntable_tick(
    turntable_state: &std::rc::Rc<std::cell::RefCell<TurntableState>>,
) {
    let mut st = turntable_state.borrow_mut();
    if let Some(id) = st.tick_id.take() {
        id.remove();
    }
    st.last_tick_time = None;
    st.accumulator = 0.0;
}

impl HomeWidgetRefs {
    pub(crate) fn set_turntable(
        &self,
        turntable: std::rc::Rc<crate::resources::turntable::TurntableModel>,
    ) {
        if let Some(tex) = turntable.get_texture(0) {
            self.model_picture.set_paintable(Some(&tex));
            *self.active_turntable.borrow_mut() = Some(turntable);
            self.current_frame.set(0);
            self.model_card.set_cursor_from_name(Some("grab"));
            self.model_stack.set_visible_child_name("picture");
            ensure_turntable_tick(
                &self.model_card,
                &self.model_stack,
                &self.model_picture,
                &self.active_turntable,
                &self.current_frame,
                &self.turntable_state,
            );
        }
    }

    pub(crate) fn clear_turntable(&self) {
        *self.active_turntable.borrow_mut() = None;
        stop_turntable_tick(&self.turntable_state);
        self.model_card.set_cursor(None::<&gtk::gdk::Cursor>);
    }
}

pub(crate) fn noise_mode_icon_name(kind: compatibility::NoiseModeKind) -> &'static str {
    match kind {
        compatibility::NoiseModeKind::Off => "audio-volume-muted-symbolic",
        compatibility::NoiseModeKind::NoiseCancellation => "audio-headphones-symbolic",
        compatibility::NoiseModeKind::Transparency => "audio-speakers-symbolic",
        compatibility::NoiseModeKind::Adaptive => "starred-symbolic",
        compatibility::NoiseModeKind::Smart => "weather-clear-symbolic",
        compatibility::NoiseModeKind::Deep => "security-high-symbolic",
        compatibility::NoiseModeKind::Moderate => "security-medium-symbolic",
        compatibility::NoiseModeKind::Mild => "security-low-symbolic",
        compatibility::NoiseModeKind::Standard => "audio-volume-medium-symbolic",
        compatibility::NoiseModeKind::LessNoise => "audio-volume-low-symbolic",
        compatibility::NoiseModeKind::MoreNoise => "audio-volume-high-symbolic",
        compatibility::NoiseModeKind::VocalEnhancement => "audio-input-microphone-symbolic",
    }
}

pub(crate) fn noise_options_title_for(group: u8, modes: &[compatibility::NoiseMode]) -> String {
    match group {
        1 => crate::t!("noise-mode-level"),
        11 => crate::t!("noise-mode-adaptive-preference"),
        2 => crate::t!("noise-mode-transparency-title"),
        _ => compatibility::noise_mode_for(modes, group)
            .map(|mode| crate::t!("common-options", crate::t!(mode.kind.as_str())))
            .unwrap_or_else(|| crate::t!("noise-mode-options")),
    }
}

fn refresh_noise_controls(
    widgets: &HomeWidgetRefs,
    supported_indices: &[u8],
    enabled: bool,
    current_mode: Option<u8>,
    modes: &[compatibility::NoiseMode],
    updating_noise_option: &std::cell::Cell<bool>,
) {
    let current_group =
        current_mode.and_then(|index| compatibility::noise_mode_group_for(modes, index));
    let mut any_noise_mode_supported = false;
    for control in &widgets.primary_noise_controls {
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
        let sensitive = enabled && visible && noise_mode_write_payload_for(index, modes).is_some();
        control.button.set_sensitive(sensitive);
        control.label.set_sensitive(sensitive);
        if current_group == Some(index) {
            control.button.add_css_class("suggested-action");
        } else {
            control.button.remove_css_class("suggested-action");
        }
    }

    widgets.mode_group.set_visible(any_noise_mode_supported);

    let visible_submodes: Vec<_> = current_group.map_or_else(Vec::new, |group| {
        modes
            .iter()
            .filter(|mode| {
                mode.parent == Some(group)
                    && compatibility::noise_mode_supported_for(mode.index, supported_indices, modes)
            })
            .collect()
    });
    let has_active_subgroup = visible_submodes.len() > 1;

    updating_noise_option.set(true);
    let prev_active = current_mode;
    widgets.noise_option_row.clear_items();

    if has_active_subgroup {
        if let Some(group) = current_group {
            widgets
                .noise_option_row
                .set_title(&noise_options_title_for(group, modes));
        }
        let items: Vec<(u8, String)> = visible_submodes
            .iter()
            .map(|mode| (mode.index, crate::t!(mode.kind.as_str())))
            .collect();
        widgets.noise_option_row.set_items(&items);
        if let Some(id) = prev_active {
            widgets.noise_option_row.set_selected_id(Some(id));
        }
        if widgets.noise_option_row.selected_id().is_none() && !visible_submodes.is_empty() {
            widgets.noise_option_row.set_selected(0);
        }
        widgets.noise_option_row.set_sensitive(enabled);
        widgets.noise_option_row.set_visible(true);
    } else {
        widgets.noise_option_row.set_visible(false);
        widgets.noise_option_row.set_sensitive(false);
    }
    updating_noise_option.set(false);
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
    pub modes: &'a [compatibility::NoiseMode],
}

pub(crate) fn update_noise_controls(
    widgets: &HomeWidgetRefs,
    state: &NoiseControlsViewState,
    updating_noise_option: &std::cell::Cell<bool>,
) {
    let worn = !state.supports_wear_detection || state.wear.any_earbud_in_ear();
    refresh_noise_controls(
        widgets,
        state.supported_indices,
        state.capability_enabled && worn,
        state.current_mode,
        state.modes,
        updating_noise_option,
    );
}

pub(crate) fn merge_battery(current: &mut BatteryState, update: BatteryState) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case.is_some() {
        current.case = update.case;
    }
}

pub(crate) fn merge_wear(
    current: &mut crate::protocol::WearState,
    update: crate::protocol::WearState,
) {
    if update.left.is_some() {
        current.left = update.left;
    }
    if update.right.is_some() {
        current.right = update.right;
    }
    if update.case_open.is_some() {
        current.case_open = update.case_open;
    }
}

pub(crate) fn set_battery(indicator: &BatteryIndicator, part: Option<BatteryPart>, stale: bool) {
    if let Some(part) = part {
        indicator
            .percentage
            .set_text(&format!("{}%", part.percentage));
        indicator.charging.set_visible(part.charging && !stale);
        let stale_tooltip = crate::t!("battery-last-read");
        indicator
            .percentage
            .set_tooltip_text(stale.then_some(stale_tooltip.as_str()));
    } else {
        indicator.percentage.set_text("—");
        indicator.percentage.set_tooltip_text(None);
        indicator.charging.set_visible(false);
    }
}

#[allow(dead_code)]
pub(crate) fn noise_mode_name(index: u8, modes: &[compatibility::NoiseMode]) -> String {
    compatibility::noise_mode_for(modes, index)
        .map(|mode| crate::t!(mode.kind.as_str()))
        .unwrap_or_else(|| crate::t!("noise-mode-protocol-mode", index))
}

pub(crate) fn wear_part_name(name: &str, part: Option<crate::protocol::WearPart>) -> String {
    match part {
        Some(part) if part.in_ear => crate::t!("wear-removed-in-ear", name),
        Some(part) if part.out_of_case => crate::t!("wear-removed-out-of-ear", name),
        Some(_) => crate::t!("wear-in-case-status", name),
        None => crate::t!("wear-status-unknown", name),
    }
}

pub(crate) fn update_equalizer_items(
    widgets: &HomeWidgetRefs,
    supports_equalizer: bool,
    presets: &[EqualizerPreset],
    custom_equalizers: &[CustomEqualizer],
    selected_id: Option<u8>,
    updating_eq: &std::cell::Cell<bool>,
) {
    widgets.eq_row.set_visible(supports_equalizer);
    updating_eq.set(true);
    let prev_eq = selected_id.or_else(|| widgets.eq_row.selected_id());
    widgets.eq_row.clear_items();
    let mut eq_items: Vec<(u8, String)> = presets
        .iter()
        .map(|preset| (preset.index, crate::t!(preset.key)))
        .collect();
    for custom in custom_equalizers {
        eq_items.push((custom.id, custom.name.clone()));
    }
    widgets.eq_row.set_items(&eq_items);
    if let Some(id) = prev_eq {
        widgets.eq_row.set_selected_id(Some(id));
    }
    widgets
        .eq_row
        .set_sensitive(supports_equalizer && !eq_items.is_empty());
    updating_eq.set(false);
}

#[derive(Clone, Copy)]
pub(crate) struct HomeSoundFeaturesViewState<'a> {
    pub supports_equalizer: bool,
    pub supports_spatial_audio: bool,
    pub supports_head_tracking: bool,
    pub supports_game_mode: bool,
    pub presets: &'a [EqualizerPreset],
    pub custom_equalizers: &'a [CustomEqualizer],
}

#[derive(Clone, Copy)]
pub(crate) struct HomeSoundUpdateFlags<'a> {
    pub updating_eq: &'a std::cell::Cell<bool>,
    pub updating_spatial: &'a std::cell::Cell<bool>,
    pub updating_game_mode: &'a std::cell::Cell<bool>,
}

pub(crate) fn update_sound_features(
    widgets: &HomeWidgetRefs,
    state: &HomeSoundFeaturesViewState,
    flags: &HomeSoundUpdateFlags,
) {
    let any_supported =
        state.supports_equalizer || state.supports_spatial_audio || state.supports_game_mode;
    widgets.sound_card_section.set_visible(any_supported);

    update_equalizer_items(
        widgets,
        state.supports_equalizer,
        state.presets,
        state.custom_equalizers,
        None,
        flags.updating_eq,
    );

    widgets
        .spatial_row
        .set_visible(state.supports_spatial_audio);
    flags.updating_spatial.set(true);
    let prev_spatial = widgets.spatial_row.selected_id();
    widgets.spatial_row.clear_items();
    let supported_modes = compatibility::spatial_audio_modes_for_profile(
        state.supports_spatial_audio,
        state.supports_head_tracking,
    );
    let spatial_items: Vec<(u8, String)> = supported_modes
        .iter()
        .map(|(mode, label)| (*mode, crate::t!(*label)))
        .collect();
    widgets.spatial_row.set_items(&spatial_items);
    if let Some(id) = prev_spatial {
        widgets.spatial_row.set_selected_id(Some(id));
    }
    widgets
        .spatial_row
        .set_sensitive(state.supports_spatial_audio && !supported_modes.is_empty());
    flags.updating_spatial.set(false);

    widgets
        .game_mode_section
        .set_visible(state.supports_game_mode);
    if !state.supports_game_mode {
        flags.updating_game_mode.set(true);
        widgets.game_mode_switch.set_state(false);
        widgets.game_mode_switch.set_active(false);
        widgets.game_mode_switch.set_sensitive(false);
        flags.updating_game_mode.set(false);
    }
}

pub(crate) fn update_equalizer(
    widgets: &HomeWidgetRefs,
    current_preset: Option<u8>,
    updating_eq: &std::cell::Cell<bool>,
) {
    updating_eq.set(true);
    widgets.eq_row.set_selected_id(current_preset);
    updating_eq.set(false);
}

pub(crate) fn update_spatial_audio(
    widgets: &HomeWidgetRefs,
    current_mode: Option<u8>,
    updating_spatial: &std::cell::Cell<bool>,
) {
    updating_spatial.set(true);
    widgets.spatial_row.set_selected_id(current_mode);
    updating_spatial.set(false);
}

pub(crate) fn update_game_mode(
    widgets: &HomeWidgetRefs,
    enabled: Option<bool>,
    updating_switch: &std::cell::Cell<bool>,
) {
    updating_switch.set(true);
    if let Some(enabled) = enabled {
        widgets.game_mode_switch.set_state(enabled);
        widgets.game_mode_switch.set_active(enabled);
        widgets.game_mode_switch.set_sensitive(true);
    } else {
        widgets.game_mode_switch.set_state(false);
        widgets.game_mode_switch.set_active(false);
        widgets.game_mode_switch.set_sensitive(false);
    }
    updating_switch.set(false);
}

pub(crate) fn reset_sound_controls(widgets: &HomeWidgetRefs, flags: &HomeSoundUpdateFlags) {
    widgets.sound_card_section.set_visible(true);
    widgets.eq_row.set_visible(true);
    widgets.spatial_row.set_visible(true);
    widgets.game_mode_section.set_visible(true);
    flags.updating_eq.set(true);
    widgets.eq_row.clear_items();
    widgets.eq_row.set_sensitive(false);
    flags.updating_eq.set(false);

    flags.updating_spatial.set(true);
    widgets.spatial_row.clear_items();
    widgets.spatial_row.set_sensitive(false);
    flags.updating_spatial.set(false);

    flags.updating_game_mode.set(true);
    widgets.game_mode_switch.set_state(false);
    widgets.game_mode_switch.set_active(false);
    widgets.game_mode_switch.set_sensitive(false);
    flags.updating_game_mode.set(false);
}
