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

pub fn show_preferences_dialog(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::PreferencesDialog::builder()
        .title(&crate::t!("pref-preferences"))
        .build();

    let page = adw::PreferencesPage::builder()
        .title(&crate::t!("pref-general"))
        .icon_name("preferences-other-symbolic")
        .build();

    let group = adw::PreferencesGroup::builder()
        .title(&crate::t!("pref-general"))
        .build();

    let languages = [
        crate::t!("pref-language-auto"),
        "English".to_string(),
        "简体中文".to_string(),
    ];
    let string_list =
        gtk::StringList::new(&languages.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    let current_configured = crate::i18n::get_configured_language();
    let initial_index = match current_configured.as_deref() {
        Some(lang) if lang.starts_with("en") => 1,
        Some(lang) if lang.starts_with("zh") => 2,
        _ => 0,
    };

    let language_row = adw::ComboRow::builder()
        .title(&crate::t!("pref-language"))
        .subtitle(&crate::t!("pref-language-desc"))
        .model(&string_list)
        .selected(initial_index)
        .build();

    let initial_selected = std::cell::Cell::new(initial_index);
    let dialog_weak = dialog.downgrade();
    language_row.connect_selected_notify(move |row| {
        let selected = row.selected();
        if selected == initial_selected.get() {
            return;
        }
        initial_selected.set(selected);
        let lang_code = match selected {
            1 => Some("en-US"),
            2 => Some("zh-CN"),
            _ => None,
        };
        crate::i18n::set_configured_language(lang_code);
        row.set_subtitle(&crate::t!("pref-language-restart-hint"));

        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.add_toast(adw::Toast::new(&crate::t!("pref-language-restart-hint")));
        }
    });

    group.add(&language_row);
    page.add(&group);
    dialog.add(&page);

    dialog.present(Some(parent));
}
