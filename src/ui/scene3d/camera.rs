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

use glam::{Mat4, Quat, Vec2, Vec3};

/// Camera states mirroring HeyMelody's `CameraController`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CameraState {
    Idle,
    Control,
    ControlOver,
    Rotate,
    Recover,
    ControlOverToRotate,
}

/// Orbit camera for HeyMelody 3D models.
/// Orbits model center at (0, 0, -4) with default camera position at (0, 0, 1).
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub current_state: CameraState,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub default_distance: f32,
    pub rotate_speed: f32, // Degrees per second when auto-rotating
    pub auto_rotate: bool,

    target_distance: f32,
    pending_slide: Vec2,
    current_slide: Vec2,
    target_slide: Vec2,
    no_input_time: f32,
    state_time: f32,
    speed: f32,
    recover_yaw: f32,
    recover_pitch: f32,
    recover_distance: f32,
}

pub const CENTER: Vec3 = Vec3::new(0.0, 0.0, -4.0);
pub const START_EYE: Vec3 = Vec3::new(0.0, 0.0, -0.6);
pub const VERTICAL_FOV_DEG: f32 = 45.0;
pub const NEAR: f32 = 0.5;
pub const FAR: f32 = 100.0;
pub const MAX_PITCH: f32 = 85.0;
pub const MIN_DISTANCE_FACTOR: f32 = 0.75;
pub const MAX_DISTANCE_FACTOR: f32 = 1.5;

const SLIDE_SPEED: f32 = 21.0;
const DAMP: f32 = 7.2;
const RECOVER_SECONDS: f32 = 1.0;
#[allow(dead_code)]
const RETURN_SECONDS: f32 = 10.0;
const CONTROL_TIMEOUT: f32 = 3.0;

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitCamera {
    pub fn new() -> Self {
        let default_dist = (START_EYE - CENTER).length();
        let mut cam = Self {
            current_state: CameraState::Rotate,
            yaw: 0.0,
            pitch: 0.0,
            distance: default_dist,
            default_distance: default_dist,
            rotate_speed: 20.0,
            auto_rotate: true,
            target_distance: default_dist,
            pending_slide: Vec2::ZERO,
            current_slide: Vec2::ZERO,
            target_slide: Vec2::ZERO,
            no_input_time: 0.0,
            state_time: 0.0,
            speed: 0.0,
            recover_yaw: 0.0,
            recover_pitch: 0.0,
            recover_distance: default_dist,
        };
        cam.switch_to_rotate();
        cam
    }

    pub fn is_moving(&self) -> bool {
        match self.current_state {
            CameraState::Rotate | CameraState::ControlOverToRotate => {
                self.auto_rotate || self.current_state == CameraState::ControlOverToRotate
            }
            CameraState::Idle => false,
            _ => true,
        }
    }

    pub fn min_distance(&self) -> f32 {
        self.default_distance * MIN_DISTANCE_FACTOR
    }

    pub fn max_distance(&self) -> f32 {
        self.default_distance * MAX_DISTANCE_FACTOR
    }

    pub fn start_drag(&mut self) {
        self.current_state = CameraState::Control;
        self.no_input_time = 0.0;
        self.pending_slide = Vec2::ZERO;
        self.current_slide = Vec2::ZERO;
        self.target_slide = Vec2::ZERO;
    }

    pub fn drag(&mut self, delta_x: f32, delta_y: f32) {
        self.current_state = CameraState::Control;
        self.no_input_time = 0.0;
        self.pending_slide += Vec2::new(delta_x, delta_y);
    }

    pub fn end_drag(&mut self) {
        if self.current_state == CameraState::Control {
            self.current_state = CameraState::ControlOver;
            self.state_time = 0.0;
        }
    }

    pub fn zoom(&mut self, factor: f32) {
        self.target_distance =
            (self.target_distance * factor).clamp(self.min_distance(), self.max_distance());
        self.no_input_time = 0.0;
    }

    pub fn reset(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.distance = self.default_distance;
        self.target_distance = self.default_distance;
        self.pending_slide = Vec2::ZERO;
        self.current_slide = Vec2::ZERO;
        self.target_slide = Vec2::ZERO;
        self.switch_to_rotate();
    }

    fn switch_to_rotate(&mut self) {
        self.current_state = CameraState::Rotate;
        self.state_time = 0.0;
        self.speed = if self.auto_rotate {
            self.rotate_speed
        } else {
            0.0
        };
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.001, 0.1);

        // Smooth zoom
        self.distance += (self.target_distance - self.distance) * (1.0 - (-8.0 * dt).exp());

        match self.current_state {
            CameraState::Control => {
                let slide = self.pending_slide;
                self.pending_slide = Vec2::ZERO;
                self.target_slide = slide * SLIDE_SPEED;
                self.current_slide +=
                    (self.target_slide - self.current_slide) * (DAMP * dt).min(1.0);

                self.yaw += self.current_slide.x * dt;
                self.pitch = (self.pitch + self.current_slide.y * dt).clamp(-MAX_PITCH, MAX_PITCH);
            }
            CameraState::ControlOver => {
                self.current_slide *= (1.0 - DAMP * dt).max(0.0);
                self.yaw += self.current_slide.x * dt;
                self.pitch = (self.pitch + self.current_slide.y * dt).clamp(-MAX_PITCH, MAX_PITCH);

                self.no_input_time += dt;
                if self.current_slide.length_squared() < 0.01
                    && self.no_input_time > CONTROL_TIMEOUT
                {
                    self.current_state = CameraState::Recover;
                    self.state_time = 0.0;
                    self.recover_yaw = self.yaw;
                    self.recover_pitch = self.pitch;
                    self.recover_distance = self.distance;
                }
            }
            CameraState::Recover => {
                self.state_time += dt;
                let t = (self.state_time / RECOVER_SECONDS).min(1.0);
                let smooth_t = t * t * (3.0 - 2.0 * t);

                self.pitch = self.recover_pitch * (1.0 - smooth_t);
                self.target_distance = self.recover_distance
                    + (self.default_distance - self.recover_distance) * smooth_t;

                if t >= 1.0 {
                    self.switch_to_rotate();
                }
            }
            CameraState::Rotate => {
                if self.auto_rotate {
                    self.yaw += self.speed * dt;
                }
            }
            CameraState::Idle | CameraState::ControlOverToRotate => {}
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        let rot = self.rotation();
        CENTER + rot * Vec3::new(0.0, 0.0, self.distance)
    }

    pub fn up_vector(&self) -> Vec3 {
        self.rotation() * Vec3::Y
    }

    fn rotation(&self) -> Quat {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();
        Quat::from_axis_angle(Vec3::Y, yaw_rad) * Quat::from_axis_angle(Vec3::X, pitch_rad)
    }

    pub fn view_matrix(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye_position(), CENTER, self.up_vector())
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        let safe_aspect = aspect.max(0.01);
        glam::camera::rh::proj::opengl::perspective(
            VERTICAL_FOV_DEG.to_radians(),
            safe_aspect,
            NEAR,
            FAR,
        )
    }

    pub fn compute_model_matrix(bounds_min: Vec3, bounds_max: Vec3, model_scale: f32) -> Mat4 {
        let center = (bounds_min + bounds_max) * 0.5;
        let half_extents = (bounds_max - bounds_min) * 0.5;
        let max_half = half_extents
            .x
            .max(half_extents.y)
            .max(half_extents.z)
            .max(0.001);
        let scale = (model_scale / max_half).max(0.001);

        Mat4::from_translation(CENTER)
            * Mat4::from_scale(Vec3::splat(scale))
            * Mat4::from_translation(-center)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_initializes_with_correct_eye_and_distance() {
        let cam = OrbitCamera::new();
        assert!((cam.distance - 3.4).abs() < 1e-4);
        assert!((cam.eye_position() - START_EYE).length() < 1e-4);
        assert_eq!(cam.pitch, 0.0);
        assert_eq!(cam.yaw, 0.0);
    }

    #[test]
    fn camera_drag_and_clamp_pitch() {
        let mut cam = OrbitCamera::new();
        cam.start_drag();
        cam.drag(0.0, 50.0);
        cam.tick(0.1);
        assert!(cam.pitch > 0.0 && cam.pitch <= MAX_PITCH);

        cam.drag(0.0, 1000.0);
        cam.tick(0.1);
        assert!(cam.pitch <= MAX_PITCH);
    }

    #[test]
    fn camera_zoom_clamps_to_range() {
        let mut cam = OrbitCamera::new();
        cam.zoom(0.1);
        assert!(cam.target_distance >= cam.min_distance());
        cam.zoom(10.0);
        assert!(cam.target_distance <= cam.max_distance());
    }

    #[test]
    fn compute_model_matrix_centers_at_target() {
        let min = Vec3::new(-2.0, -1.0, -3.0);
        let max = Vec3::new(2.0, 1.0, 3.0);
        let mat = OrbitCamera::compute_model_matrix(min, max, 1.0);
        let origin_transformed = mat.transform_point3(Vec3::new(0.0, 0.0, 0.0));
        assert!((origin_transformed - CENTER).length() < 1e-4);
    }
}
