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

mod app;
mod bluetooth;
mod compatibility;
pub mod i18n;
pub mod protocol;
pub(crate) mod resources;
mod ui;

use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;

const APP_ID: &str = "io.github.onlymash.WaveTune";

fn main() -> gtk::glib::ExitCode {
    crate::i18n::init();

    if std::env::args().nth(1).as_deref() == Some("--probe") {
        if let Err(error) = bluetooth::probe() {
            eprintln!("{}", crate::t!("device-read-failed", error));
            std::process::exit(1);
        }
        return gtk::glib::ExitCode::SUCCESS;
    }

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.add_main_option(
        "background",
        gtk::glib::Char::from(b'b'),
        gtk::glib::OptionFlags::NONE,
        gtk::glib::OptionArg::None,
        "Start silently in the background without opening the main window",
        None,
    );

    let session_bus =
        gtk::gio::bus_get_sync(gtk::gio::BusType::Session, gtk::gio::Cancellable::NONE).ok();
    let quick_settings = app::dbus_service::QuickSettingsService::new(session_bus.clone());
    let tray = app::tray_service::TrayService::new(session_bus);

    let window_slot: Rc<RefCell<Option<adw::ApplicationWindow>>> = Rc::new(RefCell::new(None));
    let hold_guard: Rc<RefCell<Option<gtk::gio::ApplicationHoldGuard>>> =
        Rc::new(RefCell::new(None));

    {
        let window_slot = Rc::clone(&window_slot);
        let hold_guard = Rc::clone(&hold_guard);
        let quick_settings = Rc::clone(&quick_settings);
        let tray = Rc::clone(&tray);
        app.connect_startup(move |app| {
            let window = ui::build(
                app,
                Some(Rc::clone(&quick_settings)),
                Some(Rc::clone(&tray)),
            );
            *window_slot.borrow_mut() = Some(window);
            *hold_guard.borrow_mut() = Some(app.hold());
        });
    }

    {
        let window_slot = Rc::clone(&window_slot);
        app.connect_activate(move |_| {
            if let Some(window) = window_slot.borrow().as_ref() {
                window.present();
            }
        });
    }

    app.connect_command_line(move |app, cmdline| {
        let background = cmdline.options_dict().contains("background");
        if !background {
            app.activate();
        }
        gtk::glib::ExitCode::SUCCESS
    });

    app.run()
}
