use std::{cell::RefCell, rc::Rc};

use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{
    camera::Camera,
    egui::initialize_egui,
    rasterizer::initialize_rasterizer,
    raytracer::{
        create_raytracer_bind_groups, create_raytracer_result_texture, initialize_raytracer,
    },
    shapes::Triangle,
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
                    .borrow_mut()
                    .on_window_event(&renderer.window, &window_event);

                if egui_event_response.repaint {
                    window.request_redraw();
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
    camera: Camera,
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Rc<&'window Window>,
    window_size: winit::dpi::PhysicalSize<u32>,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: Rc<RefCell<egui_winit::State>>,
    is_raytracer_enabled: bool,
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
        let window = Rc::new(window);
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        let camera = Camera::new(
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            (0.0, 0.0, 0.0).into(),
            // which way is "up"
            glam::Vec3::Y,
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            100.0,
        );

        let (_instance, surface, adapter, device, queue, surface_config) =
            initialize_wgpu(&window, &window_size).await;

        let (egui_renderer, egui_state) = initialize_egui(
            &window,
            &device,
            &surface_config,
            window.scale_factor() as f32,
        );
        let egui_state = Rc::new(RefCell::new(egui_state));

        // Initialize vertex and index buffers
        let shape = Triangle::new();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices Buffer"),
            contents: bytemuck::cast_slice(shape.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices Buffer"),
            contents: bytemuck::cast_slice(shape.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });
        let num_indices = shape.indices.len() as u32;

        let color_uniform = [1.0, 0.0, 0.0, 1.0];

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
            camera,
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

        // Recreate the raytracer result texture with the new size
        let (raytracer_result_texture, raytracer_result_texture_view) =
            create_raytracer_result_texture(&self.device, new_size.width, new_size.height);
        self.raytracer_result_texture = raytracer_result_texture;
        self.raytracer_result_texture_view = raytracer_result_texture_view;

        // Reconfigure the surface with the new size
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);

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
            self.run_raytracer();
        }

        // On macOS the window needs to be redrawn manually after resizing
        #[cfg(target_os = "macos")]
        {
            self.window.request_redraw();
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Command Encoder"),
                });

        let egui_state = self.egui_state.clone();
        let egui_raw_input = egui_state.borrow_mut().take_egui_input(&self.window);
        let egui_full_output =
            egui_state
                .borrow()
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
                                        self.run_raytracer();
                                    }
                                });
                        });
                });
        let egui_primitives = self
            .egui_state
            .borrow()
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

        if self.is_raytracer_enabled {
            let mut raytracer_rpass =
                render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Raytracer Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            raytracer_rpass.set_bind_group(0, &self.raytracer_render_bind_group, &[]);
            raytracer_rpass.set_pipeline(&self.raytracer_render_pipeline);
            raytracer_rpass.draw(0..3, 0..1);

            self.egui_renderer.render(
                &mut raytracer_rpass,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        } else {
            let mut rasterizer_rpass =
                render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Rasterizer Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            rasterizer_rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rasterizer_rpass
                .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rasterizer_rpass.set_bind_group(0, &self.rasterizer_bind_group, &[]);
            rasterizer_rpass.set_pipeline(&self.rasterizer_render_pipeline);
            rasterizer_rpass.draw_indexed(0..self.num_indices, 0, 0..1);

            self.egui_renderer.render(
                &mut rasterizer_rpass,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        }

        self.queue.submit(Some(render_encoder.finish()));
        frame.present();

        for id in egui_full_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }

        Ok(())
    }

    fn run_raytracer(&self) {
        let mut compute_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Command Encoder"),
                });

        {
            let mut raytracer_cpass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Raytracer Compute Pass"),
                    timestamp_writes: None,
                });

            raytracer_cpass.set_bind_group(0, &self.raytracer_compute_bind_group, &[]);
            raytracer_cpass.set_pipeline(&self.raytracer_compute_pipeline);
            raytracer_cpass.dispatch_workgroups(
                self.window_size.width / 8,
                self.window_size.height / 8,
                1,
            );
        }

        self.queue.submit(Some(compute_encoder.finish()));
    }
}
