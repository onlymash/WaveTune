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

use crate::protocol::{FindEarbudsMode, FitQuality, FitTestResult};
use std::{cell::Cell, rc::Rc};

fn find_mode_label(mode: FindEarbudsMode) -> String {
    match mode {
        FindEarbudsMode::Off | FindEarbudsMode::Both => crate::t!("controls-find-both"),
        FindEarbudsMode::Left => crate::t!("controls-find-left"),
        FindEarbudsMode::Right => crate::t!("controls-find-right"),
    }
}

#[derive(Clone)]
pub(crate) struct UtilityWidgets {
    utility_group: adw::PreferencesGroup,
    find_card: adw::ActionRow,
    find_target_button: gtk::MenuButton,
    find_target_label: Label,
    find_find_button: Button,
    find_target: Rc<Cell<FindEarbudsMode>>,
    find_active: Rc<Cell<bool>>,
    pending_find: Rc<Cell<Option<FindEarbudsMode>>>,
    fit_card: adw::ActionRow,
    fit_btn: Button,
    fit_spinner: Spinner,
    fit_result_label: Label,
}

impl UtilityWidgets {
    pub fn set_supported(
        &self,
        supports_find: bool,
        supports_find_sides: bool,
        supports_fit: bool,
    ) {
        self.utility_group
            .set_visible(supports_find || supports_fit);
        self.find_card.set_visible(supports_find);
        self.find_target_button
            .set_visible(supports_find && supports_find_sides);
        self.fit_card.set_visible(supports_fit);
    }

    pub fn set_find_mode(&self, mode: Option<FindEarbudsMode>) {
        if self.pending_find.get().is_some() {
            return;
        }
        if let Some(mode) = mode.filter(|mode| mode.is_finding()) {
            self.find_target.set(mode);
        }
        self.render_find_mode(mode.is_some_and(FindEarbudsMode::is_finding), true);
    }

    pub fn begin_find_request(&self, mode: FindEarbudsMode) {
        self.pending_find.set(Some(mode));
        if mode.is_finding() {
            self.find_target.set(mode);
            self.render_find_mode(true, true);
        } else {
            // Keep Stop visible until the device confirms that playback stopped.
            self.render_find_mode(true, false);
        }
    }

    pub fn complete_find_request(
        &self,
        requested: FindEarbudsMode,
        actual: Option<FindEarbudsMode>,
    ) {
        if self.pending_find.get() != Some(requested) {
            return;
        }
        self.pending_find.set(None);
        self.render_confirmed_find_mode(actual);
    }

    fn render_confirmed_find_mode(&self, mode: Option<FindEarbudsMode>) {
        if let Some(mode) = mode.filter(|mode| mode.is_finding()) {
            self.find_target.set(mode);
        }
        self.render_find_mode(mode.is_some_and(FindEarbudsMode::is_finding), true);
    }

    fn render_find_mode(&self, is_finding: bool, action_sensitive: bool) {
        self.find_target_label
            .set_text(&find_mode_label(self.find_target.get()));
        self.find_active.set(is_finding);
        let action_label = if is_finding {
            crate::t!("controls-find-stop")
        } else {
            crate::t!("controls-find-start")
        };
        self.find_find_button.set_label(&action_label);
        if is_finding {
            self.find_find_button.remove_css_class("suggested-action");
            self.find_find_button.add_css_class("destructive-action");
            self.find_target_button.set_sensitive(false);
        } else {
            self.find_find_button.remove_css_class("destructive-action");
            self.find_find_button.add_css_class("suggested-action");
            self.find_target_button.set_sensitive(true);
        }
        self.find_find_button.set_sensitive(action_sensitive);
    }

    pub fn set_find_sensitive(&self, sensitive: bool) {
        self.find_find_button.set_sensitive(sensitive);
        self.find_target_button
            .set_sensitive(sensitive && !self.find_active.get());
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
        self.find_target.set(FindEarbudsMode::Both);
        self.find_active.set(false);
        self.pending_find.set(None);
        self.find_target_label
            .set_text(&find_mode_label(FindEarbudsMode::Both));
        self.find_find_button
            .set_label(&crate::t!("controls-find-start"));
        self.find_find_button.remove_css_class("destructive-action");
        self.find_find_button.add_css_class("suggested-action");
        self.fit_spinner.set_visible(false);
        self.fit_spinner.set_spinning(false);
        self.fit_result_label.set_text("");
        self.set_find_sensitive(false);
        self.fit_btn.set_sensitive(false);
    }

    pub fn connect_find_clicked<F>(&self, callback: F)
    where
        F: Fn(FindEarbudsMode) + 'static,
    {
        let callback = Rc::new(callback);
        let target = self.find_target.clone();
        let active = self.find_active.clone();
        let button = self.find_find_button.clone();
        button.connect_clicked(move |_| {
            let mode = if active.get() {
                FindEarbudsMode::Off
            } else {
                target.get()
            };
            callback(mode);
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

    let find_card = adw::ActionRow::builder()
        .title(&crate::t!("controls-find-earbuds"))
        .subtitle(&crate::t!("controls-find-warning-body"))
        .build();

    let find_target = Rc::new(Cell::new(FindEarbudsMode::Both));
    let target_popover = gtk::Popover::new();
    let find_target_label = Label::new(Some(&find_mode_label(FindEarbudsMode::Both)));
    let target_popover_box = GtkBox::new(Orientation::Vertical, 8);
    target_popover_box.set_margin_top(8);
    target_popover_box.set_margin_bottom(8);
    target_popover_box.set_margin_start(8);
    target_popover_box.set_margin_end(8);
    target_popover_box.set_size_request(200, -1);
    for (mode, label) in [
        (FindEarbudsMode::Left, crate::t!("controls-find-left")),
        (FindEarbudsMode::Right, crate::t!("controls-find-right")),
        (FindEarbudsMode::Both, crate::t!("controls-find-both")),
    ] {
        let option = Button::with_label(&label);
        option.add_css_class("flat");
        let target = find_target.clone();
        let target_label = find_target_label.clone();
        let target_popover = target_popover.clone();
        option.connect_clicked(move |_| {
            target.set(mode);
            target_label.set_text(&find_mode_label(mode));
            target_popover.popdown();
        });
        target_popover_box.append(&option);
    }
    target_popover.set_child(Some(&target_popover_box));
    let target_button_child = GtkBox::new(Orientation::Horizontal, 4);
    target_button_child.append(&find_target_label);
    target_button_child.append(&gtk::Image::from_icon_name("pan-down-symbolic"));
    let find_target_button = gtk::MenuButton::builder()
        .child(&target_button_child)
        .popover(&target_popover)
        .tooltip_text(&crate::t!("controls-find-target"))
        .valign(Align::Center)
        .build();
    find_target_button.add_css_class("flat");
    let find_find_button = Button::with_label(&crate::t!("controls-find-start"));
    find_find_button.set_sensitive(false);
    find_find_button.add_css_class("suggested-action");
    let find_box = GtkBox::new(Orientation::Horizontal, 8);
    find_box.set_valign(Align::Center);
    find_box.append(&find_target_button);
    find_box.append(&find_find_button);
    find_card.add_suffix(&find_box);
    utility_group.add(&find_card);

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
        find_target_button,
        find_target_label,
        find_find_button,
        find_target,
        find_active: Rc::new(Cell::new(false)),
        pending_find: Rc::new(Cell::new(None)),
        fit_card,
        fit_btn,
        fit_spinner,
        fit_result_label,
    };

    (utility_group, widgets)
}
