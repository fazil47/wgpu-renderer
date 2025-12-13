use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// ECS-compatible camera controller for handling input and updating camera position
pub struct CameraController {
    pub amount_left: f32,
    pub amount_right: f32,
    pub amount_forward: f32,
    pub amount_backward: f32,
    pub amount_up: f32,
    pub amount_down: f32,
    pub rotate_horizontal: f32,
    pub rotate_vertical: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub cursor_locked: bool,
    pub fast_speed_multiplier: f32,
    pub fast_speed_requests: u32,
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
