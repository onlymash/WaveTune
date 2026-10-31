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
use gtk::{Align, Box as GtkBox, Button, Label, Orientation, Spinner};

use crate::protocol::{FitQuality, FitTestResult};

#[derive(Clone)]
pub(crate) struct UtilityWidgets {
    utility_group: adw::PreferencesGroup,
    find_card: adw::ActionRow,
    find_btn: Button,
    find_status_label: Label,
    fit_card: adw::ActionRow,
    fit_btn: Button,
    fit_spinner: Spinner,
    fit_result_label: Label,
}

impl UtilityWidgets {
    pub fn set_supported(&self, supports_find: bool, supports_fit: bool) {
        self.utility_group
            .set_visible(supports_find || supports_fit);
        self.find_card.set_visible(supports_find);
        self.fit_card.set_visible(supports_fit);
    }

    pub fn set_finding(&self, finding: bool) {
        if finding {
            self.find_btn.set_label(&crate::t!("controls-find-stop"));
            self.find_btn.add_css_class("destructive-action");
            self.find_status_label
                .set_text(&crate::t!("controls-find-playing-note"));
            self.find_status_label.set_visible(true);
        } else {
            self.find_btn.set_label(&crate::t!("controls-find-play"));
            self.find_btn.remove_css_class("destructive-action");
            self.find_status_label.set_visible(false);
        }
        self.find_btn.set_sensitive(true);
    }

    pub fn set_find_sensitive(&self, sensitive: bool) {
        self.find_btn.set_sensitive(sensitive);
    }

    pub fn find_button(&self) -> &Button {
        &self.find_btn
    }

    pub fn set_fit_running(&self, running: bool) {
        if running {
            self.fit_btn.set_sensitive(false);
            self.fit_spinner.set_spinning(true);
            self.fit_spinner.set_visible(true);
            self.fit_result_label
                .set_text(&crate::t!("controls-fit-test-running"));
        } else {
            self.fit_btn.set_sensitive(true);
            self.fit_spinner.set_spinning(false);
            self.fit_spinner.set_visible(false);
        }
    }

    pub fn set_fit_result(&self, result: FitTestResult) {
        let describe_quality = |q: FitQuality| match q {
            FitQuality::Good => crate::t!("controls-fit-test-good"),
            FitQuality::Average => crate::t!("controls-fit-test-average"),
            FitQuality::Poor => crate::t!("controls-fit-test-poor"),
            _ => crate::t!("controls-fit-test-failed"),
        };

        let text = if result.left == FitQuality::Good && result.right == FitQuality::Good {
            crate::t!("controls-fit-test-good")
        } else if result.left == FitQuality::Failed || result.right == FitQuality::Failed {
            crate::t!("controls-fit-test-failed")
        } else {
            format!(
                "{}: {} · {}: {}",
                crate::t!("controls-ear-left"),
                describe_quality(result.left),
                crate::t!("controls-ear-right"),
                describe_quality(result.right)
            )
        };
        self.fit_result_label.set_text(&text);
    }

    pub fn set_fit_sensitive(&self, sensitive: bool) {
        self.fit_btn.set_sensitive(sensitive);
    }

    pub fn reset(&self) {
        self.utility_group.set_visible(false);
        self.find_status_label.set_visible(false);
        self.fit_spinner.set_visible(false);
        self.fit_spinner.set_spinning(false);
        self.fit_result_label.set_text("");
        self.find_btn.set_sensitive(false);
        self.fit_btn.set_sensitive(false);
    }

    pub fn connect_find_clicked<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.find_btn.connect_clicked(move |_| {
            callback();
        });
    }

    pub fn connect_fit_clicked<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.fit_btn.connect_clicked(move |_| {
            callback();
        });
    }
}

pub(crate) fn build_utilities_group() -> (adw::PreferencesGroup, UtilityWidgets) {
    let utility_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("controls-headset-utilities"))
        .description(&crate::t!("controls-find-and-seal-desc"))
        .build();

    // Find My Earbuds ActionRow
    let find_card = adw::ActionRow::builder()
        .title(&crate::t!("controls-find-earbuds"))
        .subtitle(&crate::t!("controls-find-and-seal-desc"))
        .build();
    let find_box = GtkBox::new(Orientation::Vertical, 4);
    find_box.set_valign(Align::Center);
    let find_btn = Button::with_label(&crate::t!("controls-find-play"));
    find_btn.set_sensitive(false);
    let find_status_label = Label::new(None);
    find_status_label.set_visible(false);
    find_status_label.add_css_class("caption");
    find_status_label.add_css_class("accent");
    find_box.append(&find_btn);
    find_box.append(&find_status_label);
    find_card.add_suffix(&find_box);
    utility_group.add(&find_card);

    // Fit Test ActionRow
    let fit_card = adw::ActionRow::builder()
        .title(&crate::t!("controls-fit-test-title"))
        .subtitle(&crate::t!("controls-fit-test-desc"))
        .build();
    let fit_box = GtkBox::new(Orientation::Horizontal, 8);
    fit_box.set_valign(Align::Center);
    let fit_spinner = Spinner::new();
    fit_spinner.set_visible(false);
    let fit_btn = Button::with_label(&crate::t!("controls-fit-test-start"));
    fit_btn.set_sensitive(false);
    let fit_result_label = Label::new(None);
    fit_result_label.add_css_class("dim-label");
    fit_box.append(&fit_result_label);
    fit_box.append(&fit_spinner);
    fit_box.append(&fit_btn);
    fit_card.add_suffix(&fit_box);
    utility_group.add(&fit_card);

    let widgets = UtilityWidgets {
        utility_group: utility_group.clone(),
        find_card,
        find_btn,
        find_status_label,
        fit_card,
        fit_btn,
        fit_spinner,
        fit_result_label,
    };

    (utility_group, widgets)
}
