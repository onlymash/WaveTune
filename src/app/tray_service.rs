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

//! StatusNotifierItem (SNI) + DBusMenu system tray integration.
//!
//! The tray is implemented directly on the session `gio::DBusConnection` that is already used by
//! [`super::dbus_service::QuickSettingsService`], so it adds no dependency and shares the GTK main
//! thread with the rest of the application. GNOME Shell shows these items through the AppIndicator
//! and KStatusNotifierItem Support extension (<https://extensions.gnome.org/extension/615/>); KDE
//! Plasma, XFCE, Waybar, COSMIC, and other SNI-aware desktops consume the same protocol.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use gtk::{gio, glib, prelude::*};

use super::dbus_service::QuickSettingsSnapshot;

const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";
const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
const SNI_INTERFACE: &str = "org.kde.StatusNotifierItem";
const MENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// Default SNI object path expected by watchers when an application registers only its bus name.
pub(crate) const SNI_OBJECT_PATH: &str = "/StatusNotifierItem";
pub(crate) const MENU_OBJECT_PATH: &str = "/io/github/onlymash/WaveTune/Menu";

const TRAY_APP_ID: &str = "io.github.onlymash.WaveTune";
const TRAY_ICON_NAME: &str = "io.github.onlymash.WaveTune-symbolic";
const TRAY_TITLE: &str = "WaveTune";
const DBUS_TIMEOUT_MS: i32 = 5_000;
const DBUSMENU_PROTOCOL_VERSION: u32 = 3;

const ROOT_ID: i32 = 0;
const OPEN_ID: i32 = 1;
const STATUS_ID: i32 = 2;
const BATTERY_ID: i32 = 3;
const HINT_ID: i32 = 8;
const NOISE_SEPARATOR_ID: i32 = 4;
const NOISE_HEADER_ID: i32 = 5;
const SUB_MODES_SUBMENU_ID: i32 = 9;
const QUIT_SEPARATOR_ID: i32 = 6;
const QUIT_ID: i32 = 7;
const MODE_ID_BASE: i32 = 100;
const SUB_MODE_ID_BASE: i32 = 400;

const SNI_INTROSPECTION_XML: &str = r#"
<node>
  <interface name="org.kde.StatusNotifierItem">
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="WindowId" type="i" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconPixmap" type="a(iiay)" access="read"/>
    <property name="OverlayIconName" type="s" access="read"/>
    <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionIconName" type="s" access="read"/>
    <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionMovieName" type="s" access="read"/>
    <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <property name="Menu" type="o" access="read"/>
    <property name="IconThemePath" type="s" access="read"/>
    <property name="XAyatanaLabel" type="s" access="read"/>
    <method name="ContextMenu">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Activate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="SecondaryActivate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Scroll">
      <arg name="delta" type="i" direction="in"/>
      <arg name="orientation" type="s" direction="in"/>
    </method>
    <method name="XAyatanaSecondaryActivate">
      <arg name="timestamp" type="u" direction="in"/>
    </method>
    <method name="ProvideXdgActivationToken">
      <arg name="token" type="s" direction="in"/>
    </method>
    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewAttentionIcon"/>
    <signal name="NewOverlayIcon"/>
    <signal name="NewToolTip"/>
    <signal name="NewStatus">
      <arg name="status" type="s"/>
    </signal>
  </interface>
</node>
"#;

const MENU_INTROSPECTION_XML: &str = r#"
<node>
  <interface name="com.canonical.dbusmenu">
    <property name="Version" type="u" access="read"/>
    <property name="TextDirection" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconThemePath" type="as" access="read"/>
    <method name="GetLayout">
      <arg name="parentId" type="i" direction="in"/>
      <arg name="recursionDepth" type="i" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="revision" type="u" direction="out"/>
      <arg name="layout" type="(ia{sv}av)" direction="out"/>
    </method>
    <method name="GetGroupProperties">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="properties" type="a(ia{sv})" direction="out"/>
    </method>
    <method name="GetProperty">
      <arg name="id" type="i" direction="in"/>
      <arg name="name" type="s" direction="in"/>
      <arg name="value" type="v" direction="out"/>
    </method>
    <method name="Event">
      <arg name="id" type="i" direction="in"/>
      <arg name="eventId" type="s" direction="in"/>
      <arg name="data" type="v" direction="in"/>
      <arg name="timestamp" type="u" direction="in"/>
    </method>
    <method name="EventGroup">
      <arg name="events" type="a(isvu)" direction="in"/>
      <arg name="idErrors" type="ai" direction="out"/>
    </method>
    <method name="AboutToShow">
      <arg name="id" type="i" direction="in"/>
      <arg name="needUpdate" type="b" direction="out"/>
    </method>
    <method name="AboutToShowGroup">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="updatesNeeded" type="ai" direction="out"/>
      <arg name="idErrors" type="ai" direction="out"/>
    </method>
    <signal name="ItemsPropertiesUpdated">
      <arg name="updatedProps" type="a(ia{sv})"/>
      <arg name="removedProps" type="a(ias)"/>
    </signal>
    <signal name="LayoutUpdated">
      <arg name="revision" type="u"/>
      <arg name="parent" type="i"/>
    </signal>
  </interface>
</node>
"#;

type PresentCallback = Rc<dyn Fn(Option<&str>)>;
type QuitCallback = Rc<dyn Fn()>;
type SetModeCallback = Rc<dyn Fn(u8) -> Result<(), String>>;
type UpdatedProperties = Vec<(i32, HashMap<String, glib::Variant>)>;
type RemovedProperties = Vec<(i32, Vec<String>)>;
type MenuDiff = (bool, UpdatedProperties, RemovedProperties);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuItemKind {
    Standard,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrayMenuItem {
    id: i32,
    label: String,
    enabled: bool,
    kind: MenuItemKind,
    /// `Some(checked)` renders a DBusMenu radio item with the given state.
    toggle_state: Option<bool>,
    children: Vec<TrayMenuItem>,
}

/// Escapes literal underscores so DBusMenu clients do not strip them as mnemonics.
fn escape_dbusmenu_label(label: &str) -> String {
    label.replace('_', "__")
}

impl TrayMenuItem {
    fn standard(id: i32, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            kind: MenuItemKind::Standard,
            toggle_state: None,
            children: Vec::new(),
        }
    }

    fn info(id: i32, label: impl Into<String>) -> Self {
        Self {
            enabled: false,
            ..Self::standard(id, label)
        }
    }

    fn separator(id: i32) -> Self {
        Self {
            id,
            label: String::new(),
            enabled: false,
            kind: MenuItemKind::Separator,
            toggle_state: None,
            children: Vec::new(),
        }
    }

    fn mode(
        id_base: i32,
        index: u8,
        label: impl Into<String>,
        enabled: bool,
        checked: bool,
    ) -> Self {
        Self {
            id: id_base + i32::from(index),
            label: label.into(),
            enabled,
            kind: MenuItemKind::Standard,
            toggle_state: Some(checked),
            children: Vec::new(),
        }
    }

    fn submenu(
        id: i32,
        label: impl Into<String>,
        enabled: bool,
        children: Vec<TrayMenuItem>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            enabled,
            kind: MenuItemKind::Standard,
            toggle_state: None,
            children,
        }
    }

    fn find(&self, id: i32) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    fn collect_items<'a>(&'a self, out: &mut HashMap<i32, &'a Self>) {
        out.insert(self.id, self);
        for child in &self.children {
            child.collect_items(out);
        }
    }

    fn structure_matches(&self, other: &Self) -> bool {
        self.id == other.id
            && self.kind == other.kind
            && self.children.is_empty() == other.children.is_empty()
            && self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(&other.children)
                .all(|(lhs, rhs)| lhs.structure_matches(rhs))
    }

    fn properties(&self) -> HashMap<String, glib::Variant> {
        let mut properties = HashMap::new();
        if self.id == ROOT_ID {
            properties.insert("children-display".to_string(), "submenu".to_variant());
            return properties;
        }

        match self.kind {
            MenuItemKind::Separator => {
                properties.insert("type".to_string(), "separator".to_variant());
                properties.insert("visible".to_string(), true.to_variant());
            }
            MenuItemKind::Standard => {
                if !self.label.is_empty() {
                    properties.insert(
                        "label".to_string(),
                        escape_dbusmenu_label(&self.label).to_variant(),
                    );
                }
                properties.insert("enabled".to_string(), self.enabled.to_variant());
                properties.insert("visible".to_string(), true.to_variant());
                properties.insert("type".to_string(), "standard".to_variant());
            }
        }
        if let Some(checked) = self.toggle_state {
            properties.insert("toggle-type".to_string(), "radio".to_variant());
            properties.insert(
                "toggle-state".to_string(),
                if checked { 1i32 } else { 0i32 }.to_variant(),
            );
        }
        if !self.children.is_empty() {
            properties.insert("children-display".to_string(), "submenu".to_variant());
        }
        properties
    }

    fn filtered_properties(&self, property_names: &[String]) -> HashMap<String, glib::Variant> {
        let mut props = self.properties();
        if !property_names.is_empty() {
            props.retain(|key, _| property_names.iter().any(|wanted| wanted == key));
        }
        props
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrayMenu {
    root: TrayMenuItem,
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self::build(&QuickSettingsSnapshot::default())
    }
}

impl TrayMenu {
    fn build(snapshot: &QuickSettingsSnapshot) -> Self {
        let open_label = if snapshot.open_app_label.is_empty() {
            crate::t!("qs-open-app")
        } else {
            snapshot.open_app_label.clone()
        };
        let mut children = vec![TrayMenuItem::standard(OPEN_ID, open_label)];

        if snapshot.connected {
            if !snapshot.device_name.is_empty() {
                children.push(TrayMenuItem::info(STATUS_ID, snapshot.device_name.clone()));
            }
            if !snapshot.battery_summary.is_empty() {
                children.push(TrayMenuItem::info(
                    BATTERY_ID,
                    snapshot.battery_summary.clone(),
                ));
            }
            if !snapshot.status_hint.is_empty() {
                children.push(TrayMenuItem::info(HINT_ID, snapshot.status_hint.clone()));
            }
        } else {
            children.push(TrayMenuItem::info(STATUS_ID, crate::t!("tray-no-device")));
        }

        if !snapshot.primary_modes.is_empty() {
            children.push(TrayMenuItem::separator(NOISE_SEPARATOR_ID));
            children.push(TrayMenuItem::info(
                NOISE_HEADER_ID,
                crate::t!("tray-noise-control"),
            ));
            children.extend(snapshot.primary_modes.iter().map(|(index, label, _icon)| {
                let index = u8::try_from(*index).unwrap_or_default();
                TrayMenuItem::mode(
                    MODE_ID_BASE,
                    index,
                    label.clone(),
                    snapshot.controls_enabled,
                    i32::from(index) == snapshot.active_group_index,
                )
            }));

            if !snapshot.sub_modes.is_empty() {
                let submenu_title = if snapshot.sub_modes_title.is_empty() {
                    crate::t!("noise-mode-options")
                } else {
                    snapshot.sub_modes_title.clone()
                };
                let sub_items = snapshot
                    .sub_modes
                    .iter()
                    .map(|(index, label)| {
                        let index = u8::try_from(*index).unwrap_or_default();
                        TrayMenuItem::mode(
                            SUB_MODE_ID_BASE,
                            index,
                            label.clone(),
                            snapshot.controls_enabled,
                            i32::from(index) == snapshot.active_mode_index,
                        )
                    })
                    .collect();
                children.push(TrayMenuItem::submenu(
                    SUB_MODES_SUBMENU_ID,
                    submenu_title,
                    snapshot.controls_enabled,
                    sub_items,
                ));
            }
        }

        children.push(TrayMenuItem::separator(QUIT_SEPARATOR_ID));
        children.push(TrayMenuItem::standard(QUIT_ID, crate::t!("tray-quit")));

        Self {
            root: TrayMenuItem {
                id: ROOT_ID,
                label: String::new(),
                enabled: true,
                kind: MenuItemKind::Standard,
                toggle_state: None,
                children,
            },
        }
    }

    fn find(&self, id: i32) -> Option<&TrayMenuItem> {
        self.root.find(id)
    }

    fn diff_from(&self, prev: &Self) -> MenuDiff {
        let layout_changed = !self.root.structure_matches(&prev.root);

        let mut prev_items = HashMap::new();
        prev.root.collect_items(&mut prev_items);
        let mut next_items = HashMap::new();
        self.root.collect_items(&mut next_items);

        let mut updated_props = Vec::new();
        let mut removed_props = Vec::new();

        let mut sorted_ids: Vec<i32> = next_items.keys().copied().collect();
        sorted_ids.sort_unstable();

        for id in sorted_ids {
            if id == ROOT_ID {
                continue;
            }
            let Some(&prev_item) = prev_items.get(&id) else {
                continue;
            };
            let next_item = next_items[&id];
            if prev_item == next_item {
                continue;
            }

            let old_map = prev_item.properties();
            let new_map = next_item.properties();

            let mut changed = HashMap::new();
            for (key, val) in &new_map {
                if old_map.get(key) != Some(val) {
                    changed.insert(key.clone(), val.clone());
                }
            }
            if !changed.is_empty() {
                updated_props.push((id, changed));
            }

            let removed: Vec<String> = old_map
                .keys()
                .filter(|key| !new_map.contains_key(*key))
                .cloned()
                .collect();
            if !removed.is_empty() {
                removed_props.push((id, removed));
            }
        }

        (layout_changed, updated_props, removed_props)
    }
}

fn empty_pixmaps() -> Vec<(i32, i32, Vec<u8>)> {
    Vec::new()
}

fn menu_item_variant(
    item: &TrayMenuItem,
    recursion_depth: i32,
    property_names: &[String],
) -> glib::Variant {
    let children = if recursion_depth == 0 {
        Vec::new()
    } else {
        let next_depth = if recursion_depth < 0 {
            -1
        } else {
            recursion_depth - 1
        };
        item.children
            .iter()
            .map(|child| menu_item_variant(child, next_depth, property_names))
            .collect::<Vec<_>>()
    };
    let child_variants = children
        .iter()
        .map(glib::Variant::from_variant)
        .collect::<Vec<_>>();
    let children_variant =
        glib::Variant::array_from_iter_with_type(glib::VariantTy::VARIANT, child_variants);

    glib::Variant::tuple_from_iter([
        item.id.to_variant(),
        item.filtered_properties(property_names).to_variant(),
        children_variant,
    ])
}

fn tooltip_subtitle(snapshot: &QuickSettingsSnapshot) -> String {
    if !snapshot.connected {
        return crate::t!("tray-background");
    }
    match (
        snapshot.device_name.is_empty(),
        snapshot.battery_summary.is_empty(),
    ) {
        (true, true) => crate::t!("tray-background"),
        (false, true) => snapshot.device_name.clone(),
        (true, false) => snapshot.battery_summary.clone(),
        (false, false) => format!("{} · {}", snapshot.device_name, snapshot.battery_summary),
    }
}

struct Inner {
    connection: Option<gio::DBusConnection>,
    snapshot: QuickSettingsSnapshot,
    menu: TrayMenu,
    revision: u32,
    icon_name: String,
    icon_theme_path: String,
    activation_token: Option<String>,
    watcher_owner: Option<String>,
    unwatch_watcher: Option<Box<dyn FnOnce()>>,
    on_present: Option<PresentCallback>,
    on_quit: Option<QuitCallback>,
    on_set_mode: Option<SetModeCallback>,
    registrations: Vec<gio::RegistrationId>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(unwatch) = self.unwatch_watcher.take() {
            unwatch();
        }
        if let Some(conn) = self.connection.take() {
            for registration_id in self.registrations.drain(..) {
                let _ = conn.unregister_object(registration_id);
            }
        }
    }
}

pub(crate) struct TrayService {
    inner: RefCell<Inner>,
}

impl TrayService {
    pub(crate) fn new(connection: Option<gio::DBusConnection>) -> Rc<Self> {
        let icon_theme_path = crate::resources::resolve_tray_icon_theme_path(TRAY_ICON_NAME);
        let service = Rc::new(Self {
            inner: RefCell::new(Inner {
                connection: connection.clone(),
                snapshot: QuickSettingsSnapshot::default(),
                menu: TrayMenu::default(),
                revision: 1,
                icon_name: TRAY_ICON_NAME.to_string(),
                icon_theme_path,
                activation_token: None,
                watcher_owner: None,
                unwatch_watcher: None,
                on_present: None,
                on_quit: None,
                on_set_mode: None,
                registrations: Vec::new(),
            }),
        });

        if let Some(conn) = connection {
            register_objects(Rc::clone(&service), &conn);
            let unwatch = watch_watcher(Rc::downgrade(&service), &conn);
            service.inner.borrow_mut().unwatch_watcher = Some(unwatch);
        }

        service
    }

    pub(crate) fn set_handlers(
        &self,
        on_present: impl Fn(Option<&str>) + 'static,
        on_quit: impl Fn() + 'static,
        on_set_mode: impl Fn(u8) -> Result<(), String> + 'static,
    ) {
        let mut inner = self.inner.borrow_mut();
        inner.on_present = Some(Rc::new(on_present));
        inner.on_quit = Some(Rc::new(on_quit));
        inner.on_set_mode = Some(Rc::new(on_set_mode));
    }

    /// Refreshes the tray menu and tooltip from the same snapshot the Quick Settings service uses.
    pub(crate) fn update(&self, snapshot: QuickSettingsSnapshot) {
        let (conn, layout_changed, revision, updated_props, removed_props, tooltip_variant) = {
            let mut inner = self.inner.borrow_mut();
            if inner.snapshot == snapshot {
                return;
            }
            let prev_tooltip = tooltip_subtitle(&inner.snapshot);
            let next_tooltip = tooltip_subtitle(&snapshot);
            let next_menu = TrayMenu::build(&snapshot);
            let (layout_changed, updated_props, removed_props) = next_menu.diff_from(&inner.menu);

            inner.snapshot = snapshot;
            inner.menu = next_menu;
            if layout_changed {
                inner.revision = inner.revision.wrapping_add(1);
            }
            let tooltip_variant = (prev_tooltip != next_tooltip).then(|| {
                (
                    inner.icon_name.clone(),
                    empty_pixmaps(),
                    TRAY_TITLE.to_string(),
                    next_tooltip,
                )
                    .to_variant()
            });
            (
                inner.connection.clone(),
                layout_changed,
                inner.revision,
                updated_props,
                removed_props,
                tooltip_variant,
            )
        };

        if let Some(conn) = conn {
            if !updated_props.is_empty() || !removed_props.is_empty() {
                let params = (updated_props, removed_props).to_variant();
                let _ = conn.emit_signal(
                    None,
                    MENU_OBJECT_PATH,
                    MENU_INTERFACE,
                    "ItemsPropertiesUpdated",
                    Some(&params),
                );
            }
            if layout_changed {
                let params = (revision, ROOT_ID).to_variant();
                let _ = conn.emit_signal(
                    None,
                    MENU_OBJECT_PATH,
                    MENU_INTERFACE,
                    "LayoutUpdated",
                    Some(&params),
                );
            }
            if let Some(tooltip) = tooltip_variant {
                let _ = conn.emit_signal(None, SNI_OBJECT_PATH, SNI_INTERFACE, "NewToolTip", None);
                let changed = HashMap::from([("ToolTip", tooltip)]);
                let invalidated: Vec<&str> = Vec::new();
                let params = (SNI_INTERFACE, changed, invalidated).to_variant();
                let _ = conn.emit_signal(
                    None,
                    SNI_OBJECT_PATH,
                    "org.freedesktop.DBus.Properties",
                    "PropertiesChanged",
                    Some(&params),
                );
            }
        }
    }

    fn present(&self) {
        let (callback, token) = {
            let mut inner = self.inner.borrow_mut();
            (inner.on_present.clone(), inner.activation_token.take())
        };
        if let Some(callback) = callback {
            callback(token.as_deref());
        }
    }

    fn activate_item(&self, id: i32) {
        if id == OPEN_ID {
            self.present();
            return;
        }
        // Any non-window menu action discards the token provided by the host before Event().
        self.inner.borrow_mut().activation_token = None;

        match id {
            QUIT_ID => {
                let callback = self.inner.borrow().on_quit.clone();
                if let Some(callback) = callback {
                    callback();
                }
            }
            SUB_MODE_ID_BASE.. => {
                let Ok(index) = u8::try_from(id - SUB_MODE_ID_BASE) else {
                    return;
                };
                let callback = self.inner.borrow().on_set_mode.clone();
                if let Some(callback) = callback
                    && let Err(error) = callback(index)
                {
                    eprintln!("Tray noise sub-mode request failed: {error}");
                }
            }
            MODE_ID_BASE..SUB_MODE_ID_BASE => {
                let Ok(index) = u8::try_from(id - MODE_ID_BASE) else {
                    return;
                };
                let callback = self.inner.borrow().on_set_mode.clone();
                if let Some(callback) = callback
                    && let Err(error) = callback(index)
                {
                    eprintln!("Tray noise-mode request failed: {error}");
                }
            }
            _ => {}
        }
    }

    fn sni_property(&self, name: &str) -> glib::Variant {
        let inner = self.inner.borrow();
        match name {
            "Category" => "ApplicationStatus".to_variant(),
            "Id" => TRAY_APP_ID.to_variant(),
            "Title" => TRAY_TITLE.to_variant(),
            "Status" => "Active".to_variant(),
            "WindowId" => 0i32.to_variant(),
            "IconName" => inner.icon_name.clone().to_variant(),
            "IconPixmap" => empty_pixmaps().to_variant(),
            "OverlayIconName" => String::new().to_variant(),
            "OverlayIconPixmap" => empty_pixmaps().to_variant(),
            "AttentionIconName" => String::new().to_variant(),
            "AttentionIconPixmap" => empty_pixmaps().to_variant(),
            "AttentionMovieName" => String::new().to_variant(),
            "ToolTip" => (
                inner.icon_name.clone(),
                empty_pixmaps(),
                TRAY_TITLE.to_string(),
                tooltip_subtitle(&inner.snapshot),
            )
                .to_variant(),
            "ItemIsMenu" => false.to_variant(),
            "Menu" => glib::variant::ObjectPath::try_from(MENU_OBJECT_PATH)
                .map(|path| path.to_variant())
                .unwrap_or_else(|_| String::new().to_variant()),
            "IconThemePath" => inner.icon_theme_path.clone().to_variant(),
            "XAyatanaLabel" => String::new().to_variant(),
            _ => String::new().to_variant(),
        }
    }

    fn menu_property(&self, name: &str) -> glib::Variant {
        let inner = self.inner.borrow();
        match name {
            "Version" => DBUSMENU_PROTOCOL_VERSION.to_variant(),
            "TextDirection" => "ltr".to_variant(),
            "Status" => "normal".to_variant(),
            "IconThemePath" => if inner.icon_theme_path.is_empty() {
                Vec::<String>::new()
            } else {
                vec![inner.icon_theme_path.clone()]
            }
            .to_variant(),
            _ => String::new().to_variant(),
        }
    }

    fn handle_sni_method(
        &self,
        method: &str,
        params: &glib::Variant,
        invocation: gio::DBusMethodInvocation,
    ) {
        match method {
            "Activate" | "SecondaryActivate" | "XAyatanaSecondaryActivate" => {
                self.present();
                invocation.return_value(None);
            }
            "ProvideXdgActivationToken" => {
                if let Some((token,)) = params.get::<(String,)>() {
                    let trimmed = token.trim();
                    self.inner.borrow_mut().activation_token =
                        (!trimmed.is_empty()).then(|| trimmed.to_string());
                }
                invocation.return_value(None);
            }
            "ContextMenu" | "Scroll" => invocation.return_value(None),
            _ => invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.UnknownMethod",
                &format!("Unknown StatusNotifierItem method: {method}"),
            ),
        }
    }

    fn handle_menu_method(
        &self,
        method: &str,
        params: &glib::Variant,
        invocation: gio::DBusMethodInvocation,
    ) {
        match method {
            "GetLayout" => self.handle_get_layout(params, invocation),
            "GetGroupProperties" => self.handle_get_group_properties(params, invocation),
            "GetProperty" => self.handle_get_property(params, invocation),
            "Event" => {
                if let Some((id, event_id, _data, _timestamp)) =
                    params.get::<(i32, String, glib::Variant, u32)>()
                {
                    if event_id == "clicked" {
                        self.activate_item(id);
                    } else {
                        self.inner.borrow_mut().activation_token = None;
                    }
                }
                invocation.return_value(None);
            }
            "EventGroup" => {
                let mut id_errors = Vec::<i32>::new();
                if let Some((events,)) = params.get::<(Vec<(i32, String, glib::Variant, u32)>,)>() {
                    for (id, event_id, _data, _timestamp) in events {
                        let known = self.inner.borrow().menu.find(id).is_some();
                        if !known {
                            id_errors.push(id);
                            self.inner.borrow_mut().activation_token = None;
                        } else if event_id == "clicked" {
                            self.activate_item(id);
                        } else {
                            self.inner.borrow_mut().activation_token = None;
                        }
                    }
                }
                invocation.return_value(Some(&(id_errors,).to_variant()));
            }
            "AboutToShow" => {
                let Some((id,)) = params.get::<(i32,)>() else {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.InvalidArgs",
                        "Expected AboutToShow(i)",
                    );
                    return;
                };
                if self.inner.borrow().menu.find(id).is_none() {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.InvalidArgs",
                        &format!("Unknown DBusMenu item ID: {id}"),
                    );
                    return;
                }
                invocation.return_value(Some(&(false,).to_variant()));
            }
            "AboutToShowGroup" => {
                let mut id_errors = Vec::<i32>::new();
                if let Some((ids,)) = params.get::<(Vec<i32>,)>() {
                    let inner = self.inner.borrow();
                    for id in ids {
                        if inner.menu.find(id).is_none() {
                            id_errors.push(id);
                        }
                    }
                }
                invocation.return_value(Some(&(Vec::<i32>::new(), id_errors).to_variant()));
            }
            _ => invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.UnknownMethod",
                &format!("Unknown DBusMenu method: {method}"),
            ),
        }
    }

    fn handle_get_layout(&self, params: &glib::Variant, invocation: gio::DBusMethodInvocation) {
        let Some((parent_id, recursion_depth, property_names)) =
            params.get::<(i32, i32, Vec<String>)>()
        else {
            invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                "Expected GetLayout(i, i, as)",
            );
            return;
        };

        let result = {
            let inner = self.inner.borrow();
            inner.menu.find(parent_id).map(|item| {
                (
                    inner.revision,
                    menu_item_variant(item, recursion_depth, &property_names),
                )
            })
        };
        let Some((revision, layout)) = result else {
            invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                &format!("Unknown DBusMenu parent ID: {parent_id}"),
            );
            return;
        };
        let reply = glib::Variant::tuple_from_iter([revision.to_variant(), layout]);
        invocation.return_value(Some(&reply));
    }

    fn handle_get_group_properties(
        &self,
        params: &glib::Variant,
        invocation: gio::DBusMethodInvocation,
    ) {
        let Some((ids, property_names)) = params.get::<(Vec<i32>, Vec<String>)>() else {
            invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                "Expected GetGroupProperties(ai, as)",
            );
            return;
        };

        let properties = {
            let inner = self.inner.borrow();
            if ids.is_empty() {
                let mut all_items = HashMap::new();
                inner.menu.root.collect_items(&mut all_items);
                let mut sorted_ids: Vec<i32> = all_items.keys().copied().collect();
                sorted_ids.sort_unstable();
                sorted_ids
                    .into_iter()
                    .filter(|id| *id != ROOT_ID)
                    .map(|id| (id, all_items[&id].filtered_properties(&property_names)))
                    .collect::<Vec<_>>()
            } else {
                ids.iter()
                    .filter_map(|id| {
                        inner
                            .menu
                            .find(*id)
                            .map(|item| (item.id, item.filtered_properties(&property_names)))
                    })
                    .collect::<Vec<_>>()
            }
        };
        invocation.return_value(Some(&(properties,).to_variant()));
    }

    fn handle_get_property(&self, params: &glib::Variant, invocation: gio::DBusMethodInvocation) {
        let Some((id, name)) = params.get::<(i32, String)>() else {
            invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                "Expected GetProperty(i, s)",
            );
            return;
        };

        let value = {
            let inner = self.inner.borrow();
            inner
                .menu
                .find(id)
                .and_then(|item| item.properties().get(&name).cloned())
        };
        match value {
            Some(value) => invocation.return_value(Some(&(value,).to_variant())),
            None => invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.UnknownProperty",
                &format!("Unknown DBusMenu property: {name}"),
            ),
        }
    }

    fn register_with_watcher(self: &Rc<Self>, conn: &gio::DBusConnection, owner: &str) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.watcher_owner.as_deref() == Some(owner) {
                return;
            }
            inner.watcher_owner = Some(owner.to_string());
        }

        let Some(unique_name) = conn.unique_name() else {
            self.inner.borrow_mut().watcher_owner = None;
            eprintln!("Cannot register tray item: session bus has no unique name");
            return;
        };
        let parameters = (unique_name.as_str(),).to_variant();
        let weak_service = Rc::downgrade(self);
        let owner_owned = owner.to_string();
        conn.call(
            Some(WATCHER_BUS_NAME),
            WATCHER_OBJECT_PATH,
            WATCHER_INTERFACE,
            "RegisterStatusNotifierItem",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            DBUS_TIMEOUT_MS,
            None::<&gio::Cancellable>,
            move |result| {
                let Some(service) = weak_service.upgrade() else {
                    return;
                };
                match result {
                    Ok(_) => {
                        if let Some(conn) = service.inner.borrow().connection.clone() {
                            let _ = conn.emit_signal(
                                None,
                                SNI_OBJECT_PATH,
                                SNI_INTERFACE,
                                "NewIcon",
                                None,
                            );
                        }
                    }
                    Err(error) => {
                        let mut inner = service.inner.borrow_mut();
                        if inner.watcher_owner.as_deref() == Some(owner_owned.as_str()) {
                            inner.watcher_owner = None;
                        }
                        drop(inner);
                        eprintln!(
                            "Failed to register tray item with {WATCHER_BUS_NAME} at {WATCHER_OBJECT_PATH}: {error}"
                        );
                    }
                }
            },
        );
    }
}

fn register_objects(service: Rc<TrayService>, conn: &gio::DBusConnection) {
    register_sni(service.clone(), conn);
    register_menu(service, conn);
}

fn register_sni(service: Rc<TrayService>, conn: &gio::DBusConnection) {
    let node_info = match gio::DBusNodeInfo::for_xml(SNI_INTROSPECTION_XML) {
        Ok(node_info) => node_info,
        Err(error) => {
            eprintln!("Failed to parse StatusNotifierItem D-Bus XML: {error}");
            return;
        }
    };
    let Some(interface_info) = node_info.lookup_interface(SNI_INTERFACE) else {
        eprintln!("StatusNotifierItem D-Bus XML is missing {SNI_INTERFACE}");
        return;
    };

    let methods_service = Rc::clone(&service);
    let properties_service = Rc::clone(&service);
    match conn
        .register_object(SNI_OBJECT_PATH, &interface_info)
        .method_call(
            move |_conn, _sender, _path, _iface, method, params, invocation| {
                methods_service.handle_sni_method(method, &params, invocation);
            },
        )
        .property(move |_conn, _sender, _path, _iface, property| {
            properties_service.sni_property(property)
        })
        .build()
    {
        Ok(registration_id) => service
            .inner
            .borrow_mut()
            .registrations
            .push(registration_id),
        Err(error) => {
            eprintln!("Failed to register StatusNotifierItem object at {SNI_OBJECT_PATH}: {error}")
        }
    }
}

fn register_menu(service: Rc<TrayService>, conn: &gio::DBusConnection) {
    let node_info = match gio::DBusNodeInfo::for_xml(MENU_INTROSPECTION_XML) {
        Ok(node_info) => node_info,
        Err(error) => {
            eprintln!("Failed to parse DBusMenu D-Bus XML: {error}");
            return;
        }
    };
    let Some(interface_info) = node_info.lookup_interface(MENU_INTERFACE) else {
        eprintln!("DBusMenu D-Bus XML is missing {MENU_INTERFACE}");
        return;
    };

    let methods_service = Rc::clone(&service);
    let properties_service = Rc::clone(&service);
    match conn
        .register_object(MENU_OBJECT_PATH, &interface_info)
        .method_call(
            move |_conn, _sender, _path, _iface, method, params, invocation| {
                methods_service.handle_menu_method(method, &params, invocation);
            },
        )
        .property(move |_conn, _sender, _path, _iface, property| {
            properties_service.menu_property(property)
        })
        .build()
    {
        Ok(registration_id) => service
            .inner
            .borrow_mut()
            .registrations
            .push(registration_id),
        Err(error) => {
            eprintln!("Failed to register DBusMenu object at {MENU_OBJECT_PATH}: {error}")
        }
    }
}

fn watch_watcher(service: Weak<TrayService>, conn: &gio::DBusConnection) -> Box<dyn FnOnce()> {
    let appeared_service = service.clone();
    let vanished_service = service;
    let watcher_id = gio::bus_watch_name_on_connection(
        conn,
        WATCHER_BUS_NAME,
        gio::BusNameWatcherFlags::NONE,
        move |conn, _name, owner| {
            if let Some(service) = appeared_service.upgrade() {
                service.register_with_watcher(&conn, owner);
            }
        },
        move |_conn, _name| {
            if let Some(service) = vanished_service.upgrade() {
                service.inner.borrow_mut().watcher_owner = None;
            }
        },
    );
    Box::new(move || gio::bus_unwatch_name(watcher_id))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn connected_snapshot() -> QuickSettingsSnapshot {
        QuickSettingsSnapshot {
            connected: true,
            device_name: "OPPO Enco X4".to_string(),
            controls_enabled: true,
            active_group_index: 1,
            active_mode_index: 13,
            primary_modes: vec![
                (
                    0,
                    "Off".to_string(),
                    "audio-volume-muted-symbolic".to_string(),
                ),
                (
                    1,
                    "Noise cancelling".to_string(),
                    "audio-headphones-symbolic".to_string(),
                ),
                (
                    2,
                    "Transparency".to_string(),
                    "audio-hearing-symbolic".to_string(),
                ),
            ],
            battery_summary: "Left Ear 80% · Right Ear 75%".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn menu_exposes_open_device_battery_modes_and_quit() {
        crate::i18n::set_locale("en-US");
        let menu = TrayMenu::build(&connected_snapshot());
        let ids = menu
            .root
            .children
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&OPEN_ID));
        assert!(ids.contains(&STATUS_ID));
        assert!(ids.contains(&BATTERY_ID));
        assert!(ids.contains(&QUIT_ID));
        assert!(ids.contains(&(MODE_ID_BASE + 1)));

        let active_mode = menu.find(MODE_ID_BASE + 1).unwrap();
        assert_eq!(active_mode.toggle_state, Some(true));
        assert!(active_mode.enabled);
        let inactive_mode = menu.find(MODE_ID_BASE + 2).unwrap();
        assert_eq!(inactive_mode.toggle_state, Some(false));

        crate::i18n::reset_locale();
    }

    #[test]
    fn disconnected_menu_hides_modes_and_shows_placeholder() {
        crate::i18n::set_locale("en-US");
        let menu = TrayMenu::build(&QuickSettingsSnapshot::default());
        let ids = menu
            .root
            .children
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();

        assert_eq!(ids, vec![OPEN_ID, STATUS_ID, QUIT_SEPARATOR_ID, QUIT_ID]);
        assert!(!ids.iter().any(|id| *id >= MODE_ID_BASE));
        assert_eq!(
            menu.find(STATUS_ID).unwrap().label,
            crate::t!("tray-no-device")
        );
        assert!(!menu.find(STATUS_ID).unwrap().enabled);

        crate::i18n::reset_locale();
    }

    #[test]
    fn mode_items_are_disabled_when_controls_are_unavailable() {
        crate::i18n::set_locale("en-US");
        let mut snapshot = connected_snapshot();
        snapshot.controls_enabled = false;
        snapshot.status_hint = "Wear at least one earbud".to_string();
        let menu = TrayMenu::build(&snapshot);

        assert_eq!(
            menu.find(HINT_ID).unwrap().label,
            "Wear at least one earbud"
        );
        assert!(!menu.find(HINT_ID).unwrap().enabled);
        for (index, _, _) in &snapshot.primary_modes {
            assert!(!menu.find(MODE_ID_BASE + *index).unwrap().enabled);
        }

        crate::i18n::reset_locale();
    }

    #[test]
    fn exposes_sub_modes_submenu_and_escapes_underscore_mnemonics() {
        crate::i18n::set_locale("en-US");
        let mut snapshot = connected_snapshot();
        snapshot.device_name = "OPPO_Enco_X4".to_string();
        snapshot.sub_modes_title = "Noise cancelling level".to_string();
        snapshot.sub_modes = vec![(11, "Mild".to_string()), (13, "Max".to_string())];
        let menu = TrayMenu::build(&snapshot);

        let status_props = menu.find(STATUS_ID).unwrap().properties();
        assert_eq!(
            status_props.get("label").and_then(glib::Variant::str),
            Some("OPPO__Enco__X4")
        );

        let submenu = menu.find(SUB_MODES_SUBMENU_ID).unwrap();
        assert_eq!(
            submenu
                .properties()
                .get("children-display")
                .and_then(glib::Variant::str),
            Some("submenu")
        );
        assert_eq!(
            menu.find(SUB_MODE_ID_BASE + 11).unwrap().toggle_state,
            Some(false)
        );
        assert_eq!(
            menu.find(SUB_MODE_ID_BASE + 13).unwrap().toggle_state,
            Some(true)
        );

        crate::i18n::reset_locale();
    }

    #[test]
    fn layout_variant_uses_dbusmenu_signature_and_filters_properties() {
        let menu = TrayMenu::build(&connected_snapshot());
        let layout = menu_item_variant(&menu.root, -1, &[]);
        assert_eq!(layout.type_().as_str(), "(ia{sv}av)");

        let root_children = layout.child_value(2);
        assert_eq!(root_children.type_().as_str(), "av");
        assert_eq!(root_children.n_children(), menu.root.children.len());

        let first_child = root_children.child_value(0).as_variant().unwrap();
        assert_eq!(first_child.type_().as_str(), "(ia{sv}av)");

        let filtered = menu_item_variant(&menu.root, 1, &["label".to_string()]);
        let open_item = filtered.child_value(2).child_value(0).as_variant().unwrap();
        let props = open_item
            .child_value(1)
            .get::<HashMap<String, glib::Variant>>()
            .unwrap();
        assert!(props.contains_key("label"));
        assert!(!props.contains_key("enabled"));

        // recursion depth 0 must not include children
        let shallow = menu_item_variant(&menu.root, 0, &[]);
        assert_eq!(shallow.child_value(2).n_children(), 0);
    }

    #[test]
    fn diff_from_distinguishes_in_place_property_updates_from_layout_changes() {
        crate::i18n::set_locale("en-US");
        let base_snapshot = connected_snapshot();
        let base_menu = TrayMenu::build(&base_snapshot);

        let mut updated_snapshot = base_snapshot.clone();
        updated_snapshot.battery_summary = "Left Ear 75% · Right Ear 70%".to_string();
        updated_snapshot.active_group_index = 2;
        let updated_menu = TrayMenu::build(&updated_snapshot);

        let (layout_changed, updated_props, removed_props) = updated_menu.diff_from(&base_menu);
        assert!(!layout_changed);
        assert!(removed_props.is_empty());
        let changed_ids: Vec<i32> = updated_props.iter().map(|(id, _)| *id).collect();
        assert!(changed_ids.contains(&BATTERY_ID));
        assert!(changed_ids.contains(&(MODE_ID_BASE + 1)));
        assert!(changed_ids.contains(&(MODE_ID_BASE + 2)));

        let disconnected_menu = TrayMenu::build(&QuickSettingsSnapshot::default());
        let (layout_changed_on_disconnect, _, _) = disconnected_menu.diff_from(&updated_menu);
        assert!(layout_changed_on_disconnect);

        crate::i18n::reset_locale();
    }

    #[test]
    fn non_window_actions_discard_pending_activation_token() {
        let service = TrayService::new(None);
        let presented_token: Rc<RefCell<Option<Option<String>>>> = Rc::new(RefCell::new(None));
        let last_mode = Rc::new(Cell::new(None::<u8>));

        let presented_clone = Rc::clone(&presented_token);
        let mode_clone = Rc::clone(&last_mode);
        service.set_handlers(
            move |token| {
                *presented_clone.borrow_mut() = Some(token.map(str::to_owned));
            },
            || {},
            move |index| {
                mode_clone.set(Some(index));
                Ok(())
            },
        );

        service.inner.borrow_mut().activation_token = Some("stale-token".to_string());
        service.activate_item(SUB_MODE_ID_BASE + 13);
        assert_eq!(last_mode.get(), Some(13));
        assert_eq!(service.inner.borrow().activation_token, None);

        service.present();
        assert_eq!(*presented_token.borrow(), Some(None));

        service.inner.borrow_mut().activation_token = Some("fresh-token".to_string());
        service.activate_item(OPEN_ID);
        assert_eq!(
            *presented_token.borrow(),
            Some(Some("fresh-token".to_string()))
        );
    }
}
