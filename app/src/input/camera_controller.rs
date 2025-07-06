use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use maths::{Vec3, Quat};

use crate::ecs::scene::EcsScene;

/// ECS-compatible camera controller for handling input and updating camera position
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    speed: f32,
    sensitivity: f32,
    cursor_locked: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed,
            sensitivity: 0.002, // Much lower sensitivity for smoother camera movement
            cursor_locked: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
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
                let amount = if *state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                };
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.amount_forward = amount;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.amount_backward = amount;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.amount_left = amount;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.amount_right = amount;
                        true
                    }
                    KeyCode::KeyE => {
                        self.amount_up = amount;
                        true
                    }
                    KeyCode::KeyQ => {
                        self.amount_down = amount;
                        true
                    }
                    _ => false,
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Right {
                    self.cursor_locked = *state == ElementState::Pressed;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.cursor_locked {
            self.rotate_horizontal = mouse_dx as f32;
            self.rotate_vertical = mouse_dy as f32;
        }
    }

    pub fn update_camera(&mut self, scene: &mut EcsScene, dt: f32) {
        if let Some(camera_component) = scene.get_camera_component() {
            let mut camera = camera_component.borrow_mut();
            
            // Move the camera based on input
            let (forward, right, up) = {
                let forward = camera.forward.normalized();
                let right = forward.cross(Vec3::Y).normalized();
                let up = Vec3::Y; // Always use world up for movement
                (forward, right, up)
            };

            let mut velocity = Vec3::ZERO;
            velocity += forward * (self.amount_forward - self.amount_backward);
            velocity += right * (self.amount_right - self.amount_left);
            velocity += up * (self.amount_up - self.amount_down);

            if velocity.length() > 0.0 {
                camera.eye += velocity.normalized() * self.speed * dt;
            }

            // Rotate the camera based on mouse input
            if self.cursor_locked && (self.rotate_horizontal != 0.0 || self.rotate_vertical != 0.0) {
                // Horizontal rotation around world Y axis
                let yaw_rotation = Quat::from_rotation_y(-self.rotate_horizontal * self.sensitivity);
                
                // Vertical rotation around the camera's right axis
                let right_axis = camera.forward.cross(Vec3::Y).normalized();
                let pitch_rotation = Quat::from_axis_angle(right_axis, -self.rotate_vertical * self.sensitivity);
                
                // Apply rotations to forward vector
                camera.forward = yaw_rotation * camera.forward;
                let new_forward = pitch_rotation * camera.forward;
                
                // Prevent camera from flipping upside down (clamp pitch)
                if new_forward.dot(Vec3::Y).abs() < 0.95 {
                    camera.forward = new_forward.normalized();
                }
                
                // Update the camera's up vector to maintain orthogonality
                let right = camera.forward.cross(Vec3::Y).normalized();
                camera.up = right.cross(camera.forward).normalized();

                // Reset rotation deltas
                self.rotate_horizontal = 0.0;
                self.rotate_vertical = 0.0;
            }
        }
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.cursor_locked
    }
}