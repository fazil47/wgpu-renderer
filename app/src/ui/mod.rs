pub mod egui;
pub mod mesh_hierarchy;

pub use egui::*;
pub use mesh_hierarchy::*;

#[derive(Default)]
pub struct UiState {
    pub egui_output: Option<::egui::FullOutput>,
    pub fps: f32,
    pub frame_time_ms: f32,
}

impl ecs::Resource for UiState {}
