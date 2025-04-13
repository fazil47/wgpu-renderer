use std::sync::Arc;
use winit::window::Window;

use crate::{
    camera::{Camera, CameraController},
    egui::RendererEgui,
    engine::EngineConfiguration,
    rasterizer::{self, initialize_rasterizer, render_rasterizer},
    raytracer::{
        self, compute_raytracer, create_raytracer_bind_groups, create_raytracer_result_texture,
        initialize_raytracer, render_raytracer,
    },
    scene::Scene,
    wgpu::{RendererWgpu, update_buffer},
};

pub struct Renderer {
    pub rasterizer: rasterizer::Rasterizer,
    pub raytracer: raytracer::Raytracer,
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

        let (
            rasterizer_camera_view_proj_uniform,
            rasterizer_sun_direction_uniform_buffer,
            rasterizer_bind_group,
            rasterizer_render_pipeline,
        ) = initialize_rasterizer(
            &camera,
            &scene.sun_light.direction,
            &wgpu.device,
            &wgpu.surface,
            &wgpu.adapter,
        );

        let rasterizer_depth_texture = crate::wgpu::Texture::create_depth_texture(
            &wgpu.device,
            &wgpu.surface_config,
            "rasterizer_depth_texture",
        );

        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&wgpu.device, window_size.width, window_size.height);

        let (
            raytracer_render_bind_group_layout,
            raytracer_render_bind_group,
            raytracer_render_pipeline,
            raytracer_frame_count_uniform_buffer,
            raytracer_vertex_stride_uniform_buffer,
            raytracer_vertex_color_offset_uniform_buffer,
            raytracer_vertex_normal_offset_uniform_buffer,
            raytracer_camera_to_world_uniform_buffer,
            raytracer_camera_inverse_projection_uniform_buffer,
            raytracer_sun_direction_uniform_buffer,
            raytracer_compute_bind_group_layout,
            raytracer_compute_bind_group,
            raytracer_compute_pipeline,
        ) = initialize_raytracer(
            0,
            &vertex_buffer,
            &index_buffer,
            &camera,
            &scene.sun_light.direction,
            &raytracer_result_texture_view,
            &wgpu.device,
            &wgpu.surface,
            &wgpu.adapter,
        );

        Self {
            wgpu,
            egui,
            vertex_buffer,
            index_buffer,
            num_indices,
            rasterizer: rasterizer::Rasterizer {
                depth_texture: rasterizer_depth_texture,
                camera_view_proj_uniform: rasterizer_camera_view_proj_uniform,
                sun_direction_uniform_buffer: rasterizer_sun_direction_uniform_buffer,
                bind_group: rasterizer_bind_group,
                render_pipeline: rasterizer_render_pipeline,
            },
            raytracer: raytracer::Raytracer {
                result_texture: raytracer_result_texture,
                result_texture_view: raytracer_result_texture_view,
                render_bind_group_layout: raytracer_render_bind_group_layout,
                render_bind_group: raytracer_render_bind_group,
                render_pipeline: raytracer_render_pipeline,
                frame_count_uniform_buffer: raytracer_frame_count_uniform_buffer,
                vertex_stride_uniform_buffer: raytracer_vertex_stride_uniform_buffer,
                vertex_color_offset_uniform_buffer: raytracer_vertex_color_offset_uniform_buffer,
                vertex_normal_offset_uniform_buffer: raytracer_vertex_normal_offset_uniform_buffer,
                camera_to_world_uniform_buffer: raytracer_camera_to_world_uniform_buffer,
                camera_inverse_projection_uniform_buffer:
                    raytracer_camera_inverse_projection_uniform_buffer,
                sun_direction_uniform_buffer: raytracer_sun_direction_uniform_buffer,
                compute_bind_group_layout: raytracer_compute_bind_group_layout,
                compute_bind_group: raytracer_compute_bind_group,
                compute_pipeline: raytracer_compute_pipeline,
            },
        }
    }

    pub fn update_frame_count(&self, frame_count: &u32) {
        update_buffer(
            &self.wgpu.queue,
            &self.raytracer.frame_count_uniform_buffer,
            &[*frame_count],
        );
    }

    pub fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        camera_controller: &CameraController,
    ) {
        self.update_camera(camera_controller);

        // Reconfigure the surface with the new size
        self.wgpu.resize(&new_size);

        self.rasterizer.depth_texture = crate::wgpu::Texture::create_depth_texture(
            &self.wgpu.device,
            &self.wgpu.surface_config,
            "depth_texture",
        );

        // Recreate the raytracer result texture with the new size
        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&self.wgpu.device, new_size.width, new_size.height);
        self.raytracer.result_texture = raytracer_result_texture;
        self.raytracer.result_texture_view = raytracer_result_texture_view;

        // Recreate the raytracer bind groups with the new texture view
        let (raytracer_render_bind_group, raytracer_compute_bind_group) =
            create_raytracer_bind_groups(
                &self.raytracer.result_texture_view,
                &self.wgpu.device,
                &self.raytracer.render_bind_group_layout,
                &self.raytracer.compute_bind_group_layout,
                &self.vertex_buffer,
                &self.index_buffer,
                &self.raytracer.frame_count_uniform_buffer,
                &self.raytracer.vertex_stride_uniform_buffer,
                &self.raytracer.vertex_color_offset_uniform_buffer,
                &self.raytracer.vertex_normal_offset_uniform_buffer,
                &self.raytracer.camera_to_world_uniform_buffer,
                &self.raytracer.camera_inverse_projection_uniform_buffer,
                &self.raytracer.sun_direction_uniform_buffer,
            );
        self.raytracer.render_bind_group = raytracer_render_bind_group;
        self.raytracer.compute_bind_group = raytracer_compute_bind_group;
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
        frame_count: &u32,
        egui_output: egui::FullOutput,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_camera(camera_controller);

        if config.is_raytracer_enabled && *frame_count < config.raytracer_max_frames {
            compute_raytracer(
                &window_size,
                &self.wgpu.device,
                &self.wgpu.queue,
                &self.raytracer.compute_bind_group,
                &self.raytracer.compute_pipeline,
            );

            update_buffer(
                &self.wgpu.queue,
                &self.raytracer.frame_count_uniform_buffer,
                &[*frame_count],
            );
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
                render_raytracer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.raytracer.render_bind_group,
                    &self.raytracer.render_pipeline,
                );
            } else {
                render_rasterizer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.rasterizer.depth_texture,
                    &self.vertex_buffer,
                    &self.index_buffer,
                    self.num_indices,
                    &self.rasterizer.bind_group,
                    &self.rasterizer.render_pipeline,
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
        update_buffer(
            &self.wgpu.queue,
            &self.rasterizer.camera_view_proj_uniform,
            &[camera_controller
                .camera
                .view_projection()
                .to_cols_array_2d()],
        );

        update_buffer(
            &self.wgpu.queue,
            &self.raytracer.camera_to_world_uniform_buffer,
            &[camera_controller
                .camera
                .camera_to_world()
                .to_cols_array_2d()],
        );

        update_buffer(
            &self.wgpu.queue,
            &self.raytracer.camera_inverse_projection_uniform_buffer,
            &[camera_controller
                .camera
                .camera_inverse_projection()
                .to_cols_array_2d()],
        );
    }
}
