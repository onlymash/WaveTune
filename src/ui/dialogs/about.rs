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

pub fn show_about_dialog(parent: &impl IsA<gtk::Widget>) {
    let icon_name = if let Some(display) = gtk::gdk::Display::default() {
        let theme = gtk::IconTheme::for_display(&display);
        if theme.has_icon("io.github.onlymash.WaveTune") {
            "io.github.onlymash.WaveTune"
        } else {
            "audio-headphones-symbolic"
        }
    } else {
        "audio-headphones-symbolic"
    };

    let dialog = adw::AboutDialog::builder()
        .application_name("WaveTune")
        .application_icon(icon_name)
        .developer_name("onlymash")
        .version(env!("CARGO_PKG_VERSION"))
        .website("https://github.com/onlymash/WaveTune")
        .issue_url("https://github.com/onlymash/WaveTune/issues")
        .support_url("https://github.com/onlymash/WaveTune/discussions")
        .license_type(gtk::License::Gpl30Only)
        .copyright("© 2026 onlymash")
        .comments(&crate::t!("about-description"))
        .developers(["onlymash https://github.com/onlymash"])
        .build();

    dialog.present(Some(parent));
}
