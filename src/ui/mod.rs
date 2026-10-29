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
use gtk::{Box as GtkBox, Label, Stack};

pub(crate) mod components;
pub(crate) mod dialogs;
pub(crate) mod pages;
pub(crate) mod scene3d;
#[cfg(test)]
mod tests;

pub(crate) use components::BatteryIndicator;
pub(crate) use pages::controls::ControlsWidgets;
pub(crate) use pages::controls::state as controls_state;
pub(crate) use pages::home::state as home_state;
pub(crate) use pages::sound::SoundWidgets;
pub(crate) use pages::sound::state as sound_state;

#[derive(Clone)]
pub(crate) struct PrimaryNoiseControl {
    pub(crate) index: u8,
    pub(crate) container: GtkBox,
    pub(crate) button: gtk::Button,
    pub(crate) label: Label,
}

pub(crate) struct HomeWidgets {
    pub(crate) home_columns: components::AdaptiveColumns,
    pub(crate) model_stack: Stack,
    pub(crate) model_picture: gtk::Picture,
    pub(crate) scene_view: scene3d::DeviceSceneView,
    pub(crate) model_placeholder: gtk::Image,
    pub(crate) model_firmware: Label,
    pub(crate) model_card: GtkBox,
    pub(crate) active_turntable: std::rc::Rc<
        std::cell::RefCell<Option<std::rc::Rc<crate::resources::turntable::TurntableModel>>>,
    >,
    pub(crate) current_frame: std::rc::Rc<std::cell::Cell<usize>>,
    pub(crate) turntable_state: std::rc::Rc<std::cell::RefCell<home_state::TurntableState>>,
    pub(crate) device_name: Label,
    pub(crate) connection_tag: Label,
    pub(crate) device_menu_button: gtk::MenuButton,
    pub(crate) device_popover: gtk::Popover,
    pub(crate) device_list: gtk::ListBox,
    pub(crate) device_list_empty: Label,
    pub(crate) refresh_devices: gtk::Button,
    pub(crate) left_battery: BatteryIndicator,
    pub(crate) right_battery: BatteryIndicator,
    pub(crate) case_battery: BatteryIndicator,
    pub(crate) noise_option_row: adw::ComboRow,
    pub(crate) paired_devices_section: GtkBox,
    pub(crate) paired_devices: gtk::ListBox,
    pub(crate) primary_noise_controls: Vec<PrimaryNoiseControl>,
    pub(crate) sound_card_section: GtkBox,
    pub(crate) eq_row: adw::ComboRow,
    pub(crate) spatial_row: adw::ComboRow,
    pub(crate) game_mode_section: adw::ActionRow,
    pub(crate) game_mode_switch: gtk::Switch,
}

pub(crate) struct DevicesWidgets {
    pub(crate) connection: Label,
    pub(crate) name: Label,
    pub(crate) product_id: Label,
    pub(crate) firmware: Label,
    pub(crate) wear: Label,
}

/// Switches every navigation page between the device content and a shared
/// "no headset connected" empty state.
pub(crate) struct ConnectionStateWidgets {
    pub(crate) stack: Stack,
    pub(crate) status_page: adw::StatusPage,
}

impl ConnectionStateWidgets {
    pub(crate) const CONTENT: &'static str = "content";
    pub(crate) const EMPTY: &'static str = "empty";

    pub(crate) fn show_content(&self) {
        self.stack.set_visible_child_name(Self::CONTENT);
    }

    pub(crate) fn show_disconnected(&self) {
        self.status_page
            .set_icon_name(Some("audio-headphones-symbolic"));
        self.status_page
            .set_title(&crate::t!("empty-state-no-headset-title"));
        self.status_page
            .set_description(Some(&crate::t!("empty-state-no-headset-description")));
        self.stack.set_visible_child_name(Self::EMPTY);
    }

    pub(crate) fn show_connecting(&self, device_name: &str) {
        let spinner = adw::SpinnerPaintable::new(Some(&self.status_page));
        self.status_page.set_paintable(Some(&spinner));
        self.status_page
            .set_title(&crate::t!("empty-state-connecting-title"));
        self.status_page.set_description(Some(device_name));
        self.stack.set_visible_child_name(Self::EMPTY);
    }
}

pub fn build(app: &adw::Application) {
    crate::resources::init_app_icons();
    components::install_styles();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("WaveTune")
        .default_width(1280)
        .default_height(768)
        .width_request(420)
        .height_request(320)
        .build();

    let (home_page, widgets) = pages::home::build_home_page();

    let stack = Stack::builder()
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .hhomogeneous(false)
        .vhomogeneous(false)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();
    let content_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .halign(gtk::Align::Fill)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .propagate_natural_width(false)
        .child(&stack)
        .build();

    stack.add_titled(&home_page, Some("home"), &crate::t!("nav-home"));
    let (sound_page, sound_widgets) = pages::sound::build_sound_page();
    stack.add_titled(&sound_page, Some("sound"), &crate::t!("nav-sound"));
    let (controls_page, controls_widgets) = pages::controls::build_controls_page();
    stack.add_titled(&controls_page, Some("controls"), &crate::t!("nav-controls"));
    let (devices_page, device_widgets) = pages::devices::build_devices_page();
    stack.add_titled(&devices_page, Some("devices"), &crate::t!("nav-devices"));

    let sidebar = adw::Sidebar::new();
    let navigation_section = adw::SidebarSection::new();
    for (title, icon_name) in [
        (crate::t!("nav-home"), "view-grid-symbolic"),
        (crate::t!("nav-sound"), "audio-speakers-symbolic"),
        (crate::t!("nav-controls"), "applications-system-symbolic"),
        (crate::t!("nav-devices"), "dialog-information-symbolic"),
    ] {
        navigation_section.append(
            adw::SidebarItem::builder()
                .title(&title)
                .icon_name(icon_name)
                .build(),
        );
    }
    sidebar.append(navigation_section);
    sidebar.set_selected(0);

    let sidebar_toolbar = adw::ToolbarView::new();
    sidebar_toolbar.set_top_bar_style(adw::ToolbarStyle::Flat);
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.set_title_widget(Some(&adw::WindowTitle::new("WaveTune", "")));

    let menu = gtk::gio::Menu::new();
    menu.append(
        Some(&crate::t!("pref-preferences")),
        Some("win.preferences"),
    );
    menu.append(Some(&crate::t!("about-title")), Some("win.about"));

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .tooltip_text(&crate::t!("nav-main-menu"))
        .build();
    sidebar_header.pack_end(&menu_button);

    let preferences_action = gtk::gio::SimpleAction::new("preferences", None);
    let window_weak = window.downgrade();
    preferences_action.connect_activate(move |_, _| {
        if let Some(win) = window_weak.upgrade() {
            dialogs::show_preferences_dialog(&win);
        }
    });
    window.add_action(&preferences_action);

    let about_action = gtk::gio::SimpleAction::new("about", None);
    let window_weak = window.downgrade();
    about_action.connect_activate(move |_, _| {
        if let Some(win) = window_weak.upgrade() {
            dialogs::show_about_dialog(&win);
        }
    });
    window.add_action(&about_action);

    app.set_accels_for_action("win.preferences", &["<Control>comma"]);

    sidebar_toolbar.add_top_bar(&sidebar_header);
    sidebar_toolbar.set_content(Some(&sidebar));
    let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "WaveTune");

    let content_toolbar = adw::ToolbarView::new();
    content_toolbar.set_top_bar_style(adw::ToolbarStyle::Flat);
    let content_header = adw::HeaderBar::new();
    let home_title = crate::t!("nav-home");
    let content_title = adw::WindowTitle::new(&home_title, "");
    content_header.set_title_widget(Some(&content_title));
    content_header.pack_end(&widgets.device_menu_button);

    content_toolbar.add_top_bar(&content_header);

    let empty_state = adw::StatusPage::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let connection_stack = Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();
    connection_stack.add_named(&content_scroll, Some(ConnectionStateWidgets::CONTENT));
    connection_stack.add_named(&empty_state, Some(ConnectionStateWidgets::EMPTY));
    let connection_state = ConnectionStateWidgets {
        stack: connection_stack.clone(),
        status_page: empty_state,
    };
    connection_state.show_disconnected();

    content_toolbar.set_content(Some(&connection_stack));
    let content_page = adw::NavigationPage::new(&content_toolbar, &home_title);

    let split_view = adw::NavigationSplitView::new();
    split_view.set_min_sidebar_width(220.0);
    split_view.set_max_sidebar_width(300.0);
    split_view.set_sidebar_width_fraction(0.24);
    split_view.set_sidebar(Some(&sidebar_page));
    split_view.set_content(Some(&content_page));
    split_view.set_show_content(true);
    window.set_content(Some(&split_view));

    let stack_for_navigation = stack.clone();
    let split_view_for_nav = split_view.clone();
    sidebar.connect_activated(move |_, index| {
        let page_name = match index {
            1 => "sound",
            2 => "controls",
            3 => "devices",
            _ => "home",
        };
        stack_for_navigation.set_visible_child_name(page_name);
        split_view_for_nav.set_show_content(true);
    });

    let content_page_for_updates = content_page.clone();
    let content_title_for_updates = content_title.clone();
    stack.connect_visible_child_notify(move |stack| {
        let title = match stack.visible_child_name().as_deref() {
            Some("sound") => crate::t!("nav-sound"),
            Some("controls") => crate::t!("nav-controls"),
            Some("devices") => crate::t!("nav-devices"),
            _ => crate::t!("nav-home"),
        };
        content_page_for_updates.set_title(&title);
        content_title_for_updates.set_title(&title);
    });

    let medium_bp = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("min-width: 800sp and max-width: 1200sp").unwrap(),
    );
    let hc_medium_apply = widgets.home_columns.clone();
    let hc_medium_unapply = widgets.home_columns.clone();
    let sc_medium_apply = sound_widgets.sound_columns.clone();
    let sc_medium_unapply = sound_widgets.sound_columns.clone();
    medium_bp.connect_apply(move |_| {
        hc_medium_apply.set_single_column(true);
        sc_medium_apply.set_single_column(true);
    });
    medium_bp.connect_unapply(move |_| {
        hc_medium_unapply.set_single_column(false);
        sc_medium_unapply.set_single_column(false);
    });
    window.add_breakpoint(medium_bp);

    let collapse_bp =
        adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 800sp").unwrap());
    let split_view_apply = split_view.clone();
    let split_view_unapply = split_view.clone();
    let hc_collapse = widgets.home_columns.clone();
    let sc_collapse = sound_widgets.sound_columns.clone();
    collapse_bp.connect_apply(move |_| {
        split_view_apply.set_collapsed(true);
        hc_collapse.set_single_column(true);
        sc_collapse.set_single_column(true);
    });
    collapse_bp.connect_unapply(move |_| {
        split_view_unapply.set_collapsed(false);
    });
    window.add_breakpoint(collapse_bp);

    window.present();

    crate::app::controller::wire_bluetooth_controls(
        widgets,
        sound_widgets,
        controls_widgets,
        device_widgets,
        connection_state,
    );
}
