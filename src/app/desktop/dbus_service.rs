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

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gtk::{gio, glib, prelude::*};

use crate::{
    compatibility::{self, NoiseMode, NoiseModeKind},
    protocol::{BatteryPart, BatteryState, WearState, noise_mode_write_payload_for},
    ui::home_state::{noise_mode_icon_name, noise_options_title_for},
};

pub const QUICK_SETTINGS_OBJECT_PATH: &str = "/io/github/onlymash/WaveTune/QuickSettings";
pub const QUICK_SETTINGS_INTERFACE: &str = "io.github.onlymash.WaveTune.QuickSettings";

const INTROSPECTION_XML: &str = r#"
<node>
  <interface name="io.github.onlymash.WaveTune.QuickSettings">
    <property name="Connected" type="b" access="read"/>
    <property name="DeviceName" type="s" access="read"/>
    <property name="ControlsEnabled" type="b" access="read"/>
    <property name="StatusHint" type="s" access="read"/>
    <property name="ActiveGroupIndex" type="i" access="read"/>
    <property name="ActiveModeIndex" type="i" access="read"/>
    <property name="ActiveModeLabel" type="s" access="read"/>
    <property name="ActiveModeIcon" type="s" access="read"/>
    <property name="NoiseControlActive" type="b" access="read"/>
    <property name="PrimaryModes" type="a(iss)" access="read"/>
    <property name="SubModesTitle" type="s" access="read"/>
    <property name="SubModes" type="a(is)" access="read"/>
    <property name="Battery" type="(ibibib)" access="read"/>
    <property name="BatterySummary" type="s" access="read"/>
    <property name="OpenAppLabel" type="s" access="read"/>
    <method name="ToggleNoiseControl"/>
    <method name="SetNoiseMode">
      <arg name="index" type="i" direction="in"/>
    </method>
    <method name="PresentWindow"/>
  </interface>
</node>
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuickSettingsSnapshot {
    pub(crate) connected: bool,
    pub(crate) device_name: String,
    pub(crate) controls_enabled: bool,
    pub(crate) status_hint: String,
    pub(crate) active_group_index: i32,
    pub(crate) active_mode_index: i32,
    pub(crate) active_mode_label: String,
    pub(crate) active_mode_icon: String,
    pub(crate) noise_control_active: bool,
    pub(crate) primary_modes: Vec<(i32, String, String)>,
    pub(crate) sub_modes_title: String,
    pub(crate) sub_modes: Vec<(i32, String)>,
    pub(crate) battery: (i32, bool, i32, bool, i32, bool),
    pub(crate) battery_summary: String,
    pub(crate) open_app_label: String,
}

impl Default for QuickSettingsSnapshot {
    fn default() -> Self {
        Self {
            connected: false,
            device_name: String::new(),
            controls_enabled: false,
            status_hint: String::new(),
            active_group_index: -1,
            active_mode_index: -1,
            active_mode_label: String::new(),
            active_mode_icon: "audio-headphones-symbolic".to_string(),
            noise_control_active: false,
            primary_modes: Vec::new(),
            sub_modes_title: String::new(),
            sub_modes: Vec::new(),
            battery: (-1, false, -1, false, -1, false),
            battery_summary: String::new(),
            open_app_label: crate::t!("qs-open-app"),
        }
    }
}

impl QuickSettingsSnapshot {
    fn property_variant(&self, name: &str) -> Option<glib::Variant> {
        match name {
            "Connected" => Some(self.connected.to_variant()),
            "DeviceName" => Some(self.device_name.to_variant()),
            "ControlsEnabled" => Some(self.controls_enabled.to_variant()),
            "StatusHint" => Some(self.status_hint.to_variant()),
            "ActiveGroupIndex" => Some(self.active_group_index.to_variant()),
            "ActiveModeIndex" => Some(self.active_mode_index.to_variant()),
            "ActiveModeLabel" => Some(self.active_mode_label.to_variant()),
            "ActiveModeIcon" => Some(self.active_mode_icon.to_variant()),
            "NoiseControlActive" => Some(self.noise_control_active.to_variant()),
            "PrimaryModes" => Some(self.primary_modes.to_variant()),
            "SubModesTitle" => Some(self.sub_modes_title.to_variant()),
            "SubModes" => Some(self.sub_modes.to_variant()),
            "Battery" => Some(self.battery.to_variant()),
            "BatterySummary" => Some(self.battery_summary.to_variant()),
            "OpenAppLabel" => Some(self.open_app_label.to_variant()),
            _ => None,
        }
    }

    fn changed_properties_since(
        &self,
        prev: &QuickSettingsSnapshot,
    ) -> HashMap<&'static str, glib::Variant> {
        let mut changed = HashMap::new();
        if self.connected != prev.connected {
            changed.insert("Connected", self.connected.to_variant());
        }
        if self.device_name != prev.device_name {
            changed.insert("DeviceName", self.device_name.to_variant());
        }
        if self.controls_enabled != prev.controls_enabled {
            changed.insert("ControlsEnabled", self.controls_enabled.to_variant());
        }
        if self.status_hint != prev.status_hint {
            changed.insert("StatusHint", self.status_hint.to_variant());
        }
        if self.active_group_index != prev.active_group_index {
            changed.insert("ActiveGroupIndex", self.active_group_index.to_variant());
        }
        if self.active_mode_index != prev.active_mode_index {
            changed.insert("ActiveModeIndex", self.active_mode_index.to_variant());
        }
        if self.active_mode_label != prev.active_mode_label {
            changed.insert("ActiveModeLabel", self.active_mode_label.to_variant());
        }
        if self.active_mode_icon != prev.active_mode_icon {
            changed.insert("ActiveModeIcon", self.active_mode_icon.to_variant());
        }
        if self.noise_control_active != prev.noise_control_active {
            changed.insert("NoiseControlActive", self.noise_control_active.to_variant());
        }
        if self.primary_modes != prev.primary_modes {
            changed.insert("PrimaryModes", self.primary_modes.to_variant());
        }
        if self.sub_modes_title != prev.sub_modes_title {
            changed.insert("SubModesTitle", self.sub_modes_title.to_variant());
        }
        if self.sub_modes != prev.sub_modes {
            changed.insert("SubModes", self.sub_modes.to_variant());
        }
        if self.battery != prev.battery {
            changed.insert("Battery", self.battery.to_variant());
        }
        if self.battery_summary != prev.battery_summary {
            changed.insert("BatterySummary", self.battery_summary.to_variant());
        }
        if self.open_app_label != prev.open_app_label {
            changed.insert("OpenAppLabel", self.open_app_label.to_variant());
        }
        changed
    }
}

fn format_battery_part(label: &str, part: Option<BatteryPart>, stale: bool) -> Option<String> {
    let part = part?;
    let charging_mark = if part.charging && !stale { "⚡" } else { "" };
    Some(format!("{label} {}%{charging_mark}", part.percentage))
}

fn format_battery_summary(battery: &BatteryState, case_stale: bool) -> String {
    let mut segments = Vec::with_capacity(3);
    if let Some(left) = format_battery_part(&crate::t!("battery-left-ear"), battery.left, false) {
        segments.push(left);
    }
    if let Some(right) = format_battery_part(&crate::t!("battery-right-ear"), battery.right, false)
    {
        segments.push(right);
    }
    if let Some(case) = format_battery_part(&crate::t!("battery-case"), battery.case, case_stale) {
        segments.push(case);
    }
    segments.join(" · ")
}

fn format_active_mode_label(current_mode: Option<u8>, modes: &[NoiseMode]) -> String {
    let Some(mode_idx) = current_mode else {
        return String::new();
    };
    let Some(mode) = compatibility::noise_mode_for(modes, mode_idx) else {
        return crate::t!("noise-mode-numbered", mode_idx);
    };
    let mode_text = crate::t!(mode.kind.as_str());
    if let Some(group_idx) = compatibility::noise_mode_group_for(modes, mode_idx)
        && group_idx != mode_idx
        && let Some(group_mode) = compatibility::noise_mode_for(modes, group_idx)
    {
        let group_text = crate::t!(group_mode.kind.as_str());
        if mode_text != group_text && !mode_text.contains(&group_text) {
            return format!("{group_text} · {mode_text}");
        }
    }
    mode_text
}

#[derive(Clone, Copy)]
pub(crate) struct DesktopSnapshotInput<'a> {
    pub connected: bool,
    pub device_name: &'a str,
    pub noise_controls_enabled: bool,
    pub supports_wear_detection: bool,
    pub wear: &'a WearState,
    pub battery: &'a BatteryState,
    pub case_battery_stale: bool,
    pub current_noise_mode: Option<u8>,
    pub supported_indices: &'a [u8],
    pub modes: &'a [NoiseMode],
}

pub(crate) fn build_snapshot(input: &DesktopSnapshotInput) -> QuickSettingsSnapshot {
    if !input.connected {
        return QuickSettingsSnapshot::default();
    }

    let worn = !input.supports_wear_detection || input.wear.any_earbud_in_ear();
    let mut primary_modes = Vec::new();
    for index in compatibility::catalog_noise_indices() {
        if let Some(mode) = compatibility::noise_mode_for(input.modes, index)
            && mode.parent.is_none()
            && compatibility::noise_mode_supported_for(index, input.supported_indices, input.modes)
            && noise_mode_write_payload_for(index, input.modes).is_some()
        {
            let label = crate::t!(mode.kind.as_str());
            let icon = noise_mode_icon_name(mode.kind).to_string();
            primary_modes.push((i32::from(index), label, icon));
        }
    }

    let controls_enabled = input.noise_controls_enabled && worn && !primary_modes.is_empty();
    let status_hint = if controls_enabled {
        String::new()
    } else if input.modes.is_empty() && input.current_noise_mode.is_none() {
        crate::t!("qs-syncing")
    } else if !input.noise_controls_enabled || primary_modes.is_empty() {
        crate::t!("qs-unavailable")
    } else if !worn {
        crate::t!("qs-not-worn")
    } else {
        String::new()
    };

    let current_group = input
        .current_noise_mode
        .and_then(|idx| compatibility::noise_mode_group_for(input.modes, idx));
    let active_group_index = current_group.map(i32::from).unwrap_or(-1);
    let active_mode_index = input.current_noise_mode.map(i32::from).unwrap_or(-1);
    let active_mode_label = format_active_mode_label(input.current_noise_mode, input.modes);
    let active_mode_icon = current_group
        .and_then(|grp| compatibility::noise_mode_for(input.modes, grp))
        .map(|mode| noise_mode_icon_name(mode.kind))
        .unwrap_or("audio-headphones-symbolic")
        .to_string();

    let noise_control_active = current_group
        .and_then(|grp| compatibility::noise_mode_for(input.modes, grp))
        .is_some_and(|m| m.kind != NoiseModeKind::Off);

    let mut sub_modes = Vec::new();
    let mut sub_modes_title = String::new();
    if let Some(group) = current_group {
        let visible_submodes: Vec<_> = input
            .modes
            .iter()
            .filter(|mode| {
                mode.parent == Some(group)
                    && compatibility::noise_mode_supported_for(
                        mode.index,
                        input.supported_indices,
                        input.modes,
                    )
                    && noise_mode_write_payload_for(mode.index, input.modes).is_some()
            })
            .collect();
        if visible_submodes.len() > 1 {
            sub_modes_title = noise_options_title_for(group, input.modes);
            sub_modes = visible_submodes
                .into_iter()
                .map(|m| (i32::from(m.index), crate::t!(m.kind.as_str())))
                .collect();
        }
    }

    let pack_part = |part: Option<BatteryPart>, stale: bool| -> (i32, bool) {
        match part {
            Some(p) => (i32::from(p.percentage), p.charging && !stale),
            None => (-1, false),
        }
    };
    let (l_pct, l_chg) = pack_part(input.battery.left, false);
    let (r_pct, r_chg) = pack_part(input.battery.right, false);
    let (c_pct, c_chg) = pack_part(input.battery.case, input.case_battery_stale);

    QuickSettingsSnapshot {
        connected: true,
        device_name: input.device_name.to_string(),
        controls_enabled,
        status_hint,
        active_group_index,
        active_mode_index,
        active_mode_label,
        active_mode_icon,
        noise_control_active,
        primary_modes,
        sub_modes_title,
        sub_modes,
        battery: (l_pct, l_chg, r_pct, r_chg, c_pct, c_chg),
        battery_summary: format_battery_summary(input.battery, input.case_battery_stale),
        open_app_label: crate::t!("qs-open-app"),
    }
}

/// Resolves which protocol mode index `ToggleNoiseControl` should request next,
/// switching between the `Off` mode and the last confirmed non-`Off` mode (or the first
/// supported non-`Off` primary mode when no history exists yet).
pub(crate) fn resolve_toggle_target(
    current_noise_mode: Option<u8>,
    last_non_off_mode: Option<u8>,
    supported_indices: &[u8],
    modes: &[NoiseMode],
) -> Option<u8> {
    let off_mode = compatibility::catalog_noise_indices()
        .into_iter()
        .find(|&idx| {
            compatibility::noise_mode_for(modes, idx).is_some_and(|m| {
                m.parent.is_none()
                    && m.kind == NoiseModeKind::Off
                    && compatibility::noise_mode_supported_for(idx, supported_indices, modes)
                    && noise_mode_write_payload_for(idx, modes).is_some()
            })
        });

    let is_currently_non_off = current_noise_mode
        .and_then(|idx| compatibility::noise_mode_group_for(modes, idx))
        .and_then(|grp| compatibility::noise_mode_for(modes, grp))
        .is_some_and(|m| m.kind != NoiseModeKind::Off);

    if is_currently_non_off {
        return off_mode;
    }

    if let Some(saved) = last_non_off_mode
        && compatibility::noise_mode_supported_for(saved, supported_indices, modes)
        && noise_mode_write_payload_for(saved, modes).is_some()
    {
        return Some(saved);
    }

    compatibility::catalog_noise_indices()
        .into_iter()
        .find(|&idx| {
            compatibility::noise_mode_for(modes, idx).is_some_and(|m| {
                m.parent.is_none()
                    && m.kind != NoiseModeKind::Off
                    && compatibility::noise_mode_supported_for(idx, supported_indices, modes)
                    && noise_mode_write_payload_for(idx, modes).is_some()
            })
        })
}

pub(crate) fn is_non_off_writable_mode(index: u8, modes: &[NoiseMode]) -> bool {
    let group_non_off = compatibility::noise_mode_group_for(modes, index)
        .and_then(|grp| compatibility::noise_mode_for(modes, grp))
        .is_some_and(|m| m.kind != NoiseModeKind::Off);
    group_non_off && noise_mode_write_payload_for(index, modes).is_some()
}

type SetModeCallback = Rc<dyn Fn(u8) -> Result<(), String>>;
type ToggleCallback = Rc<dyn Fn() -> Result<(), String>>;
type PresentCallback = Rc<dyn Fn()>;

struct Inner {
    snapshot: QuickSettingsSnapshot,
    connection: Option<gio::DBusConnection>,
    _registration_id: Option<gio::RegistrationId>,
    on_set_mode: Option<SetModeCallback>,
    on_toggle: Option<ToggleCallback>,
    on_present: Option<PresentCallback>,
}

pub(crate) struct QuickSettingsService {
    inner: RefCell<Inner>,
}

impl QuickSettingsService {
    pub(crate) fn new(connection: Option<gio::DBusConnection>) -> Rc<Self> {
        let service = Rc::new(Self {
            inner: RefCell::new(Inner {
                snapshot: QuickSettingsSnapshot::default(),
                connection: connection.clone(),
                _registration_id: None,
                on_set_mode: None,
                on_toggle: None,
                on_present: None,
            }),
        });

        if let Some(conn) = connection {
            match gio::DBusNodeInfo::for_xml(INTROSPECTION_XML) {
                Ok(node_info) => {
                    if let Some(iface_info) = node_info.lookup_interface(QUICK_SETTINGS_INTERFACE) {
                        let svc_methods = Rc::clone(&service);
                        let svc_props = Rc::clone(&service);
                        match conn
                            .register_object(QUICK_SETTINGS_OBJECT_PATH, &iface_info)
                            .method_call(
                                move |_conn, _sender, _path, _iface, method, params, invocation| {
                                    svc_methods.handle_method_call(method, params, invocation);
                                },
                            )
                            .property(move |_conn, _sender, _path, _iface, prop_name| {
                                svc_props
                                    .inner
                                    .borrow()
                                    .snapshot
                                    .property_variant(prop_name)
                                    .unwrap_or_else(|| "".to_variant())
                            })
                            .build()
                        {
                            Ok(reg_id) => {
                                service.inner.borrow_mut()._registration_id = Some(reg_id);
                            }
                            Err(err) => {
                                eprintln!(
                                    "Failed to register D-Bus QuickSettings object at {QUICK_SETTINGS_OBJECT_PATH}: {err}"
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to parse QuickSettings D-Bus XML: {err}");
                }
            }
        }

        service
    }

    pub(crate) fn set_handlers(
        &self,
        on_set_mode: impl Fn(u8) -> Result<(), String> + 'static,
        on_toggle: impl Fn() -> Result<(), String> + 'static,
        on_present: impl Fn() + 'static,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner.on_set_mode = Some(Rc::new(on_set_mode));
        inner.on_toggle = Some(Rc::new(on_toggle));
        inner.on_present = Some(Rc::new(on_present));
    }

    pub(crate) fn update(&self, next: QuickSettingsSnapshot) {
        let (conn, changed) = {
            let mut inner = self.inner.borrow_mut();
            let changed = next.changed_properties_since(&inner.snapshot);
            if changed.is_empty() {
                return;
            }
            inner.snapshot = next;
            (inner.connection.clone(), changed)
        };

        if let Some(conn) = conn {
            let invalidated: [&str; 0] = [];
            let params = (QUICK_SETTINGS_INTERFACE, changed, &invalidated[..]).to_variant();
            let _ = conn.emit_signal(
                None,
                QUICK_SETTINGS_OBJECT_PATH,
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
                Some(&params),
            );
        }
    }

    fn handle_method_call(
        &self,
        method: &str,
        params: glib::Variant,
        invocation: gio::DBusMethodInvocation,
    ) {
        match method {
            "PresentWindow" => {
                let cb = self.inner.borrow().on_present.clone();
                if let Some(cb) = cb {
                    cb();
                }
                invocation.return_value(None);
            }
            "ToggleNoiseControl" => {
                let cb = self.inner.borrow().on_toggle.clone();
                let res = match cb {
                    Some(cb) => cb(),
                    None => Err(crate::t!("channel-not-connected")),
                };
                match res {
                    Ok(()) => invocation.return_value(None),
                    Err(msg) => invocation
                        .return_dbus_error("io.github.onlymash.WaveTune.Error.Failed", &msg),
                }
            }
            "SetNoiseMode" => {
                let Some((index,)) = params.get::<(i32,)>() else {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.InvalidArgs",
                        "Expected (i) mode index",
                    );
                    return;
                };
                let Ok(mode_u8) = u8::try_from(index) else {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.InvalidArgs",
                        "Mode index out of u8 range",
                    );
                    return;
                };
                let cb = self.inner.borrow().on_set_mode.clone();
                let res = match cb {
                    Some(cb) => cb(mode_u8),
                    None => Err(crate::t!("channel-not-connected")),
                };
                match res {
                    Ok(()) => invocation.return_value(None),
                    Err(msg) => invocation
                        .return_dbus_error("io.github.onlymash.WaveTune.Error.Failed", &msg),
                }
            }
            _ => {
                invocation.return_dbus_error(
                    "org.freedesktop.DBus.Error.UnknownMethod",
                    &format!("Unknown method: {method}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::WearPart;

    fn sample_wear(in_ear: bool) -> WearState {
        WearState {
            left: Some(WearPart {
                in_ear,
                out_of_case: true,
                raw_flags: 0,
            }),
            right: Some(WearPart {
                in_ear: false,
                out_of_case: true,
                raw_flags: 0,
            }),
            case_open: Some(true),
        }
    }

    #[test]
    fn snapshot_reflects_x4_modes_wear_gate_and_battery() {
        crate::i18n::set_locale("zh-CN");
        let profile = compatibility::profile_for_product_id(0x06ec10).unwrap();
        let modes = profile.noise_modes;
        let supported = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14];
        let battery = BatteryState {
            left: Some(BatteryPart {
                percentage: 80,
                charging: false,
            }),
            right: Some(BatteryPart {
                percentage: 75,
                charging: true,
            }),
            case: None,
        };

        // Worn + Deep ANC (index 4, group 1)
        let worn_wear = sample_wear(true);
        let snap = build_snapshot(&DesktopSnapshotInput {
            connected: true,
            device_name: "OPPO Enco X4",
            noise_controls_enabled: true,
            supports_wear_detection: true,
            wear: &worn_wear,
            battery: &battery,
            case_battery_stale: false,
            current_noise_mode: Some(4),
            supported_indices: &supported,
            modes,
        });
        assert!(snap.connected);
        assert_eq!(snap.device_name, "OPPO Enco X4");
        assert!(snap.controls_enabled);
        assert_eq!(snap.status_hint, "");
        assert_eq!(snap.active_group_index, 1);
        assert_eq!(snap.active_mode_index, 4);
        assert_eq!(snap.active_mode_label, "深度降噪");
        assert_eq!(snap.active_mode_icon, "audio-headphones-symbolic");
        assert!(snap.noise_control_active);
        assert_eq!(snap.primary_modes.len(), 4);
        assert_eq!(snap.sub_modes.len(), 4);
        assert_eq!(snap.battery, (80, false, 75, true, -1, false));
        assert_eq!(snap.battery_summary, "左耳 80% · 右耳 75%⚡");

        // Adaptive + Standard (index 13, group 11) combines group and submode label
        let adaptive_snap = build_snapshot(&DesktopSnapshotInput {
            connected: true,
            device_name: "OPPO Enco X4",
            noise_controls_enabled: true,
            supports_wear_detection: true,
            wear: &worn_wear,
            battery: &battery,
            case_battery_stale: false,
            current_noise_mode: Some(13),
            supported_indices: &supported,
            modes,
        });
        assert_eq!(adaptive_snap.active_mode_label, "自适应 · 标准");
        assert_eq!(adaptive_snap.active_mode_icon, "starred-symbolic");
        assert!(adaptive_snap.noise_control_active);

        // Out-of-ear disables controls and sets status_hint
        let unworn_wear = sample_wear(false);
        let unworn_snap = build_snapshot(&DesktopSnapshotInput {
            connected: true,
            device_name: "OPPO Enco X4",
            noise_controls_enabled: true,
            supports_wear_detection: true,
            wear: &unworn_wear,
            battery: &battery,
            case_battery_stale: false,
            current_noise_mode: Some(3),
            supported_indices: &supported,
            modes,
        });
        assert!(!unworn_snap.controls_enabled);
        assert_eq!(unworn_snap.status_hint, "未佩戴");
        assert!(!unworn_snap.noise_control_active);
        assert_eq!(unworn_snap.active_mode_label, "关闭");
        assert_eq!(unworn_snap.active_mode_icon, "audio-volume-muted-symbolic");

        crate::i18n::reset_locale();
    }

    #[test]
    fn toggle_target_switches_between_off_and_remembered_mode() {
        let modes = compatibility::profile_for_product_id(0x06ec10)
            .unwrap()
            .noise_modes;
        let supported = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14];

        // Active mode is Deep ANC (4) -> toggle turns Off (0)
        assert_eq!(
            resolve_toggle_target(Some(4), Some(4), &supported, modes),
            Some(0)
        );

        // Active mode is Off child (3) with remembered Transparency (2) -> restores 2
        assert_eq!(
            resolve_toggle_target(Some(3), Some(2), &supported, modes),
            Some(2)
        );

        // Active mode is Off child (3) with no history -> falls back to first non-Off primary mode (1)
        assert_eq!(
            resolve_toggle_target(Some(3), None, &supported, modes),
            Some(1)
        );
    }
}
