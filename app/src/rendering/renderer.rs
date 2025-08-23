use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::window::Window;

use crate::{
    core::engine::EngineConfiguration, rendering::rasterizer::Rasterizer,
    rendering::raytracer::Raytracer, rendering::wgpu_utils::WgpuResources, ui::egui::RendererEgui,
};
use ecs::{EntityId, Resource, World};

pub struct Renderer {
    pub rasterizer: Rc<RefCell<Rasterizer>>,
    pub raytracer: Raytracer,
    pub egui: RendererEgui,
    pub wgpu: WgpuResources,
}

impl Resource for Renderer {}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        window_size: &winit::dpi::PhysicalSize<u32>,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
    ) -> Self {
        let wgpu = WgpuResources::new(window.clone(), window_size).await;
        let egui = RendererEgui::new(
            &window,
            &wgpu.device,
            &wgpu.surface_config,
            window.scale_factor() as f32,
        );

        let mut rasterizer = Rasterizer::new(&wgpu);
        if let Err(err) = rasterizer.update_scene_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Failed to update rasterizer scene data: {err}");
        }
        let rasterizer = Rc::new(RefCell::new(rasterizer));

        let mut raytracer = Raytracer::new(&wgpu, window_size);
        if let Err(err) = raytracer.update_scene_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Failed to update raytracer scene data: {err}");
        }

        Self {
            wgpu,
            rasterizer,
            raytracer,
            egui,
        }
    }

    pub fn update_scene_data(&mut self, world: &World) {
        let camera_entity = world
            .get_entities_with::<crate::rendering::Camera>()
            .into_iter()
            .next();
        let sun_light_entity = world
            .get_entities_with::<crate::lighting::DirectionalLight>()
            .into_iter()
            .next();

        if camera_entity.is_none() || sun_light_entity.is_none() {
            eprintln!("Warning: No camera or sun light entity found in the world.");
            return;
        }

        if sun_light_entity.is_none() {
            eprintln!("Warning: No sun light entity found in the world.");
            return;
        }

        let camera_entity = camera_entity.unwrap();
        let sun_light_entity = sun_light_entity.unwrap();

        if let Err(err) = self.rasterizer.borrow_mut().update_scene_data(
            &self.wgpu.device,
            &self.wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Error updating scene data: {err}");
        }

        if let Err(err) = self.raytracer.update_scene_data(
            &self.wgpu.device,
            &self.wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Error updating scene data: {err}");
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, _world: &World) {
        // We'll need camera_entity to be passed in, for now skip the camera update
        // self.update_camera(world, camera_entity);

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
        run_ui: &mut impl FnMut(&egui::Context, &RendererEgui),
    ) -> egui::FullOutput {
        let egui_raw_input = self.egui.state.take_egui_input(window);
        let egui_ref = &self.egui;
        self.egui
            .state
            .egui_ctx()
            .run(egui_raw_input, |ctx| run_ui(ctx, egui_ref))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        window: &winit::window::Window,
        window_size: &winit::dpi::PhysicalSize<u32>,
        config: &EngineConfiguration,
        world: &World,
        camera_entity: EntityId,
        sun_light_entity: EntityId,
        frame_count: u32,
        egui_output: egui::FullOutput,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_camera(world, camera_entity);

        // Light dirty check is now handled by Engine
        self.update_light(world, sun_light_entity);

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
                    &mut render_encoder,
                    &surface_texture_view,
                    camera_entity,
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

    pub fn update_camera(&self, world: &World, camera_entity: EntityId) {
        self.rasterizer
            .borrow()
            .update_camera(&self.wgpu.queue, world, camera_entity);
        self.raytracer
            .update_camera(&self.wgpu.queue, world, camera_entity);
        self.egui.update_camera(world, camera_entity);
    }

    pub fn update_light(&self, world: &World, sun_light_entity: EntityId) {
        self.rasterizer
            .borrow()
            .update_light(&self.wgpu.queue, world, sun_light_entity);
        self.raytracer
            .update_light(&self.wgpu.queue, world, sun_light_entity);
    }
}
