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

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use adw::prelude::*;
use gtk::glib;

use super::camera::OrbitCamera;
use super::model::GltfScene;
use super::renderer::SceneRenderer;
use crate::resources::vfxms::{VfxScene, VfxSceneConfig};

struct SceneViewState {
    camera: OrbitCamera,
    renderer: Option<SceneRenderer>,
    model_data: Option<(GltfScene, VfxSceneConfig, Vec<u8>)>,
    viewport_size: (i32, i32),
    last_frame_time: Option<Instant>,
    tick_id: Option<gtk::TickCallbackId>,
    is_dragging: bool,
    last_drag_offset: (f64, f64),
}

#[derive(Clone)]
pub struct DeviceSceneView {
    gl_area: gtk::GLArea,
    state: Rc<RefCell<SceneViewState>>,
}

impl Default for DeviceSceneView {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSceneView {
    pub fn new() -> Self {
        let gl_area = gtk::GLArea::new();
        gl_area.set_has_depth_buffer(true);
        gl_area.set_has_stencil_buffer(false);
        gl_area.set_auto_render(true);
        gl_area.set_hexpand(true);
        gl_area.set_vexpand(true);

        let state = Rc::new(RefCell::new(SceneViewState {
            camera: OrbitCamera::new(),
            renderer: None,
            model_data: None,
            viewport_size: (0, 0),
            last_frame_time: None,
            tick_id: None,
            is_dragging: false,
            last_drag_offset: (0.0, 0.0),
        }));

        // Realize handler
        {
            let state = Rc::clone(&state);
            gl_area.connect_realize(move |area| {
                area.make_current();
                let mut st = state.borrow_mut();
                if st.renderer.is_none() {
                    if let Some((scene, config, ibl_bytes)) = &st.model_data {
                        match SceneRenderer::new(scene.clone(), config.clone(), ibl_bytes) {
                            Ok(renderer) => {
                                st.renderer = Some(renderer);
                                area.set_cursor_from_name(Some("grab"));
                            }
                            Err(e) => {
                                eprintln!("Failed to initialize 3D scene renderer: {e}");
                            }
                        }
                    }
                }
            });
        }

        // Unrealize handler - release GL resources when GL context is detached
        {
            let state = Rc::clone(&state);
            gl_area.connect_unrealize(move |area| {
                area.make_current();
                let mut st = state.borrow_mut();
                if let Some(renderer) = st.renderer.take() {
                    drop(renderer);
                }
                if let Some(id) = st.tick_id.take() {
                    id.remove();
                }
            });
        }

        // Resize handler - track accurate framebuffer dimensions
        {
            let state = Rc::clone(&state);
            gl_area.connect_resize(move |area, width, height| {
                let mut st = state.borrow_mut();
                st.viewport_size = (width, height);
                drop(st);
                area.queue_render();
            });
        }

        // Render handler
        {
            let state = Rc::clone(&state);
            gl_area.connect_render(move |area, _| {
                if let Some(err) = area.error() {
                    eprintln!("GLArea error before render: {err}");
                    return glib::Propagation::Stop;
                }

                let mut st = state.borrow_mut();
                if st.renderer.is_none() {
                    if let Some((scene, config, ibl_bytes)) = &st.model_data {
                        area.make_current();
                        match SceneRenderer::new(scene.clone(), config.clone(), ibl_bytes) {
                            Ok(renderer) => {
                                st.renderer = Some(renderer);
                                area.set_cursor_from_name(Some("grab"));
                            }
                            Err(e) => {
                                eprintln!("Failed to initialize 3D scene renderer: {e}");
                            }
                        }
                    }
                }

                if let Some(ref renderer) = st.renderer {
                    let scale = area.scale_factor();
                    let (mut width, mut height) = st.viewport_size;
                    if width <= 0 || height <= 0 {
                        width = area.allocated_width() * scale;
                        height = area.allocated_height() * scale;
                    }
                    if width > 0 && height > 0 {
                        renderer.render(&st.camera, width, height);
                    }
                }

                glib::Propagation::Stop
            });
        }

        // Drag gesture
        let drag = gtk::GestureDrag::new();
        {
            let state = Rc::clone(&state);
            let gl_area_weak = gl_area.downgrade();
            drag.connect_drag_begin(move |_, _, _| {
                if let Some(area) = gl_area_weak.upgrade() {
                    let mut st = state.borrow_mut();
                    if st.renderer.is_some() {
                        st.camera.start_drag();
                        st.is_dragging = true;
                        st.last_drag_offset = (0.0, 0.0);
                        area.set_cursor_from_name(Some("grabbing"));
                        drop(st);
                        Self::ensure_tick(&area, &state);
                    }
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let gl_area_weak = gl_area.downgrade();
            drag.connect_drag_update(move |_, offset_x, offset_y| {
                if let Some(area) = gl_area_weak.upgrade() {
                    let mut st = state.borrow_mut();
                    if st.is_dragging {
                        let dx = (offset_x - st.last_drag_offset.0) as f32;
                        let dy = (offset_y - st.last_drag_offset.1) as f32;
                        st.last_drag_offset = (offset_x, offset_y);
                        st.camera.drag(dx, dy);
                        drop(st);
                        area.queue_render();
                    }
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let gl_area_weak = gl_area.downgrade();
            drag.connect_drag_end(move |_, _, _| {
                if let Some(area) = gl_area_weak.upgrade() {
                    let mut st = state.borrow_mut();
                    if st.is_dragging {
                        st.camera.end_drag();
                        st.is_dragging = false;
                        area.set_cursor_from_name(Some("grab"));
                        drop(st);
                        Self::ensure_tick(&area, &state);
                    }
                }
            });
        }
        gl_area.add_controller(drag);

        // Scroll gesture (zoom)
        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::HORIZONTAL,
        );
        {
            let state = Rc::clone(&state);
            let gl_area_weak = gl_area.downgrade();
            scroll.connect_scroll(move |_, dx, dy| {
                if let Some(area) = gl_area_weak.upgrade() {
                    let mut st = state.borrow_mut();
                    if st.renderer.is_some() {
                        let delta = if dy.abs() > dx.abs() { dy } else { dx };
                        let factor = if delta > 0.0 {
                            1.08
                        } else if delta < 0.0 {
                            0.92
                        } else {
                            1.0
                        };
                        st.camera.zoom(factor);
                        drop(st);
                        Self::ensure_tick(&area, &state);
                        area.queue_render();
                        return glib::Propagation::Stop;
                    }
                }
                glib::Propagation::Proceed
            });
        }
        gl_area.add_controller(scroll);

        // Click gesture (double click reset)
        let click = gtk::GestureClick::new();
        click.set_button(1);
        {
            let state = Rc::clone(&state);
            let gl_area_weak = gl_area.downgrade();
            click.connect_pressed(move |_, n_press, _, _| {
                if n_press == 2 {
                    if let Some(area) = gl_area_weak.upgrade() {
                        let mut st = state.borrow_mut();
                        if st.renderer.is_some() {
                            st.camera.reset();
                            drop(st);
                            Self::ensure_tick(&area, &state);
                            area.queue_render();
                        }
                    }
                }
            });
        }
        gl_area.add_controller(click);

        // Pause tick when unmapped (e.g., hidden page) and resume when mapped
        {
            let state = Rc::clone(&state);
            gl_area.connect_map(move |area| {
                let st = state.borrow();
                if st.renderer.is_some() && (st.camera.is_moving() || st.is_dragging) {
                    drop(st);
                    Self::ensure_tick(area, &state);
                    area.queue_render();
                }
            });
        }
        {
            let state = Rc::clone(&state);
            gl_area.connect_unmap(move |_| {
                let mut st = state.borrow_mut();
                if let Some(id) = st.tick_id.take() {
                    id.remove();
                }
                st.last_frame_time = None;
            });
        }

        Self { gl_area, state }
    }

    pub fn widget(&self) -> &gtk::GLArea {
        &self.gl_area
    }

    pub fn has_scene(&self) -> bool {
        let st = self.state.borrow();
        st.renderer.is_some() || st.model_data.is_some()
    }

    pub fn load_scene(&self, scene: &VfxScene) -> Result<(), String> {
        let gltf_scene = GltfScene::from_glb_bytes(&scene.glb_bytes)?;
        let config = scene.config.clone();
        let ibl_bytes = scene.ibl_bytes.clone();

        let mut st = self.state.borrow_mut();
        st.camera.reset();
        st.model_data = Some((gltf_scene.clone(), config.clone(), ibl_bytes.clone()));

        if self.gl_area.is_realized() {
            self.gl_area.make_current();
            let renderer = SceneRenderer::new(gltf_scene, config, &ibl_bytes)?;
            st.renderer = Some(renderer);
            self.gl_area.set_cursor_from_name(Some("grab"));
            drop(st);
            Self::ensure_tick(&self.gl_area, &self.state);
            self.gl_area.queue_render();
        } else {
            st.renderer = None;
        }

        Ok(())
    }

    pub fn clear(&self) {
        let mut st = self.state.borrow_mut();
        st.model_data = None;
        if let Some(id) = st.tick_id.take() {
            id.remove();
        }
        st.last_frame_time = None;
        if st.renderer.is_some() {
            if self.gl_area.is_realized() {
                self.gl_area.make_current();
            }
            st.renderer = None;
        }
        st.camera.reset();
        self.gl_area.set_cursor(None::<&gtk::gdk::Cursor>);
        drop(st);
        self.gl_area.queue_render();
    }

    #[allow(dead_code)]
    pub fn set_auto_rotate(&self, auto_rotate: bool) {
        let mut st = self.state.borrow_mut();
        st.camera.auto_rotate = auto_rotate;
        drop(st);
        Self::ensure_tick(&self.gl_area, &self.state);
    }

    fn ensure_tick(gl_area: &gtk::GLArea, state: &Rc<RefCell<SceneViewState>>) {
        let mut st = state.borrow_mut();
        if st.tick_id.is_none() && (st.renderer.is_some() || st.model_data.is_some()) {
            st.last_frame_time = Some(Instant::now());
            let state_clone = Rc::clone(state);
            let area_weak = gl_area.downgrade();
            let id = gl_area.add_tick_callback(move |_, _| {
                let Some(area) = area_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };

                let mut s = state_clone.borrow_mut();
                let now = Instant::now();
                let dt = s
                    .last_frame_time
                    .map_or(0.016, |prev| now.duration_since(prev).as_secs_f32());
                s.last_frame_time = Some(now);

                s.camera.tick(dt);
                area.queue_render();

                if !s.is_dragging && !s.camera.is_moving() {
                    s.tick_id = None;
                    s.last_frame_time = None;
                    return glib::ControlFlow::Break;
                }

                glib::ControlFlow::Continue
            });
            st.tick_id = Some(id);
        }
    }
}
