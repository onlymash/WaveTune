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

pub(crate) mod state;

use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Button, Label, Orientation, Stack};

use crate::{
    compatibility,
    ui::{
        HomeWidgets, PrimaryNoiseControl,
        components::{
            AdaptiveColumns, battery_indicator, combo_row, configure_label_wrapping,
            page_container, preferences_content_row, switch_row,
        },
        scene3d::DeviceSceneView,
    },
};

pub(crate) fn build_home_page() -> (GtkBox, HomeWidgets) {
    let page = page_container();

    let refresh_devices = Button::from_icon_name("view-refresh-symbolic");
    refresh_devices.set_tooltip_text(Some(&crate::t!("device-refresh-list")));
    refresh_devices.set_valign(Align::Center);
    refresh_devices.add_css_class("flat");

    let device_popover = gtk::Popover::new();
    let popover_box = GtkBox::new(Orientation::Vertical, 8);
    popover_box.set_margin_top(8);
    popover_box.set_margin_bottom(8);
    popover_box.set_margin_start(8);
    popover_box.set_margin_end(8);
    popover_box.set_size_request(280, -1);

    let popover_header = GtkBox::new(Orientation::Horizontal, 8);
    let popover_title = Label::builder()
        .label(&crate::t!("device-title"))
        .css_classes(["heading"])
        .halign(Align::Start)
        .hexpand(true)
        .build();
    popover_header.append(&popover_title);
    popover_header.append(&refresh_devices);
    popover_box.append(&popover_header);

    let device_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    popover_box.append(&device_list);

    let device_list_empty = Label::builder()
        .label(&crate::t!("device-no-supported-connected"))
        .css_classes(["dim-label"])
        .wrap(true)
        .xalign(0.5)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    popover_box.append(&device_list_empty);

    device_popover.set_child(Some(&popover_box));

    let device_menu_button = gtk::MenuButton::builder()
        .icon_name("audio-headphones-symbolic")
        .tooltip_text(&crate::t!("device-title"))
        .popover(&device_popover)
        .valign(Align::Center)
        .build();

    let device_row = GtkBox::new(Orientation::Horizontal, 16);
    device_row.set_hexpand(true);
    device_row.set_halign(Align::Fill);
    device_row.set_valign(Align::Center);
    device_row.set_margin_top(12);
    device_row.set_margin_bottom(12);
    device_row.set_margin_start(16);
    device_row.set_margin_end(16);
    let device_info = GtkBox::new(Orientation::Vertical, 6);
    device_info.set_valign(Align::Center);
    device_info.set_hexpand(true);
    let device_name = Label::builder()
        .label(&crate::t!("device-no-supported-connected"))
        .xalign(0.0)
        .css_classes(["device-name"])
        .build();
    configure_label_wrapping(&device_name);
    let connection_tag = Label::builder()
        .label(&crate::t!("common-disconnected"))
        .css_classes(["connection-tag", "connection-tag-disconnected"])
        .halign(Align::Start)
        .valign(Align::Center)
        .build();
    let batteries = GtkBox::new(Orientation::Horizontal, 12);
    batteries.set_halign(Align::Start);
    let (left_battery_widget, left_battery) = battery_indicator(Some("L"));
    let (right_battery_widget, right_battery) = battery_indicator(Some("R"));
    let (case_battery_widget, case_battery) = battery_indicator(None);
    batteries.append(&left_battery_widget);
    batteries.append(&right_battery_widget);
    batteries.append(&case_battery_widget);
    device_info.append(&device_name);
    device_info.append(&connection_tag);
    device_info.append(&batteries);
    device_row.append(&device_info);

    let model_card = GtkBox::new(Orientation::Vertical, 0);
    model_card.add_css_class("model-card");
    model_card.set_vexpand(false);
    let model_content = GtkBox::new(Orientation::Vertical, 0);
    model_content.set_hexpand(true);
    model_content.set_vexpand(true);
    model_content.set_valign(Align::Fill);
    model_content.set_halign(Align::Fill);

    let overlay = gtk::Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    overlay.set_halign(Align::Fill);
    overlay.set_valign(Align::Fill);

    let model_placeholder = gtk::Image::from_icon_name("audio-headphones-symbolic");
    model_placeholder.set_pixel_size(112);
    model_placeholder.add_css_class("dim-label");
    model_placeholder.set_valign(Align::Center);
    model_placeholder.set_halign(Align::Center);
    model_placeholder.set_opacity(0.35);

    let model_picture = gtk::Picture::new();
    model_picture.set_content_fit(gtk::ContentFit::Contain);
    model_picture.set_hexpand(true);
    model_picture.set_vexpand(true);
    model_picture.set_halign(Align::Fill);
    model_picture.set_valign(Align::Fill);

    let scene_view = DeviceSceneView::new();

    let model_stack = Stack::new();
    model_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    model_stack.set_hexpand(true);
    model_stack.set_vexpand(true);
    model_stack.set_halign(Align::Fill);
    model_stack.set_valign(Align::Fill);
    model_stack.add_named(&model_picture, Some("picture"));
    model_stack.add_named(scene_view.widget(), Some("scene3d"));
    model_stack.set_visible_child_name("picture");

    let ph = model_placeholder.clone();
    let sv = scene_view.clone();
    model_picture.connect_paintable_notify(move |pic| {
        if sv.has_scene() {
            ph.set_visible(false);
        } else {
            ph.set_visible(pic.paintable().is_none());
        }
    });

    overlay.set_child(Some(&model_stack));
    overlay.add_overlay(&model_placeholder);

    let aspect_frame = gtk::AspectFrame::builder()
        .ratio(1.25)
        .obey_child(false)
        .xalign(0.5)
        .yalign(0.5)
        .hexpand(true)
        .vexpand(true)
        .valign(Align::Fill)
        .halign(Align::Fill)
        .child(&overlay)
        .build();

    let height_clamp = adw::Clamp::builder()
        .orientation(Orientation::Vertical)
        .maximum_size(420)
        .tightening_threshold(420)
        .hexpand(true)
        .vexpand(false)
        .valign(Align::Fill)
        .child(&aspect_frame)
        .build();

    model_content.append(&height_clamp);

    let model_firmware = Label::builder()
        .halign(Align::Center)
        .valign(Align::Center)
        .margin_top(4)
        .margin_bottom(8)
        .css_classes(["dim-label", "caption"])
        .visible(false)
        .build();
    model_content.append(&model_firmware);

    model_card.append(&model_content);

    let active_turntable: std::rc::Rc<
        std::cell::RefCell<Option<std::rc::Rc<crate::resources::turntable::TurntableModel>>>,
    > = std::rc::Rc::new(std::cell::RefCell::new(None));
    let current_frame = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let drag_start_frame = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let turntable_state =
        std::rc::Rc::new(std::cell::RefCell::new(state::TurntableState::default()));

    let drag = gtk::GestureDrag::new();
    let tt_drag = std::rc::Rc::clone(&active_turntable);
    let frame_drag = std::rc::Rc::clone(&current_frame);
    let start_frame = std::rc::Rc::clone(&drag_start_frame);
    let mc_drag = model_card.clone();
    let state_drag = std::rc::Rc::clone(&turntable_state);

    drag.connect_drag_begin(move |_, _, _| {
        if tt_drag.borrow().is_some() {
            start_frame.set(frame_drag.get());
            mc_drag.set_cursor_from_name(Some("grabbing"));
            let mut st = state_drag.borrow_mut();
            st.is_dragging = true;
            st.accumulator = 0.0;
        }
    });

    let tt_update = std::rc::Rc::clone(&active_turntable);
    let frame_update = std::rc::Rc::clone(&current_frame);
    let start_update = std::rc::Rc::clone(&drag_start_frame);
    let pic_update = model_picture.clone();
    let mc_update = model_card.clone();

    drag.connect_drag_update(move |_, offset_x, _| {
        let tt_guard = tt_update.borrow();
        if let Some(ref tt) = *tt_guard {
            let total = tt.frame_count();
            if total > 0 {
                let width = mc_update.allocated_width().max(200) as f64;
                let delta = (offset_x * (total as f64) / (width * 1.6)).round() as isize;
                let new_frame =
                    (start_update.get() as isize + delta).rem_euclid(total as isize) as usize;
                if new_frame != frame_update.get() {
                    frame_update.set(new_frame);
                    if let Some(tex) = tt.get_texture(new_frame) {
                        pic_update.set_paintable(Some(&tex));
                    }
                }
            }
        }
    });

    let tt_end = std::rc::Rc::clone(&active_turntable);
    let mc_end = model_card.clone();
    let state_end = std::rc::Rc::clone(&turntable_state);

    drag.connect_drag_end(move |_, _, _| {
        if tt_end.borrow().is_some() {
            mc_end.set_cursor_from_name(Some("grab"));
            let mut st = state_end.borrow_mut();
            st.is_dragging = false;
            st.last_interaction = std::time::Instant::now();
            st.accumulator = 0.0;
        } else {
            mc_end.set_cursor(None::<&gtk::gdk::Cursor>);
        }
    });

    model_card.add_controller(drag);

    let scroll = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::HORIZONTAL,
    );
    let tt_scroll = std::rc::Rc::clone(&active_turntable);
    let frame_scroll = std::rc::Rc::clone(&current_frame);
    let pic_scroll = model_picture.clone();
    let state_scroll = std::rc::Rc::clone(&turntable_state);

    scroll.connect_scroll(move |_, dx, dy| {
        let tt_guard = tt_scroll.borrow();
        if let Some(ref tt) = *tt_guard {
            let total = tt.frame_count();
            if total > 0 {
                let delta = if dy.abs() > dx.abs() { dy } else { dx };
                let step = if delta > 0.0 {
                    1
                } else if delta < 0.0 {
                    -1
                } else {
                    0
                };
                if step != 0 {
                    let new_frame =
                        (frame_scroll.get() as isize + step).rem_euclid(total as isize) as usize;
                    frame_scroll.set(new_frame);
                    if let Some(tex) = tt.get_texture(new_frame) {
                        pic_scroll.set_paintable(Some(&tex));
                    }
                    let mut st = state_scroll.borrow_mut();
                    st.last_interaction = std::time::Instant::now();
                    st.accumulator = 0.0;
                    return gtk::glib::Propagation::Stop;
                }
            }
        }
        gtk::glib::Propagation::Proceed
    });
    model_card.add_controller(scroll);

    {
        let model_card_weak = model_card.downgrade();
        let model_stack = model_stack.clone();
        let model_picture = model_picture.clone();
        let active_turntable = std::rc::Rc::clone(&active_turntable);
        let current_frame = std::rc::Rc::clone(&current_frame);
        let turntable_state = std::rc::Rc::clone(&turntable_state);

        model_card.connect_map(move |_| {
            if let Some(card) = model_card_weak.upgrade() {
                state::ensure_turntable_tick(
                    &card,
                    &model_stack,
                    &model_picture,
                    &active_turntable,
                    &current_frame,
                    &turntable_state,
                );
            }
        });
    }

    {
        let turntable_state = std::rc::Rc::clone(&turntable_state);
        model_card.connect_unmap(move |_| {
            state::stop_turntable_tick(&turntable_state);
        });
    }

    let device_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("device-information"))
        .build();

    device_group.add(&preferences_content_row(&device_row));

    model_card.set_margin_top(8);
    model_card.set_margin_bottom(8);
    model_card.set_margin_start(12);
    model_card.set_margin_end(12);
    device_group.add(&preferences_content_row(&model_card));

    let mode_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("noise-mode-title"))
        .build();

    let primary_modes = GtkBox::new(Orientation::Horizontal, 8);
    primary_modes.set_homogeneous(true);
    primary_modes.set_hexpand(true);
    primary_modes.set_halign(Align::Fill);
    primary_modes.set_valign(Align::Start);
    primary_modes.set_margin_top(14);
    primary_modes.set_margin_bottom(14);
    primary_modes.set_margin_start(12);
    primary_modes.set_margin_end(12);
    let mut primary_noise_controls = Vec::new();
    for index in compatibility::catalog_noise_indices() {
        let item_box = GtkBox::new(Orientation::Vertical, 6);
        item_box.set_halign(Align::Center);
        item_box.set_valign(Align::Start);
        item_box.set_hexpand(true);
        item_box.set_vexpand(false);
        item_box.set_visible(false);

        let button = Button::builder()
            .css_classes(["circular", "noise-mode-button"])
            .valign(Align::Start)
            .halign(Align::Center)
            .sensitive(false)
            .build();
        let icon = gtk::Image::new();
        icon.set_pixel_size(22);
        button.set_child(Some(&icon));

        let label = Label::builder()
            .label(&crate::t!("noise-mode-numbered", index))
            .css_classes(["caption", "noise-mode-label"])
            .halign(Align::Center)
            .valign(Align::Start)
            .justify(gtk::Justification::Center)
            .xalign(0.5)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .max_width_chars(14)
            .width_chars(1)
            .sensitive(false)
            .build();

        let click_gesture = gtk::GestureClick::new();
        let btn_clone = button.clone();
        click_gesture.connect_released(move |_, _, _, _| {
            if btn_clone.is_sensitive() {
                btn_clone.emit_clicked();
            }
        });
        label.add_controller(click_gesture);

        item_box.append(&button);
        item_box.append(&label);

        primary_noise_controls.push(PrimaryNoiseControl {
            index,
            container: item_box.clone(),
            button,
            label,
        });
        primary_modes.append(&item_box);
    }

    mode_group.add(&preferences_content_row(&primary_modes));

    let noise_option_row = combo_row(&crate::t!("noise-mode-level"), None);
    noise_option_row.set_visible(false);
    mode_group.add(&noise_option_row);

    let sound_card_section = GtkBox::new(Orientation::Vertical, 0);
    sound_card_section.set_hexpand(true);
    sound_card_section.set_halign(Align::Fill);

    let sound_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("sound-quality-and-effects"))
        .build();

    let eq_row = combo_row(
        &crate::t!("equalizer-title"),
        Some(&crate::t!("equalizer-select-preset")),
    );

    let spatial_row = combo_row(
        &crate::t!("spatial-audio-title"),
        Some(&crate::t!("spatial-audio-effects-desc")),
    );

    let (game_mode_section, game_mode_switch) = switch_row(
        &crate::t!("game-mode-title"),
        Some(&crate::t!("game-mode-desc")),
    );

    let sound_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    sound_list.append(&eq_row);
    sound_list.append(&spatial_row);
    sound_list.append(&game_mode_section);

    sound_group.add(&sound_list);
    sound_card_section.append(&sound_group);

    let paired_devices_section = GtkBox::new(Orientation::Vertical, 0);
    paired_devices_section.set_hexpand(true);
    paired_devices_section.set_halign(Align::Fill);

    let paired_devices_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("device-management"))
        .build();

    let paired_devices = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let placeholder = Label::builder()
        .label(&crate::t!("dual-device-read-after-connecting"))
        .xalign(0.5)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(["dim-label"])
        .build();
    configure_label_wrapping(&placeholder);
    paired_devices.set_placeholder(Some(&placeholder));

    paired_devices_group.add(&paired_devices);
    paired_devices_section.append(&paired_devices_group);

    let columns = AdaptiveColumns::new();
    columns.left().append(&device_group);
    columns.right().append(&mode_group);
    columns.right().append(&sound_card_section);
    columns.right().append(&paired_devices_section);
    page.append(columns.widget());

    {
        let overlay = overlay.clone();
        let aspect_frame = aspect_frame.clone();
        let height_clamp = height_clamp.clone();
        let model_content = model_content.clone();

        let update_card_layout = move |is_single_column: bool| {
            if is_single_column {
                if overlay.parent().as_ref() == Some(aspect_frame.upcast_ref::<gtk::Widget>()) {
                    aspect_frame.set_child(None::<&gtk::Widget>);
                    model_content.remove(&height_clamp);
                    overlay.set_height_request(240);
                    model_content.prepend(&overlay);
                }
            } else if overlay.parent().as_ref() == Some(model_content.upcast_ref::<gtk::Widget>()) {
                model_content.remove(&overlay);
                overlay.set_height_request(-1);
                aspect_frame.set_child(Some(&overlay));
                model_content.prepend(&height_clamp);
            }
        };

        columns.container().connect_orientation_notify(move |hc| {
            let is_single = hc.orientation() == Orientation::Vertical;
            update_card_layout(is_single);
        });
    }

    (
        page,
        HomeWidgets {
            home_columns: columns.clone(),
            model_stack,
            model_picture,
            scene_view,
            model_placeholder,
            model_firmware,
            model_card,
            active_turntable,
            current_frame,
            turntable_state,
            device_name,
            connection_tag,
            device_menu_button,
            device_popover,
            device_list,
            device_list_empty,
            refresh_devices,
            left_battery,
            right_battery,
            case_battery,
            noise_option_row,
            paired_devices_section,
            paired_devices,
            primary_noise_controls,
            sound_card_section,
            eq_row,
            spatial_row,
            game_mode_section,
            game_mode_switch,
        },
    )
}
