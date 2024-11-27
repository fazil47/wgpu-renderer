use winit::window::Window;

pub fn initialize_egui(
    window: &Window,
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    pixels_per_point: f32,
) -> (egui_wgpu::Renderer, egui_winit::State) {
    // TODO: Try with dithering enabled
    let egui_renderer = egui_wgpu::Renderer::new(device, surface_config.format, None, 1, false);
    let egui_ctx = egui::Context::default();

    let egui_viewport_id = egui_ctx.viewport_id();
    let egui_state = egui_winit::State::new(
        egui_ctx,
        egui_viewport_id,
        window,
        Some(pixels_per_point),
        None,
        None,
    );

    (egui_renderer, egui_state)
}

pub struct RendererEguiResources {
    pub renderer: egui_wgpu::Renderer,
    pub state: egui_winit::State,
}

pub fn render_egui(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_encoder: &mut wgpu::CommandEncoder,
    surface_texture_view: &wgpu::TextureView,
    renderer: &mut egui_wgpu::Renderer,
    egui_primitives: &Vec<egui::ClippedPrimitive>,
    egui_screen_descriptor: &egui_wgpu::ScreenDescriptor,
) {
    renderer.update_buffers(
        device,
        queue,
        render_encoder,
        egui_primitives,
        egui_screen_descriptor,
    );

    let egui_rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Rasterizer Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &surface_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    renderer.render(
        &mut egui_rpass.forget_lifetime(),
        &egui_primitives,
        &egui_screen_descriptor,
    );
}
