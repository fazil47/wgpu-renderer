use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController},
    egui::render_egui,
    rasterizer::render_rasterizer,
    raytracer::{
        create_raytracer_bind_groups, create_raytracer_result_texture, render_raytracer,
        run_raytracer,
    },
    renderer::Renderer,
    scene::Scene,
    wgpu::update_buffer,
};

pub struct Engine {
    // The window must be declared after the wgpu surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    pub window: Arc<Window>,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub camera_controller: CameraController,
    pub renderer: Renderer,
    scene: Scene,
    stat: EngineStatistics,
    config: EngineConfiguration,
}

impl Engine {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: Arc<Window>) -> Engine {
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        // position the camera 4 units back
        // +z is out of the screen
        let camera_position: maths::Vec3 = (0.0, 0.0, 4.0).into();
        let camera = Camera::new(
            camera_position,
            -camera_position.normalize(), // have the camera look at the origin
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            100.0,
        );
        let camera_controller = CameraController::new(camera, 0.8);
        let scene = Scene::default();

        let renderer = Renderer::new(
            window.clone(),
            &window_size,
            &camera_controller.camera,
            &scene,
        )
        .await;

        Self {
            window,
            window_size,
            camera_controller,
            renderer,
            scene,
            stat: EngineStatistics::default(),
            config: EngineConfiguration::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;

        // Update camera
        self.camera_controller
            .camera
            .set_aspect(new_size.width as f32 / new_size.height as f32);
        self.update_camera_uniforms();

        Self::reset_frame_count(
            &mut self.stat.frame_count,
            &self.renderer.wgpu,
            &self.renderer.raytracer,
        );

        // Recreate the raytracer result texture with the new size
        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(
                &self.renderer.wgpu.device,
                new_size.width,
                new_size.height,
            );
        self.renderer.raytracer.result_texture = raytracer_result_texture;
        self.renderer.raytracer.result_texture_view = raytracer_result_texture_view;

        // Reconfigure the surface with the new size
        self.renderer.wgpu.surface_config.width = new_size.width.max(1);
        self.renderer.wgpu.surface_config.height = new_size.height.max(1);
        self.renderer.wgpu.surface.configure(
            &self.renderer.wgpu.device,
            &self.renderer.wgpu.surface_config,
        );

        self.renderer.rasterizer.depth_texture = crate::wgpu::Texture::create_depth_texture(
            &self.renderer.wgpu.device,
            &self.renderer.wgpu.surface_config,
            "depth_texture",
        );

        // Recreate the raytracer bind groups with the new texture view
        let (raytracer_render_bind_group, raytracer_compute_bind_group) =
            create_raytracer_bind_groups(
                &self.renderer.raytracer.result_texture_view,
                &self.renderer.wgpu.device,
                &self.renderer.raytracer.render_bind_group_layout,
                &self.renderer.raytracer.compute_bind_group_layout,
                &self.renderer.vertex_buffer,
                &self.renderer.index_buffer,
                &self.renderer.raytracer.frame_count_uniform_buffer,
                &self.renderer.raytracer.vertex_stride_uniform_buffer,
                &self.renderer.raytracer.vertex_color_offset_uniform_buffer,
                &self.renderer.raytracer.vertex_normal_offset_uniform_buffer,
                &self.renderer.raytracer.camera_to_world_uniform_buffer,
                &self
                    .renderer
                    .raytracer
                    .camera_inverse_projection_uniform_buffer,
                &self.renderer.raytracer.sun_direction_uniform_buffer,
            );
        self.renderer.raytracer.render_bind_group = raytracer_render_bind_group;
        self.renderer.raytracer.compute_bind_group = raytracer_compute_bind_group;

        // On macOS the window needs to be redrawn manually after resizing
        #[cfg(target_os = "macos")]
        {
            self.window.request_redraw();
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.config.is_raytracer_enabled
            && self.stat.frame_count < self.config.raytracer_max_frames
        {
            run_raytracer(
                &self.renderer.wgpu.device,
                &self.renderer.wgpu.queue,
                self.window_size,
                &self.renderer.raytracer.compute_bind_group,
                &self.renderer.raytracer.compute_pipeline,
            );

            Self::increment_frame_count(
                &mut self.stat.frame_count,
                &self.renderer.wgpu,
                &self.renderer.raytracer,
            );
        }

        // Update delta time
        let current_time = Instant::now();
        self.stat.delta_time = current_time
            .duration_since(self.stat.last_frame_time)
            .as_secs_f32();
        self.stat.last_frame_time = current_time;

        let surface_texture = self.renderer.wgpu.surface.get_current_texture()?;

        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder =
            self.renderer
                .wgpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Command Encoder"),
                });

        let egui_raw_input = self.renderer.egui.state.take_egui_input(&self.window);
        let egui_full_output =
            self.renderer
                .egui
                .state
                .egui_ctx()
                .run(egui_raw_input, |egui_ctx: &egui::Context| {
                    egui::SidePanel::right("fps_panel")
                        .exact_width(150.0)
                        .show_separator_line(false)
                        .resizable(false)
                        .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                        .show(egui_ctx, |ui| {
                            ui.label(format!(
                                "Frame Time: {:.2}ms",
                                self.stat.delta_time * 1000.0
                            ));
                            ui.label(format!("FPS: {:.1}", 1.0 / self.stat.delta_time));

                            if self.config.is_raytracer_enabled {
                                ui.label(format!("Frame Count: {}", self.stat.frame_count));
                            }
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                        .show(egui_ctx, |ui| {
                            let sun_azi_changed = ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.scene.sun_light.azimuth,
                                        0.0..=360.0,
                                    )
                                    .text("Sun Azimuth"),
                                )
                                .changed();

                            let sun_alt_changed = ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.scene.sun_light.altitude,
                                        0.0..=90.0,
                                    )
                                    .text("Sun Altitude"),
                                )
                                .changed();

                            if sun_azi_changed || sun_alt_changed {
                                self.scene.sun_light.recalculate();

                                update_buffer(
                                    &self.renderer.wgpu.queue,
                                    &self.renderer.rasterizer.sun_direction_uniform_buffer,
                                    &self.scene.sun_light.direction.to_array(),
                                );
                                update_buffer(
                                    &self.renderer.wgpu.queue,
                                    &self.renderer.raytracer.sun_direction_uniform_buffer,
                                    &self.scene.sun_light.direction.to_array(),
                                );

                                Self::reset_frame_count(
                                    &mut self.stat.frame_count,
                                    &self.renderer.wgpu,
                                    &self.renderer.raytracer,
                                );
                            }

                            // Run the raytracer when the checkbox is toggled on
                            if ui
                                .checkbox(&mut self.config.is_raytracer_enabled, "Raytracing")
                                .changed()
                            {
                                Self::reset_frame_count(
                                    &mut self.stat.frame_count,
                                    &self.renderer.wgpu,
                                    &self.renderer.raytracer,
                                );
                            }
                        });
                });
        let egui_primitives = self
            .renderer
            .egui
            .state
            .egui_ctx()
            .tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let egui_screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                self.renderer.wgpu.surface_config.width,
                self.renderer.wgpu.surface_config.height,
            ],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, image_delta) in egui_full_output.textures_delta.set {
            self.renderer.egui.renderer.update_texture(
                &self.renderer.wgpu.device,
                &self.renderer.wgpu.queue,
                id,
                &image_delta,
            );
        }

        {
            if self.config.is_raytracer_enabled {
                render_raytracer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.renderer.raytracer.render_bind_group,
                    &self.renderer.raytracer.render_pipeline,
                );
            } else {
                render_rasterizer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.renderer.rasterizer.depth_texture,
                    &self.renderer.vertex_buffer,
                    &self.renderer.index_buffer,
                    self.renderer.num_indices,
                    &self.renderer.rasterizer.bind_group,
                    &self.renderer.rasterizer.render_pipeline,
                );
            };

            render_egui(
                &self.renderer.wgpu.device,
                &self.renderer.wgpu.queue,
                &mut render_encoder,
                &surface_texture_view,
                &mut self.renderer.egui.renderer,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        }

        self.renderer
            .wgpu
            .queue
            .submit(Some(render_encoder.finish()));
        surface_texture.present();

        for id in egui_full_output.textures_delta.free {
            self.renderer.egui.renderer.free_texture(&id);
        }

        Ok(())
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera_controller.process_events(event);
    }

    pub fn update(&mut self) {
        self.stat.time_since_last_frame += self.stat.delta_time;

        // Raytracing is expensive, so run it every 4 frames
        if self.config.is_raytracer_enabled
            && self.stat.time_since_last_frame < self.config.target_frame_time * 4.0
        {
            return;
        } else if self.stat.time_since_last_frame < self.config.target_frame_time {
            return;
        } else {
            self.stat.time_since_last_frame = 0.0;
        }

        self.camera_controller.update_camera(self.stat.delta_time);
        self.update_camera_uniforms();

        Self::reset_frame_count(
            &mut self.stat.frame_count,
            &self.renderer.wgpu,
            &self.renderer.raytracer,
        );

        self.window.request_redraw();
    }

    fn update_camera_uniforms(&self) {
        update_buffer(
            &self.renderer.wgpu.queue,
            &self.renderer.rasterizer.camera_view_proj_uniform,
            &[self
                .camera_controller
                .camera
                .view_projection()
                .to_cols_array_2d()],
        );

        update_buffer(
            &self.renderer.wgpu.queue,
            &self.renderer.raytracer.camera_to_world_uniform_buffer,
            &[self
                .camera_controller
                .camera
                .camera_to_world()
                .to_cols_array_2d()],
        );

        update_buffer(
            &self.renderer.wgpu.queue,
            &self
                .renderer
                .raytracer
                .camera_inverse_projection_uniform_buffer,
            &[self
                .camera_controller
                .camera
                .camera_inverse_projection()
                .to_cols_array_2d()],
        );
    }

    fn increment_frame_count(
        frame_count: &mut u32,
        wgpu: &crate::wgpu::RendererWgpuResources,
        raytracer: &crate::raytracer::Raytracer,
    ) {
        *frame_count += 1;

        update_buffer(
            &wgpu.queue,
            &raytracer.frame_count_uniform_buffer,
            &[*frame_count],
        );
    }

    fn reset_frame_count(
        frame_count: &mut u32,
        wgpu: &crate::wgpu::RendererWgpuResources,
        raytracer: &crate::raytracer::Raytracer,
    ) {
        *frame_count = 0;

        update_buffer(
            &wgpu.queue,
            &raytracer.frame_count_uniform_buffer,
            &[*frame_count],
        );
    }
}

struct EngineStatistics {
    time_since_last_frame: f32,
    last_frame_time: Instant,
    delta_time: f32,
    frame_count: u32,
}

impl Default for EngineStatistics {
    fn default() -> Self {
        Self {
            time_since_last_frame: 0.0,
            last_frame_time: Instant::now(),
            delta_time: 0.0,
            frame_count: 0,
        }
    }
}

struct EngineConfiguration {
    target_frame_time: f32,
    raytracer_max_frames: u32,
    is_raytracer_enabled: bool,
}

impl Default for EngineConfiguration {
    fn default() -> Self {
        Self {
            target_frame_time: 1.0 / 120.0,
            raytracer_max_frames: 100,
            is_raytracer_enabled: false,
        }
    }
}
