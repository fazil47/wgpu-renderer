use winit::window::Window;

pub fn initialize_egui(
    window: &Window,
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    pixels_per_point: f32,
) -> (egui_wgpu::Renderer, egui_winit::State) {
    let egui_renderer = egui_wgpu::Renderer::new(device, surface_config.format, None, 1);
    let egui_ctx = egui::Context::default();

    let egui_viewport_id = egui_ctx.viewport_id();
    let egui_state = egui_winit::State::new(
        egui_ctx,
        egui_viewport_id,
        window,
        Some(pixels_per_point),
        None,
    );

    (egui_renderer, egui_state)
}

pub struct RendererEguiResources {
    pub renderer: egui_wgpu::Renderer,
    pub state: egui_winit::State,
}
