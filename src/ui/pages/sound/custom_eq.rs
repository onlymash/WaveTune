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

use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, Entry, ListBox, Orientation, Scale};

use crate::ui::components::muted;

#[derive(Clone)]
pub(crate) struct CustomEqualizerWidgets {
    preset_rows: Rc<RefCell<Vec<(u8, CheckButton, adw::ActionRow)>>>,
    custom_list: ListBox,
    add_custom_row: adw::ActionRow,
    custom_editor: GtkBox,
}

impl CustomEqualizerWidgets {
    pub fn update_features(&self, supports_custom_equalizer: bool, custom_count: usize) {
        let can_add_custom = supports_custom_equalizer && custom_count < 3;
        self.add_custom_row.set_visible(can_add_custom);
        self.add_custom_row.set_sensitive(can_add_custom);
        if !supports_custom_equalizer {
            self.custom_list.set_visible(false);
            self.custom_editor.set_visible(false);
        }
    }

    pub fn set_selected(&self, flag: &crate::ui::ReentrancyFlag, selected_preset: Option<u8>) {
        let _guard = flag.enter();
        if let Some(selected) = selected_preset {
            for (idx, radio, _) in self.preset_rows.borrow().iter() {
                if *idx == selected {
                    radio.set_active(true);
                }
            }
        }
    }

    pub fn update_presets<FSelect, FEdit, FDelete>(
        &self,
        preset_list: &ListBox,
        first_radio: &CheckButton,
        eq_flag: &crate::ui::ReentrancyFlag,
        custom_eqs: &[crate::protocol::CustomEqualizer],
        supports_custom_equalizer: bool,
        on_select: FSelect,
        on_edit: FEdit,
        on_delete: FDelete,
    ) where
        FSelect: Fn(u8) + Clone + 'static,
        FEdit: Fn(&crate::protocol::CustomEqualizer, &gtk::Widget) + Clone + 'static,
        FDelete: Fn(u8, &gtk::Widget) + Clone + 'static,
    {
        // 1. Remove old custom preset rows from preset_list
        {
            let mut old_rows = self.preset_rows.borrow_mut();
            for (_, _, row) in old_rows.drain(..) {
                preset_list.remove(&row);
            }
        }

        // 2. Insert new custom preset rows before add_custom_row
        let insert_pos = self.add_custom_row.index();
        let mut new_rows = Vec::new();

        let _guard = eq_flag.enter();
        for (i, entry) in custom_eqs.iter().enumerate() {
            let (row, radio) = crate::ui::components::radio_row(&entry.name, Some(first_radio));
            radio.set_sensitive(supports_custom_equalizer);

            // Menu button with popover
            let popover = gtk::Popover::new();
            let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
            menu_box.set_margin_top(4);
            menu_box.set_margin_bottom(4);
            menu_box.set_margin_start(4);
            menu_box.set_margin_end(4);

            let edit_btn = gtk::Button::builder()
                .label(&crate::t!("equalizer-custom-modify"))
                .has_frame(false)
                .build();
            edit_btn.set_halign(gtk::Align::Fill);

            let delete_btn = gtk::Button::builder()
                .label(&crate::t!("equalizer-custom-delete"))
                .has_frame(false)
                .build();
            delete_btn.add_css_class("destructive-action");
            delete_btn.set_halign(gtk::Align::Fill);

            menu_box.append(&edit_btn);
            menu_box.append(&delete_btn);
            popover.set_child(Some(&menu_box));

            let menu_btn = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .popover(&popover)
                .valign(gtk::Align::Center)
                .has_frame(false)
                .tooltip_text(&crate::t!("common-options", "EQ"))
                .build();

            row.add_suffix(&menu_btn);

            // Wiring signals
            {
                let on_select = on_select.clone();
                let id = entry.id;
                let eq_flag = eq_flag.clone();
                radio.connect_toggled(move |r| {
                    if r.is_active() && !eq_flag.is_updating() {
                        on_select(id);
                    }
                });
            }

            {
                let on_edit = on_edit.clone();
                let entry_clone = entry.clone();
                let popover_weak = popover.downgrade();
                let row_widget = row.clone();
                edit_btn.connect_clicked(move |_| {
                    if let Some(pop) = popover_weak.upgrade() {
                        pop.popdown();
                    }
                    on_edit(&entry_clone, row_widget.upcast_ref());
                });
            }

            {
                let on_delete = on_delete.clone();
                let id = entry.id;
                let popover_weak = popover.downgrade();
                let row_widget = row.clone();
                delete_btn.connect_clicked(move |_| {
                    if let Some(pop) = popover_weak.upgrade() {
                        pop.popdown();
                    }
                    on_delete(id, row_widget.upcast_ref());
                });
            }

            if entry.is_selected {
                radio.set_active(true);
            }

            let pos = if insert_pos >= 0 {
                insert_pos + i as i32
            } else {
                -1
            };
            preset_list.insert(&row, pos);
            new_rows.push((entry.id, radio, row));
        }
        drop(_guard);

        self.preset_rows.replace(new_rows);

        // 3. Hide add_custom_row if max presets reached
        let max_reached = custom_eqs.len() >= 3;
        let can_add = supports_custom_equalizer && !max_reached;
        self.add_custom_row.set_visible(can_add);
        self.add_custom_row.set_sensitive(can_add);
    }

    pub fn reset(&self, preset_list: &ListBox) {
        let mut old_rows = self.preset_rows.borrow_mut();
        for (_, _, row) in old_rows.drain(..) {
            preset_list.remove(&row);
        }
        self.add_custom_row.set_visible(false);
        self.add_custom_row.set_sensitive(false);
        self.custom_list.set_visible(false);
        self.custom_editor.set_visible(false);
    }

    // Accessors for assembling into parent eq_card
    pub(crate) fn add_custom_row(&self) -> &adw::ActionRow {
        &self.add_custom_row
    }

    pub(crate) fn custom_list(&self) -> &ListBox {
        &self.custom_list
    }

    pub(crate) fn custom_editor(&self) -> &GtkBox {
        &self.custom_editor
    }
}

#[cfg(test)]
impl CustomEqualizerWidgets {
    pub fn preset_count(&self) -> usize {
        self.preset_rows.borrow().len()
    }

    pub fn is_add_visible(&self) -> bool {
        self.add_custom_row.is_visible()
    }

    pub fn is_add_sensitive(&self) -> bool {
        self.add_custom_row.is_sensitive()
    }

    pub fn is_editor_visible(&self) -> bool {
        self.custom_editor.is_visible()
    }

    pub fn is_preset_active(&self, index: usize) -> bool {
        self.preset_rows
            .borrow()
            .get(index)
            .map(|(_, r, _)| r.is_active())
            .unwrap_or(false)
    }
}

pub(crate) fn build_custom_eq_section() -> CustomEqualizerWidgets {
    let add_custom_row = adw::ActionRow::builder()
        .title(&crate::t!("equalizer-add-custom"))
        .activatable(true)
        .visible(false)
        .build();
    add_custom_row.set_sensitive(false);
    let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
    add_custom_row.add_prefix(&add_icon);

    let custom_list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();

    let custom_editor = GtkBox::new(Orientation::Vertical, 10);
    custom_editor.set_visible(false);

    let editor_top_row = GtkBox::new(Orientation::Horizontal, 8);
    let custom_name_entry = Entry::builder()
        .placeholder_text(&crate::t!("equalizer-custom-title"))
        .hexpand(true)
        .max_length(42)
        .build();
    let rename_btn = Button::with_label(&crate::t!("common-rename"));
    let delete_btn = Button::with_label(&crate::t!("common-delete"));
    delete_btn.add_css_class("destructive-action");
    editor_top_row.append(&custom_name_entry);
    editor_top_row.append(&rename_btn);
    editor_top_row.append(&delete_btn);
    custom_editor.append(&editor_top_row);

    // 10 Frequency Bands: 31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz
    let band_box = GtkBox::new(Orientation::Horizontal, 6);
    band_box.set_homogeneous(true);
    let frequencies = [
        "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ];
    for freq in frequencies {
        let band_col = GtkBox::new(Orientation::Vertical, 4);
        band_col.set_halign(gtk::Align::Center);
        let val_label = muted("0");
        val_label.set_halign(gtk::Align::Center);
        let scale = Scale::with_range(Orientation::Vertical, -6.0, 6.0, 1.0);
        scale.set_inverted(true);
        scale.set_height_request(130);
        scale.set_draw_value(false);
        scale.set_sensitive(false);
        let freq_label = muted(freq);
        freq_label.set_halign(gtk::Align::Center);

        band_col.append(&val_label);
        band_col.append(&scale);
        band_col.append(&freq_label);
        band_box.append(&band_col);
    }
    custom_editor.append(&band_box);

    CustomEqualizerWidgets {
        preset_rows: Rc::new(RefCell::new(Vec::new())),
        custom_list,
        add_custom_row,
        custom_editor,
    }
}
