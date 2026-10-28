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

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build);
    app.run()
}
