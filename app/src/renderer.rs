use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::window::Window;

use crate::{
    egui::RendererEgui, engine::EngineConfiguration, rasterizer::Rasterizer, raytracer::Raytracer,
    scene::Scene, wgpu::RendererWgpu,
};

pub struct Renderer {
    pub rasterizer: Rc<RefCell<Rasterizer>>,
    pub raytracer: Raytracer,
    pub egui: RendererEgui,
    pub wgpu: RendererWgpu,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        window_size: &winit::dpi::PhysicalSize<u32>,
        scene: &mut Scene,
    ) -> Self {
        let wgpu = RendererWgpu::new(window.clone(), window_size).await;
        let egui = RendererEgui::new(
            &window,
            &wgpu.device,
            &wgpu.surface_config,
            window.scale_factor() as f32,
        );

        let rasterizer = Rc::new(RefCell::new(Rasterizer::new(&wgpu, scene)));
        let raytracer = Raytracer::new(&wgpu, window_size, scene);

        Self {
            wgpu,
            rasterizer,
            raytracer,
            egui,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scene: &Scene) {
        self.update_camera(scene);

        // Reconfigure the surface with the new size
        self.wgpu.resize(&new_size);

        // Update the rasterizer with the new size
        self.rasterizer.borrow_mut().resize(&self.wgpu);

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
        scene: &mut Scene,
        frame_count: u32,
        egui_output: egui::FullOutput,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_camera(scene);

        if scene.is_light_dirty() {
            self.update_light(scene);
        }

        // Check if probe grid configuration changed
        if self.rasterizer.borrow().is_probe_dirty() {
            self.rasterizer
                .borrow_mut()
                .update_probes(&self.wgpu.device, &self.wgpu.queue);
            self.rasterizer.borrow_mut().clear_probe_dirty();
        }

        if config.is_raytracer_enabled && frame_count < config.raytracer_max_frames {
            self.raytracer
                .compute(window_size, &self.wgpu.device, &self.wgpu.queue);
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
                self.rasterizer.borrow().render(
                    &self.wgpu.device,
                    &mut render_encoder,
                    &surface_texture_view,
                    &scene.materials,
                );

                // Render probe visualization if enabled
                let rasterizer = self.rasterizer.borrow();
                if rasterizer.should_render_probe_visualization() {
                    rasterizer
                        .render_probe_visualization(&mut render_encoder, &surface_texture_view);
                }
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

    pub fn update_camera(&self, scene: &Scene) {
        self.rasterizer
            .borrow()
            .update_camera(&self.wgpu.queue, scene);
        self.raytracer.update_camera(&self.wgpu.queue, scene);
    }

    pub fn update_light(&self, scene: &Scene) {
        self.rasterizer
            .borrow()
            .update_light(&self.wgpu.queue, scene);
        self.raytracer.update_light(&self.wgpu.queue, scene);
    }

    // pub fn update_probe_grid(&self, scene: &mut Scene) {
    //     scene.update_probe_grid(&self.wgpu.device, &self.wgpu.queue);
    // }
}
