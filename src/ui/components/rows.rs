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
use gtk::{Align, CheckButton, Switch};

/// Creates an ActionRow with a trailing Switch toggle, pre-configured with centered alignment.
#[allow(dead_code)]
pub(crate) fn switch_row(title: &str, subtitle: Option<&str>) -> (adw::ActionRow, Switch) {
    let switch = Switch::new();
    switch.set_valign(Align::Center);
    switch.set_sensitive(false);

    let mut builder = adw::ActionRow::builder()
        .title(title)
        .activatable_widget(&switch);
    if let Some(sub) = subtitle {
        builder = builder.subtitle(sub);
    }
    let row = builder.build();
    row.add_suffix(&switch);
    (row, switch)
}

/// Creates an ActionRow with a prefix circular CheckButton (radio), where clicking the row activates the radio.
#[allow(dead_code)]
pub(crate) fn radio_row(title: &str, group: Option<&CheckButton>) -> (adw::ActionRow, CheckButton) {
    let mut builder = CheckButton::builder().valign(Align::Center);
    if let Some(grp) = group {
        builder = builder.group(grp);
    }
    let check = builder.build();
    check.set_sensitive(false);

    let row = adw::ActionRow::builder()
        .title(title)
        .activatable_widget(&check)
        .build();
    row.add_prefix(&check);
    (row, check)
}

/// Creates a modern Adwaita ComboRow dropdown.
#[allow(dead_code)]
pub(crate) fn combo_row(title: &str, subtitle: Option<&str>) -> adw::ComboRow {
    let mut builder = adw::ComboRow::builder().title(title);
    if let Some(sub) = subtitle {
        builder = builder.subtitle(sub);
    }
    let row = builder.build();
    row.set_sensitive(false);
    row
}

pub(crate) trait ComboRowExtManual {
    fn set_items<S: AsRef<str>>(&self, items: &[(u8, S)]);
    fn selected_id(&self) -> Option<u8>;
    fn set_selected_id(&self, id: Option<u8>);
    fn clear_items(&self);
}

impl ComboRowExtManual for adw::ComboRow {
    fn set_items<S: AsRef<str>>(&self, items: &[(u8, S)]) {
        let labels: Vec<&str> = items.iter().map(|(_, l)| l.as_ref()).collect();
        let string_list = gtk::StringList::new(&labels);
        self.set_model(Some(&string_list));
        let ids: Vec<u8> = items.iter().map(|(id, _)| *id).collect();
        unsafe {
            self.set_data("enco-item-ids", ids);
        }
    }

    fn selected_id(&self) -> Option<u8> {
        let pos = self.selected();
        if pos == gtk::INVALID_LIST_POSITION {
            return None;
        }
        let ids = unsafe { self.data::<Vec<u8>>("enco-item-ids") }?;
        let ids = unsafe { ids.as_ref() };
        ids.get(pos as usize).copied()
    }

    fn set_selected_id(&self, id: Option<u8>) {
        let Some(target) = id else {
            self.set_selected(gtk::INVALID_LIST_POSITION);
            return;
        };
        let ids = unsafe { self.data::<Vec<u8>>("enco-item-ids") };
        if let Some(ids) = ids {
            let ids = unsafe { ids.as_ref() };
            if let Some(pos) = ids.iter().position(|&x| x == target) {
                self.set_selected(pos as u32);
                return;
            }
        }
        self.set_selected(gtk::INVALID_LIST_POSITION);
    }

    fn clear_items(&self) {
        self.set_model(None::<&gtk::StringList>);
        unsafe {
            self.set_data("enco-item-ids", Vec::<u8>::new());
        }
    }
}
