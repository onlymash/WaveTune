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

use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
};

use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Label, Orientation, Stack};

use crate::{
    resources::{turntable::TurntableModel, vfxms::VfxScene},
    ui::scene3d::DeviceSceneView,
};

#[derive(Debug)]
pub(crate) struct TurntableState {
    pub(crate) tick_id: Option<gtk::TickCallbackId>,
    pub(crate) is_dragging: bool,
    pub(crate) last_interaction: std::time::Instant,
    pub(crate) last_tick_time: Option<std::time::Instant>,
    pub(crate) accumulator: f64,
}

impl Default for TurntableState {
    fn default() -> Self {
        Self {
            tick_id: None,
            is_dragging: false,
            last_interaction: std::time::Instant::now(),
            last_tick_time: None,
            accumulator: 0.0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct DeviceModelHeroWidgets {
    model_card: GtkBox,
    model_content: GtkBox,
    aspect_frame: gtk::AspectFrame,
    height_clamp: adw::Clamp,
    overlay: gtk::Overlay,
    model_stack: Stack,
    model_picture: gtk::Picture,
    scene_view: DeviceSceneView,
    model_placeholder: gtk::Image,
    model_firmware: Label,
    active_turntable: Rc<RefCell<Option<Rc<TurntableModel>>>>,
    current_frame: Rc<Cell<usize>>,
    turntable_state: Rc<RefCell<TurntableState>>,
}

impl DeviceModelHeroWidgets {
    pub fn set_firmware(&self, text: &str) {
        self.model_firmware.set_text(text);
        self.model_firmware.set_visible(true);
    }

    pub fn hide_firmware(&self) {
        self.model_firmware.set_visible(false);
        self.model_firmware.set_text("");
    }

    pub fn set_placeholder_visible(&self, visible: bool) {
        self.model_placeholder.set_visible(visible);
    }

    pub fn show_picture(&self) {
        self.model_stack.set_visible_child_name("picture");
    }

    pub fn show_scene3d(&self) {
        self.model_stack.set_visible_child_name("scene3d");
    }

    pub fn set_picture_paintable(&self, paintable: Option<&gtk::gdk::Paintable>) {
        self.model_picture.set_paintable(paintable);
    }

    pub fn set_picture_filename(&self, path: Option<&Path>) {
        self.model_picture.set_filename(path);
    }

    pub fn has_turntable(&self) -> bool {
        self.active_turntable.borrow().is_some()
    }

    pub fn has_scene(&self) -> bool {
        self.scene_view.has_scene()
    }

    pub fn clear_turntable(&self) {
        *self.active_turntable.borrow_mut() = None;
        self.stop_turntable_tick();
        self.model_card.set_cursor(None::<&gtk::gdk::Cursor>);
    }

    pub fn set_turntable(&self, turntable: Rc<TurntableModel>) {
        if let Some(tex) = turntable.get_texture(0) {
            self.model_picture.set_paintable(Some(&tex));
            *self.active_turntable.borrow_mut() = Some(turntable);
            self.current_frame.set(0);
            self.model_card.set_cursor_from_name(Some("grab"));
            self.show_picture();
            self.ensure_turntable_tick();
        }
    }

    pub fn clear_scene(&self) {
        self.scene_view.clear();
    }

    pub fn load_scene(&self, scene: &VfxScene) -> Result<(), String> {
        self.scene_view.load_scene(scene)
    }

    pub fn update_card_layout(&self, is_single_column: bool) {
        if is_single_column {
            if self.overlay.parent().as_ref() == Some(self.aspect_frame.upcast_ref::<gtk::Widget>())
            {
                self.aspect_frame.set_child(None::<&gtk::Widget>);
                self.model_content.remove(&self.height_clamp);
                self.overlay.set_height_request(240);
                self.model_content.prepend(&self.overlay);
            }
        } else if self.overlay.parent().as_ref()
            == Some(self.model_content.upcast_ref::<gtk::Widget>())
        {
            self.model_content.remove(&self.overlay);
            self.overlay.set_height_request(-1);
            self.aspect_frame.set_child(Some(&self.overlay));
            self.model_content.prepend(&self.height_clamp);
        }
    }

    pub fn ensure_turntable_tick(&self) {
        let mut st = self.turntable_state.borrow_mut();
        if st.tick_id.is_none() && self.active_turntable.borrow().is_some() {
            st.last_tick_time = Some(std::time::Instant::now());
            st.accumulator = 0.0;
            let active_turntable = Rc::clone(&self.active_turntable);
            let current_frame = Rc::clone(&self.current_frame);
            let turntable_state = Rc::clone(&self.turntable_state);
            let model_picture = self.model_picture.clone();
            let model_stack = self.model_stack.clone();

            let id = self.model_card.add_tick_callback(move |widget, _| {
                if !widget.is_mapped()
                    || model_stack.visible_child_name().as_deref() != Some("picture")
                {
                    turntable_state.borrow_mut().tick_id = None;
                    return gtk::glib::ControlFlow::Break;
                }

                let tt_guard = active_turntable.borrow();
                let Some(ref tt) = *tt_guard else {
                    turntable_state.borrow_mut().tick_id = None;
                    return gtk::glib::ControlFlow::Break;
                };

                let total = tt.frame_count();
                if total == 0 {
                    turntable_state.borrow_mut().tick_id = None;
                    return gtk::glib::ControlFlow::Break;
                }

                let now = std::time::Instant::now();
                let mut st = turntable_state.borrow_mut();
                let dt = st
                    .last_tick_time
                    .map_or(0.016, |prev| now.duration_since(prev).as_secs_f64())
                    .min(0.2);
                st.last_tick_time = Some(now);

                if st.is_dragging {
                    st.accumulator = 0.0;
                    return gtk::glib::ControlFlow::Continue;
                }

                if now.duration_since(st.last_interaction).as_millis() < 1200 {
                    st.accumulator = 0.0;
                    return gtk::glib::ControlFlow::Continue;
                }

                let duration_ms = tt.frame_duration_ms(current_frame.get());
                let frame_duration = (duration_ms as f64 / 1000.0).max(0.025);
                st.accumulator += dt;
                if st.accumulator >= frame_duration {
                    let steps = (st.accumulator / frame_duration).floor() as usize;
                    st.accumulator %= frame_duration;
                    // Reverse rotation direction to match natural turntable spin
                    let next = (current_frame.get() as isize - steps as isize)
                        .rem_euclid(total as isize) as usize;
                    current_frame.set(next);
                    if let Some(tex) = tt.get_texture(next) {
                        model_picture.set_paintable(Some(&tex));
                    }
                    // Pre-warm upcoming frame in cache to eliminate decode lag
                    let upcoming = (next as isize - 1).rem_euclid(total as isize) as usize;
                    let _ = tt.get_texture(upcoming);
                }

                gtk::glib::ControlFlow::Continue
            });

            st.tick_id = Some(id);
        }
    }

    pub fn stop_turntable_tick(&self) {
        let mut st = self.turntable_state.borrow_mut();
        if let Some(id) = st.tick_id.take() {
            id.remove();
        }
        st.last_tick_time = None;
        st.accumulator = 0.0;
    }

    pub fn reset(&self) {
        self.clear_turntable();
        self.clear_scene();
        self.set_picture_paintable(None);
        self.show_picture();
        self.set_placeholder_visible(true);
        self.hide_firmware();
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl DeviceModelHeroWidgets {
    pub fn model_card(&self) -> &GtkBox {
        &self.model_card
    }

    pub fn model_firmware(&self) -> &Label {
        &self.model_firmware
    }
}

pub(crate) fn build_model_hero() -> (GtkBox, DeviceModelHeroWidgets) {
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

    let active_turntable: Rc<RefCell<Option<Rc<TurntableModel>>>> = Rc::new(RefCell::new(None));
    let current_frame = Rc::new(Cell::new(0usize));
    let drag_start_frame = Rc::new(Cell::new(0usize));
    let turntable_state = Rc::new(RefCell::new(TurntableState::default()));

    let drag = gtk::GestureDrag::new();
    let tt_drag = Rc::clone(&active_turntable);
    let frame_drag = Rc::clone(&current_frame);
    let start_frame = Rc::clone(&drag_start_frame);
    let mc_drag = model_card.clone();
    let state_drag = Rc::clone(&turntable_state);

    drag.connect_drag_begin(move |_, _, _| {
        if tt_drag.borrow().is_some() {
            start_frame.set(frame_drag.get());
            mc_drag.set_cursor_from_name(Some("grabbing"));
            let mut st = state_drag.borrow_mut();
            st.is_dragging = true;
            st.accumulator = 0.0;
        }
    });

    let tt_update = Rc::clone(&active_turntable);
    let frame_update = Rc::clone(&current_frame);
    let start_update = Rc::clone(&drag_start_frame);
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

    let tt_end = Rc::clone(&active_turntable);
    let mc_end = model_card.clone();
    let state_end = Rc::clone(&turntable_state);

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
    let tt_scroll = Rc::clone(&active_turntable);
    let frame_scroll = Rc::clone(&current_frame);
    let pic_scroll = model_picture.clone();
    let state_scroll = Rc::clone(&turntable_state);

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

    let widgets = DeviceModelHeroWidgets {
        model_card: model_card.clone(),
        model_content,
        aspect_frame,
        height_clamp,
        overlay,
        model_stack,
        model_picture,
        scene_view,
        model_placeholder,
        model_firmware,
        active_turntable,
        current_frame,
        turntable_state,
    };

    {
        let hero = widgets.clone();
        let model_card_weak = model_card.downgrade();

        model_card.connect_map(move |_| {
            if model_card_weak.upgrade().is_some() {
                hero.ensure_turntable_tick();
            }
        });
    }

    {
        let hero = widgets.clone();
        model_card.connect_unmap(move |_| {
            hero.stop_turntable_tick();
        });
    }

    (model_card, widgets)
}
