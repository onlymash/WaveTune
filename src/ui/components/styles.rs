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

use gtk::prelude::*;
use gtk::{Button, Label};

pub(crate) fn install_styles() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        .page-title { font-size: 1.65rem; font-weight: 700; }
        .section-title { font-size: 1.12rem; font-weight: 650; }
        .battery-value { font-size: 0.95rem; font-weight: 600; }
        .device-name { font-size: 1.25rem; font-weight: 650; }
        .battery-device-icon {
            min-width: 20px;
            min-height: 20px;
            border-radius: 999px;
            background-color: alpha(@theme_fg_color, 0.08);
        }
        .battery-device-letter { font-size: 0.65rem; font-weight: 650; }
        .battery-charging { color: #2ec27e; font-size: 0.9rem; }
        .app-card { padding: 20px; min-width: 0; }
        .adaptive-column { min-width: 0; }
        .model-card { min-height: 240px; padding: 6px 8px; min-width: 0; }
        .paired-device-row { padding: 10px 4px; }
        .paired-device-emoji { font-size: 1.25rem; }
        .paired-device-name { font-weight: 600; }
        .noise-options-title { font-size: 0.85rem; font-weight: 600; margin-top: 4px; }
        .noise-mode-button {
            min-width: 52px;
            min-height: 52px;
            padding: 0;
            border-radius: 9999px;
        }
        .noise-mode-button image {
            -gtk-icon-size: 22px;
        }
        .noise-mode-label {
            font-size: 0.82rem;
            font-weight: 550;
            margin-top: 4px;
        }
        .option-title { font-size: 0.98rem; font-weight: 600; }
        .connection-tag {
            font-size: 0.75rem;
            font-weight: 600;
            padding: 2px 8px;
            border-radius: 6px;
        }
        .connection-tag-connected {
            background-color: alpha(#2ec27e, 0.16);
            color: #2ec27e;
        }
        .connection-tag-disconnected {
            background-color: alpha(@theme_fg_color, 0.08);
            color: alpha(@theme_fg_color, 0.55);
        }
        scale.labeled-slider value {
            margin-bottom: 18px;
            padding-bottom: 0px;
        }
        scale.labeled-slider marks {
            margin-top: 4px;
            padding-top: 0px;
        }
        scale.labeled-slider {
            padding-bottom: 38px;
        }
        ",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[allow(dead_code)]
pub(crate) fn heading(text: &str) -> Label {
    let label = Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(["section-title"])
        .build();
    configure_label_wrapping(&label);
    label
}

#[allow(dead_code)]
pub(crate) fn subheading(text: &str) -> Label {
    let label = Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(["option-title"])
        .build();
    configure_label_wrapping(&label);
    label
}

pub(crate) fn muted(text: &str) -> Label {
    let label = Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();
    configure_label_wrapping(&label);
    label
}

pub(crate) fn configure_label_wrapping(label: &Label) {
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_width_chars(1);
    label.set_max_width_chars(48);
}

#[allow(dead_code)]
pub(crate) fn configure_button_ellipsize(button: &Button) {
    if let Some(child) = button.child() {
        if let Some(label) = child.downcast_ref::<Label>() {
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(1);
        }
    }
}
