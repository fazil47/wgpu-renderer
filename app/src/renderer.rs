use std::sync::Arc;
use winit::window::Window;

use crate::{
    camera::{Camera, CameraController},
    egui::RendererEgui,
    engine::EngineConfiguration,
    rasterizer::Rasterizer,
    raytracer::Raytracer,
    scene::Scene,
    wgpu::RendererWgpu,
};

pub struct Renderer {
    pub rasterizer: Rasterizer,
    pub raytracer: Raytracer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub egui: RendererEgui,
    pub wgpu: RendererWgpu,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        window_size: &winit::dpi::PhysicalSize<u32>,
        camera: &Camera,
        scene: &Scene,
    ) -> Self {
        let wgpu = RendererWgpu::new(window.clone(), window_size).await;
        let egui = RendererEgui::new(
            &window,
            &wgpu.device,
            &wgpu.surface_config,
            window.scale_factor() as f32,
        );

        // Initialize vertex and index buffers
        let vertex_buffer = scene.mesh.create_vertex_buffer(&wgpu.device);
        let index_buffer = scene.mesh.create_index_buffer(&wgpu.device);
        let num_indices = scene.mesh.get_index_count();

        let rasterizer = Rasterizer::new(&camera, &scene.sun_light.direction, &wgpu);
        let raytracer = Raytracer::new(
            &window_size,
            &wgpu,
            &vertex_buffer,
            &index_buffer,
            &scene.sun_light.direction,
            &camera,
            0,
        );

        Self {
            wgpu,
            egui,
            vertex_buffer,
            index_buffer,
            num_indices,
            rasterizer,
            raytracer,
        }
    }

    pub fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        camera_controller: &CameraController,
    ) {
        self.update_camera(camera_controller);

        // Reconfigure the surface with the new size
        self.wgpu.resize(&new_size);

        // Update the rasterizer with the new size
        self.rasterizer.resize(&self.wgpu);

        // Update the raytracer with the new size
        self.raytracer.resize(&new_size, &self.wgpu);
    }

    pub fn setup_egui(
        &mut self,
        window: &winit::window::Window,
        run_ui: impl FnMut(&egui::Context),
    ) -> egui::FullOutput {
        let egui_raw_input = self.egui.state.take_egui_input(window);
        self.egui.state.egui_ctx().run(egui_raw_input, run_ui)
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        window_size: &winit::dpi::PhysicalSize<u32>,
        config: &EngineConfiguration,
        camera_controller: &CameraController,
        frame_count: u32,
        egui_output: egui::FullOutput,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_camera(camera_controller);

        if config.is_raytracer_enabled && frame_count < config.raytracer_max_frames {
            self.raytracer
                .compute(&window_size, &self.wgpu.device, &self.wgpu.queue);
            self.raytracer
                .update_frame_count(&self.wgpu.queue, frame_count);
        }

        let surface_texture = self.wgpu.surface.get_current_texture()?;
        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder =
            self.wgpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Command Encoder"),
                });

        let egui_primitives = self
            .egui
            .state
            .egui_ctx()
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);
        let egui_screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                self.wgpu.surface_config.width,
                self.wgpu.surface_config.height,
            ],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in egui_output.textures_delta.set {
            self.egui.renderer.update_texture(
                &self.wgpu.device,
                &self.wgpu.queue,
                id,
                &image_delta,
            );
        }

        {
            if config.is_raytracer_enabled {
                self.raytracer
                    .render(&mut render_encoder, &surface_texture_view);
            } else {
                self.rasterizer.render(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.vertex_buffer,
                    &self.index_buffer,
                    self.num_indices,
                );
            };

            self.egui.render(
                &self.wgpu.device,
                &self.wgpu.queue,
                &mut render_encoder,
                &surface_texture_view,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        }

        self.wgpu.queue.submit(Some(render_encoder.finish()));
        surface_texture.present();

        for id in egui_output.textures_delta.free {
            self.egui.renderer.free_texture(&id);
        }

        Ok(())
    }

    pub fn update_camera(&self, camera_controller: &CameraController) {
        self.rasterizer
            .update_camera(&self.wgpu.queue, camera_controller);
        self.raytracer
            .update_camera(&self.wgpu.queue, camera_controller);
    }
}
