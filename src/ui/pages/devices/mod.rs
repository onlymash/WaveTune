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
use gtk::{Box as GtkBox, Orientation};

use crate::ui::{
    DevicesWidgets,
    components::{card, muted, page_container},
};

pub(crate) fn build_devices_page() -> (GtkBox, DevicesWidgets) {
    let page = page_container();

    let device_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("channel-bluetooth-connection"))
        .build();
    let device = card();
    let content = GtkBox::new(Orientation::Vertical, 12);
    let connection = muted(&crate::t!("device-status-disconnected-generic"));
    let name = muted(&crate::t!("device-name-placeholder"));
    let product_id = muted(&crate::t!("device-model-placeholder"));
    let firmware = muted(&crate::t!("firmware-version-placeholder"));
    let wear = muted(&crate::t!("wear-status-placeholder"));
    content.append(&connection);
    content.append(&name);
    content.append(&product_id);
    content.append(&firmware);
    content.append(&wear);
    device.append(&content);
    device_group.add(&device);
    page.append(&device_group);

    let notice_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("device-compatibility"))
        .build();
    let notice = card();
    let note = GtkBox::new(Orientation::Vertical, 8);
    note.append(&muted(&crate::t!("device-catalog-desc")));
    notice.append(&note);
    notice_group.add(&notice);
    page.append(&notice_group);

    let diag_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("channel-and-diagnostics"))
        .build();
    let diag = card();
    let diag_content = GtkBox::new(Orientation::Vertical, 8);
    diag_content.append(&muted(&crate::t!("channel-desc")));
    diag.append(&diag_content);
    diag_group.add(&diag);
    page.append(&diag_group);
    (
        page,
        DevicesWidgets {
            connection,
            name,
            product_id,
            firmware,
            wear,
        },
    )
}
