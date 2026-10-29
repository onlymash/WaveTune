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

    let icon_labels = [
        crate::t!("pref-app-icon-default"),
        crate::t!("pref-app-icon-circle"),
        crate::t!("pref-app-icon-tws"),
        crate::t!("pref-app-icon-tws-circle"),
    ];
    let icon_list =
        gtk::StringList::new(&icon_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    let current_icon_style = crate::resources::get_configured_icon_style();
    let initial_icon_index = current_icon_style.to_index();

    let icon_preview = gtk::Image::builder()
        .icon_name(current_icon_style.icon_name())
        .pixel_size(32)
        .build();

    let icon_row = adw::ComboRow::builder()
        .title(&crate::t!("pref-app-icon"))
        .subtitle(&crate::t!("pref-app-icon-desc"))
        .model(&icon_list)
        .selected(initial_icon_index)
        .build();
    icon_row.add_prefix(&icon_preview);
    crate::ui::components::enable_combo_row_subtitle_wrap(&icon_row);

    let initial_icon_selected = std::cell::Cell::new(initial_icon_index);
    let dialog_weak_for_icon = dialog.downgrade();
    icon_row.connect_selected_notify(move |row| {
        let selected = row.selected();
        if selected == initial_icon_selected.get() {
            return;
        }
        initial_icon_selected.set(selected);
        let style = crate::resources::AppIconStyle::from_index(selected);
        icon_preview.set_icon_name(Some(style.icon_name()));

        match crate::resources::set_configured_icon_style(style) {
            Ok(()) => {
                row.set_subtitle(&crate::t!("pref-app-icon-applied-hint"));
            }
            Err(err) => {
                let message = crate::t!("pref-app-icon-apply-failed", err);
                row.set_subtitle(&message);
                if let Some(dialog) = dialog_weak_for_icon.upgrade() {
                    dialog.add_toast(adw::Toast::new(&message));
                }
            }
        }
    });

    group.add(&icon_row);

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
    crate::ui::components::enable_combo_row_subtitle_wrap(&language_row);

    let initial_selected = std::cell::Cell::new(initial_index);
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
    });

    group.add(&language_row);

    let bg_row = adw::SwitchRow::builder()
        .title(&crate::t!("pref-run-in-background"))
        .subtitle(&crate::t!("pref-run-in-background-desc"))
        .active(crate::resources::get_run_in_background())
        .build();
    let dialog_weak_for_bg = dialog.downgrade();
    bg_row.connect_active_notify(move |row| {
        if let Err(err) = crate::resources::set_run_in_background(row.is_active()) {
            let message = crate::t!("pref-save-failed", err);
            if let Some(dialog) = dialog_weak_for_bg.upgrade() {
                dialog.add_toast(adw::Toast::new(&message));
            }
        }
    });
    group.add(&bg_row);

    let autostart_row = adw::SwitchRow::builder()
        .title(&crate::t!("pref-autostart-background"))
        .subtitle(&crate::t!("pref-autostart-background-desc"))
        .active(crate::resources::get_autostart_background())
        .build();
    let dialog_weak_for_autostart = dialog.downgrade();
    autostart_row.connect_active_notify(move |row| {
        if let Err(err) = crate::resources::set_autostart_background(row.is_active()) {
            let message = crate::t!("pref-save-failed", err);
            if let Some(dialog) = dialog_weak_for_autostart.upgrade() {
                dialog.add_toast(adw::Toast::new(&message));
            }
        }
    });
    group.add(&autostart_row);

    page.add(&group);
    dialog.add(&page);

    dialog.present(Some(parent));
}
