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
use gtk::{Align, Box as GtkBox, FlowBox, Label, Orientation, SelectionMode};

use super::styles::muted;

#[derive(Clone)]
pub(crate) struct AdaptiveColumns {
    container: GtkBox,
    left: GtkBox,
    right: GtkBox,
}

impl AdaptiveColumns {
    pub(crate) fn new() -> Self {
        let left = GtkBox::new(Orientation::Vertical, 16);
        left.add_css_class("adaptive-column");
        left.set_hexpand(true);
        left.set_vexpand(false);
        left.set_halign(Align::Fill);
        left.set_valign(Align::Start);

        let right = GtkBox::new(Orientation::Vertical, 16);
        right.add_css_class("adaptive-column");
        right.set_hexpand(true);
        right.set_vexpand(false);
        right.set_halign(Align::Fill);
        right.set_valign(Align::Start);

        let container = GtkBox::new(Orientation::Horizontal, 16);
        container.set_hexpand(true);
        container.set_vexpand(false);
        container.set_halign(Align::Fill);
        container.set_valign(Align::Start);
        container.set_homogeneous(true);
        container.append(&left);
        container.append(&right);

        Self {
            container,
            left,
            right,
        }
    }

    pub(crate) fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub(crate) fn container(&self) -> &GtkBox {
        &self.container
    }

    pub(crate) fn left(&self) -> &GtkBox {
        &self.left
    }

    pub(crate) fn right(&self) -> &GtkBox {
        &self.right
    }

    pub(crate) fn set_single_column(&self, is_single: bool) {
        if is_single {
            self.container.set_orientation(Orientation::Vertical);
            self.container.set_homogeneous(false);
        } else {
            self.container.set_orientation(Orientation::Horizontal);
            self.container.set_homogeneous(true);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_single_column(&self) -> bool {
        self.container.orientation() == Orientation::Vertical
    }
}

pub(crate) fn page_container() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 16);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(20);
    page.set_margin_end(20);
    page.set_halign(Align::Fill);
    page.set_valign(Align::Fill);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page
}

#[allow(dead_code)]
pub(crate) fn page_title(title: &str, subtitle: &str) -> GtkBox {
    let block = GtkBox::new(Orientation::Vertical, 4);
    block.append(
        &Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["page-title"])
            .build(),
    );
    block.append(&muted(subtitle));
    block
}

pub(crate) fn card() -> GtkBox {
    let wrapper = GtkBox::new(Orientation::Vertical, 0);
    wrapper.set_hexpand(true);
    wrapper.set_vexpand(false);
    wrapper.set_halign(Align::Fill);
    wrapper.set_css_classes(&["card", "app-card"]);
    wrapper.set_margin_top(2);
    wrapper.set_margin_bottom(2);
    wrapper.set_margin_start(2);
    wrapper.set_margin_end(2);
    wrapper
}

/// Wraps any widget in an inert `adw::PreferencesRow` to be added into an `adw::PreferencesGroup`.
#[allow(dead_code)]
pub(crate) fn preferences_content_row(child: &impl IsA<gtk::Widget>) -> adw::PreferencesRow {
    adw::PreferencesRow::builder()
        .selectable(false)
        .activatable(false)
        .child(child)
        .build()
}

/// Creates a standardized FlowBox for pill buttons, mode selectors, or tags.
#[allow(dead_code)]
pub(crate) fn flow_button_box(min_per_line: u32, max_per_line: u32) -> FlowBox {
    FlowBox::builder()
        .selection_mode(SelectionMode::None)
        .homogeneous(false)
        .min_children_per_line(min_per_line)
        .max_children_per_line(max_per_line)
        .row_spacing(8)
        .column_spacing(8)
        .build()
}
