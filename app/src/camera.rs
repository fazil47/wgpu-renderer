use egui::Vec2;
use maths::{Mat4, Quat, Vec3, Vec4};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::scene::Scene;

pub struct Camera {
    eye: Vec3,
    // The normalized forward vector of the camera is the direction the camera is looking at.
    forward: Vec3,
    // The normalized up vector of the camera is the direction that is considered up for the camera.
    up: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    world_to_camera: Mat4,
    camera_to_world: Mat4,
    camera_projection: Mat4,
    camera_inverse_projection: Mat4,
    view_projection: Mat4,
}

impl Camera {
    const GLOBAL_UP: Vec3 = Vec3::Y;

    pub fn new(eye: Vec3, forward: Vec3, aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        // The camera's up vector stays close to the global up
        let up = forward.cross(Camera::GLOBAL_UP.cross(forward)).normalize();

        let (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        ) = Self::calculate_matrices(eye, forward, up, aspect, fovy, znear, zfar);

        Self {
            eye,
            forward,
            up,
            aspect,
            fovy,
            znear,
            zfar,
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_matrices();
    }

    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.update_matrices();
    }

    pub fn set_znear(&mut self, znear: f32) {
        self.znear = znear;
        self.update_matrices();
    }

    pub fn set_zfar(&mut self, zfar: f32) {
        self.zfar = zfar;
        self.update_matrices();
    }

    pub fn set_eye(&mut self, eye: Vec3) {
        self.eye = eye;
        self.update_matrices();
    }

    pub fn set_forward(&mut self, forward: Vec3) {
        self.forward = forward;
        self.update_matrices();
    }

    pub fn set_up(&mut self, up: Vec3) {
        self.up = up;
        self.update_matrices();
    }

    pub fn eye(&self) -> Vec3 {
        self.eye
    }

    pub fn forward(&self) -> Vec3 {
        self.forward
    }

    pub fn up(&self) -> Vec3 {
        self.up
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn fovy(&self) -> f32 {
        self.fovy
    }

    pub fn znear(&self) -> f32 {
        self.znear
    }

    pub fn zfar(&self) -> f32 {
        self.zfar
    }

    pub fn world_to_camera(&self) -> Mat4 {
        self.world_to_camera
    }

    pub fn camera_to_world(&self) -> Mat4 {
        self.camera_to_world
    }

    pub fn camera_projection(&self) -> Mat4 {
        self.camera_projection
    }

    pub fn camera_inverse_projection(&self) -> Mat4 {
        self.camera_inverse_projection
    }

    /// Returns the view projection matrix of the camera.
    /// The view projection matrix is the product of the camera projection matrix and the world to camera matrix,
    /// and it's used to transform the vertices of the objects from world space to clip space.
    ///
    /// # Returns
    ///
    /// * `Mat4` - The view projection matrix of the camera.
    /// ```
    pub fn view_projection(&self) -> Mat4 {
        self.view_projection
    }

    fn update_matrices(&mut self) {
        let (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        ) = Self::calculate_matrices(
            self.eye,
            self.forward,
            self.up,
            self.aspect,
            self.fovy,
            self.znear,
            self.zfar,
        );

        self.world_to_camera = world_to_camera;
        self.camera_to_world = camera_to_world;
        self.camera_projection = camera_projection;
        self.camera_inverse_projection = camera_inverse_projection;
        self.view_projection = view_projection;
    }

    // `forward` is the normalized forward vector of the camera.
    // `up` is the normalized up vector of the camera.
    fn calculate_matrices(
        eye: Vec3,
        forward: Vec3,
        up: Vec3,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> (Mat4, Mat4, Mat4, Mat4, Mat4) {
        let right = forward.cross(up);

        let world_to_camera = Mat4::from_cols(
            Vec4::new(right.x, up.x, -forward.x, 0.0),
            Vec4::new(right.y, up.y, -forward.y, 0.0),
            Vec4::new(right.z, up.z, -forward.z, 0.0),
            Vec4::new(-right.dot(eye), -up.dot(eye), forward.dot(eye), 1.0),
        );
        let camera_to_world = world_to_camera.inverse();

        let top = znear * (fovy / 2.0).tan();
        let right = top * aspect;

        let camera_projection = Mat4::from_cols(
            Vec4::new(znear / right, 0.0, 0.0, 0.0),
            Vec4::new(0.0, znear / top, 0.0, 0.0),
            Vec4::new(0.0, 0.0, -(zfar + znear) / (zfar - znear), -1.0),
            Vec4::new(0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0),
        );
        let camera_inverse_projection = camera_projection.inverse();

        let view_projection = camera_projection * world_to_camera;

        (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        )
    }
}

// Ref: https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms
pub struct CameraController {
    speed: f32,
    is_shift_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_cursor_locked: bool,
    cursor_position: Vec2,
    cursor_delta: Vec2,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_shift_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_cursor_locked: false,
            cursor_position: Vec2::ZERO,
            cursor_delta: Vec2::ZERO,
            sensitivity: 0.003,
        }
    }

    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.is_cursor_locked
    }

    pub fn process_events(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyE => {
                        self.is_up_pressed = is_pressed;
                    }
                    KeyCode::KeyQ => {
                        self.is_down_pressed = is_pressed;
                    }
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                    }
                    KeyCode::ShiftLeft => {
                        self.is_shift_pressed = is_pressed;
                    }
                    _ => (),
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Right {
                    self.is_cursor_locked = state.is_pressed();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let new_position = Vec2::new(position.x as f32, position.y as f32);
                self.cursor_delta = new_position - self.cursor_position;
                self.cursor_position = new_position;
            }

            _ => (),
        }
    }

    pub fn update_camera(&mut self, scene: &mut Scene, delta_time: f32) {
        if !self.is_cursor_locked {
            return;
        }

        let camera = &mut scene.camera;
        let rotation_delta = self.cursor_delta * self.sensitivity;

        // Rotate around the global Y-axis (yaw)
        let yaw_rotation = Quat::from_rotation_y(-rotation_delta.x);
        camera.forward = yaw_rotation * camera.forward;
        camera.up = yaw_rotation * camera.up;

        // Rotate around the camera's local X-axis (pitch)
        let right = camera.forward.cross(camera.up).normalize();
        let pitch_rotation = Quat::from_axis_angle(right, -rotation_delta.y);
        camera.forward = pitch_rotation * camera.forward;
        camera.up = pitch_rotation * camera.up;

        // Ensure the camera's up vector stays close to the global up
        camera.up = camera
            .forward
            .cross(Camera::GLOBAL_UP.cross(camera.forward))
            .normalize();

        // Reset cursor delta
        self.cursor_delta = Vec2::ZERO;

        let mut translation_delta = Vec3::ZERO;
        let right = camera.forward.cross(camera.up).normalize();

        if self.is_forward_pressed {
            translation_delta += camera.forward;
        }
        if self.is_backward_pressed {
            translation_delta -= camera.forward;
        }
        if self.is_right_pressed {
            translation_delta += right;
        }
        if self.is_left_pressed {
            translation_delta -= right;
        }
        if self.is_up_pressed {
            translation_delta += camera.up;
        }
        if self.is_down_pressed {
            translation_delta -= camera.up;
        }

        if translation_delta != Vec3::ZERO {
            translation_delta = translation_delta.normalize() * self.speed * delta_time;

            if self.is_shift_pressed {
                translation_delta *= 2.0;
            }

            camera.eye += translation_delta;
        }

        camera.update_matrices();
    }
}
