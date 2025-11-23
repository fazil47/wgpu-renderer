use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use maths::{Quat, Vec3};

use crate::{camera::Camera, time::Time};
use ecs::World;

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
    fast_speed_multiplier: f32,
    fast_speed_requests: u32,
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
            fast_speed_multiplier: 10.0,
            fast_speed_requests: 0,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        repeat,
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
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                        if !repeat {
                            if *state == ElementState::Pressed {
                                self.fast_speed_requests += 1;
                            } else if self.fast_speed_requests > 0 {
                                self.fast_speed_requests -= 1;
                            }
                        }
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

    pub fn has_camera_moved(&self) -> bool {
        self.amount_forward != 0.0
            || self.amount_backward != 0.0
            || self.amount_left != 0.0
            || self.amount_right != 0.0
            || self.amount_up != 0.0
            || self.amount_down != 0.0
            || self.rotate_horizontal != 0.0
            || self.rotate_vertical != 0.0
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.cursor_locked
    }
}

impl ecs::Resource for CameraController {}

pub fn camera_controller_system(world: &mut World) {
    let dt = world.get_resource::<Time>().unwrap().delta_time;

    let mut controller = world.get_resource_mut::<CameraController>().unwrap();

    let camera_entity = world
        .get_entities_with::<Camera>()
        .first()
        .copied()
        .expect("Expected camera entity");

    if let Some(mut camera) = world.get_component_mut::<Camera>(camera_entity) {
        // Move the camera based on input
        let (forward, right, up) = {
            let forward = camera.forward.normalized();
            let right = forward.cross(Vec3::Y).normalized();
            let up = Vec3::Y; // Always use world up for movement
            (forward, right, up)
        };

        let mut velocity = Vec3::ZERO;
        velocity += forward * (controller.amount_forward - controller.amount_backward);
        velocity += right * (controller.amount_right - controller.amount_left);
        velocity += up * (controller.amount_up - controller.amount_down);

        if velocity.length() > 0.0 {
            let speed_multiplier = if controller.fast_speed_requests > 0 {
                controller.fast_speed_multiplier
            } else {
                1.0
            };
            camera.eye += velocity.normalized() * controller.speed * speed_multiplier * dt;
        }

        // Rotate the camera based on mouse input
        if controller.cursor_locked
            && (controller.rotate_horizontal != 0.0 || controller.rotate_vertical != 0.0)
        {
            // Horizontal rotation around world Y axis
            let yaw_rotation =
                Quat::from_rotation_y(-controller.rotate_horizontal * controller.sensitivity);

            // Vertical rotation around the camera's right axis
            let right_axis = camera.forward.cross(Vec3::Y).normalized();
            let pitch_rotation = Quat::from_axis_angle(
                right_axis,
                -controller.rotate_vertical * controller.sensitivity,
            );

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
            controller.rotate_horizontal = 0.0;
            controller.rotate_vertical = 0.0;
        }
    }
}
