use std::{rc::Rc, time::Instant};

use glam::Vec3;
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{
    camera::{Camera, CameraController},
    egui::initialize_egui,
    rasterizer::{initialize_rasterizer, render_rasterizer},
    raytracer::{
        create_raytracer_bind_groups, create_raytracer_result_texture, initialize_raytracer,
        render_raytracer, run_raytracer,
    },
    wgpu::{initialize_wgpu, update_buffer},
};

pub async fn run(event_loop: EventLoop<()>, window: Window) {
    let window = Rc::new(window);
    let renderer_window = window.clone();
    let mut renderer = Renderer::new(&renderer_window).await;

    event_loop
        .run(move |event, target| {
            if let Event::WindowEvent {
                window_id: _,
                event: window_event,
            } = event
            {
                let egui_event_response = renderer
                    .egui_state
                    .on_window_event(&renderer.window, &window_event);

                if egui_event_response.repaint {
                    window.request_redraw();
                }

                renderer.input(&window_event);
                if renderer.camera_controller.is_cursor_locked() {
                    renderer.update();
                }

                if egui_event_response.consumed {
                    return;
                }

                match window_event {
                    WindowEvent::Resized(new_size) => renderer.resize(new_size),

                    WindowEvent::RedrawRequested => renderer.render().unwrap(),

                    WindowEvent::CloseRequested => target.exit(),

                    _ => {}
                };
            }
        })
        .unwrap();
}

struct Renderer<'window> {
    target_frame_time: f32,
    time_since_last_frame: f32,
    camera: Camera,
    camera_controller: CameraController,
    last_frame_time: Instant,
    delta_time: f32,
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'window Window,
    window_size: winit::dpi::PhysicalSize<u32>,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    is_raytracer_enabled: bool,
    depth_texture: crate::wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    color_uniform: [f32; 4],
    rasterizer_camera_view_proj_uniform: wgpu::Buffer,
    rasterizer_color_uniform_buffer: wgpu::Buffer,
    rasterizer_bind_group: wgpu::BindGroup,
    rasterizer_render_pipeline: wgpu::RenderPipeline,
    raytracer_result_texture: wgpu::Texture,
    raytracer_result_texture_view: wgpu::TextureView,
    raytracer_render_bind_group_layout: wgpu::BindGroupLayout,
    raytracer_render_bind_group: wgpu::BindGroup,
    raytracer_render_pipeline: wgpu::RenderPipeline,
    raytracer_vertex_stride_uniform_buffer: wgpu::Buffer,
    raytracer_vertex_color_offset_uniform_buffer: wgpu::Buffer,
    raytracer_camera_to_world_uniform_buffer: wgpu::Buffer,
    raytracer_camera_inverse_projection_uniform_buffer: wgpu::Buffer,
    raytracer_compute_bind_group_layout: wgpu::BindGroupLayout,
    raytracer_compute_bind_group: wgpu::BindGroup,
    raytracer_compute_pipeline: wgpu::ComputePipeline,
}

impl<'window> Renderer<'window> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'window Window) -> Renderer<'window> {
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        // position the camera 4 units back
        // +z is out of the screen
        let camera_position: Vec3 = (0.0, 0.0, 4.0).into();
        let camera = Camera::new(
            camera_position,
            -camera_position.normalize(), // have the camera look at the origin
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            100.0,
        );
        let camera_controller = CameraController::new(0.8);

        let (_instance, surface, adapter, device, queue, surface_config) =
            initialize_wgpu(&window, &window_size).await;

        let (egui_renderer, egui_state) = initialize_egui(
            &window,
            &device,
            &surface_config,
            window.scale_factor() as f32,
        );

        let depth_texture =
            crate::wgpu::Texture::create_depth_texture(&device, &surface_config, "depth_texture");

        // Initialize vertex and index buffers
        let mesh = crate::mesh::PlyMesh::new("assets/cornell-box.ply");
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        let num_indices = mesh.indices.len() as u32;

        let color_uniform = [1.0, 1.0, 1.0, 1.0];

        let (
            rasterizer_camera_view_proj_uniform,
            rasterizer_color_uniform_buffer,
            rasterizer_bind_group,
            rasterizer_render_pipeline,
        ) = initialize_rasterizer(&camera, &color_uniform, &device, &surface, &adapter);

        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&device, window_size.width, window_size.height);

        let (
            raytracer_render_bind_group_layout,
            raytracer_render_bind_group,
            raytracer_render_pipeline,
            raytracer_vertex_stride_uniform_buffer,
            raytracer_vertex_color_offset_uniform_buffer,
            raytracer_camera_to_world_uniform_buffer,
            raytracer_camera_inverse_projection_uniform_buffer,
            raytracer_compute_bind_group_layout,
            raytracer_compute_bind_group,
            raytracer_compute_pipeline,
        ) = initialize_raytracer(
            &vertex_buffer,
            &index_buffer,
            &camera,
            &raytracer_result_texture_view,
            &device,
            &surface,
            &adapter,
        );

        Self {
            target_frame_time: 1.0 / 120.0,
            time_since_last_frame: 0.0,
            camera,
            camera_controller,
            last_frame_time: Instant::now(),
            delta_time: 0.0,
            _instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            surface_config,
            window,
            window_size,
            egui_renderer,
            egui_state,
            is_raytracer_enabled: false,
            depth_texture,
            vertex_buffer,
            index_buffer,
            num_indices,
            color_uniform,
            rasterizer_camera_view_proj_uniform,
            rasterizer_color_uniform_buffer,
            rasterizer_bind_group,
            rasterizer_render_pipeline,
            raytracer_result_texture,
            raytracer_result_texture_view,
            raytracer_render_bind_group_layout,
            raytracer_render_bind_group,
            raytracer_render_pipeline,
            raytracer_vertex_stride_uniform_buffer,
            raytracer_vertex_color_offset_uniform_buffer,
            raytracer_camera_to_world_uniform_buffer,
            raytracer_camera_inverse_projection_uniform_buffer,
            raytracer_compute_bind_group_layout,
            raytracer_compute_bind_group,
            raytracer_compute_pipeline,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;

        // Update camera
        self.camera
            .set_aspect(new_size.width as f32 / new_size.height as f32);
        self.update_camera_uniforms();

        // Recreate the raytracer result texture with the new size
        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&self.device, new_size.width, new_size.height);
        self.raytracer_result_texture = raytracer_result_texture;
        self.raytracer_result_texture_view = raytracer_result_texture_view;

        // Reconfigure the surface with the new size
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);

        self.depth_texture = crate::wgpu::Texture::create_depth_texture(
            &self.device,
            &self.surface_config,
            "depth_texture",
        );

        // Recreate the raytracer bind groups with the new texture view
        let (raytracer_render_bind_group, raytracer_compute_bind_group) =
            create_raytracer_bind_groups(
                &self.raytracer_result_texture_view,
                &self.device,
                &self.raytracer_render_bind_group_layout,
                &self.raytracer_compute_bind_group_layout,
                &self.vertex_buffer,
                &self.index_buffer,
                &self.raytracer_vertex_stride_uniform_buffer,
                &self.raytracer_vertex_color_offset_uniform_buffer,
                &self.raytracer_camera_to_world_uniform_buffer,
                &self.raytracer_camera_inverse_projection_uniform_buffer,
            );
        self.raytracer_render_bind_group = raytracer_render_bind_group;
        self.raytracer_compute_bind_group = raytracer_compute_bind_group;

        if self.is_raytracer_enabled {
            run_raytracer(
                &self.device,
                &self.queue,
                self.window_size,
                &self.raytracer_compute_bind_group,
                &self.raytracer_compute_pipeline,
            );
        }

        // On macOS the window needs to be redrawn manually after resizing
        #[cfg(target_os = "macos")]
        {
            self.window.request_redraw();
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Update delta time
        let current_time = Instant::now();
        self.delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        let surface_texture = self.surface.get_current_texture()?;

        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Command Encoder"),
                });

        let egui_raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_full_output =
            self.egui_state
                .egui_ctx()
                .run(egui_raw_input, |egui_ctx: &egui::Context| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().inner_margin(egui::Margin::same(10.0)))
                        .show(egui_ctx, |ui| {
                            if ui
                                .color_edit_button_rgba_unmultiplied(&mut self.color_uniform)
                                .changed()
                            {
                                update_buffer(
                                    &self.queue,
                                    &self.rasterizer_color_uniform_buffer,
                                    &self.color_uniform,
                                );
                            }

                            // Run the raytracer when the checkbox is toggled on
                            ui.checkbox(&mut self.is_raytracer_enabled, "Raytracing")
                                .changed()
                                .then(|| {
                                    if self.is_raytracer_enabled {
                                        run_raytracer(
                                            &self.device,
                                            &self.queue,
                                            self.window_size,
                                            &self.raytracer_compute_bind_group,
                                            &self.raytracer_compute_pipeline,
                                        );
                                    }
                                });
                        });
                });
        let egui_primitives = self
            .egui_state
            .egui_ctx()
            .tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let egui_screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, image_delta) in egui_full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, id, &image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut render_encoder,
            &egui_primitives,
            &egui_screen_descriptor,
        );

        {
            if self.is_raytracer_enabled {
                render_raytracer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.raytracer_render_bind_group,
                    &self.raytracer_render_pipeline,
                );
            } else {
                render_rasterizer(
                    &mut render_encoder,
                    &surface_texture_view,
                    &self.depth_texture,
                    &self.vertex_buffer,
                    &self.index_buffer,
                    self.num_indices,
                    &self.rasterizer_bind_group,
                    &self.rasterizer_render_pipeline,
                );
            };
        }

        {
            let mut egui_rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            self.egui_renderer
                .render(&mut egui_rpass, &egui_primitives, &egui_screen_descriptor);
        }

        self.queue.submit(Some(render_encoder.finish()));
        surface_texture.present();

        for id in egui_full_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }

        Ok(())
    }

    fn input(&mut self, event: &WindowEvent) {
        self.camera_controller.process_events(event);
    }

    fn update(&mut self) {
        self.time_since_last_frame += self.delta_time;

        // Raytracing is expensive, so run it every 4 frames
        if self.is_raytracer_enabled && self.time_since_last_frame < self.target_frame_time * 4.0 {
            return;
        } else if self.time_since_last_frame < self.target_frame_time {
            return;
        } else {
            self.time_since_last_frame = 0.0;
        }

        self.camera_controller
            .update_camera(&mut self.camera, self.delta_time);
        self.update_camera_uniforms();

        if self.is_raytracer_enabled {
            run_raytracer(
                &self.device,
                &self.queue,
                self.window_size,
                &self.raytracer_compute_bind_group,
                &self.raytracer_compute_pipeline,
            );
        }

        self.window.request_redraw();
    }

    fn update_camera_uniforms(&self) {
        update_buffer(
            &self.queue,
            &self.rasterizer_camera_view_proj_uniform,
            &[self.camera.view_projection().to_cols_array_2d()],
        );

        update_buffer(
            &self.queue,
            &self.raytracer_camera_to_world_uniform_buffer,
            &[self.camera.camera_to_world().to_cols_array_2d()],
        );

        update_buffer(
            &self.queue,
            &self.raytracer_camera_inverse_projection_uniform_buffer,
            &[self.camera.camera_inverse_projection().to_cols_array_2d()],
        );
    }
}
