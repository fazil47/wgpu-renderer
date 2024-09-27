use std::rc::Rc;

use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{
    egui::initialize_egui,
    rasterizer::initialize_rasterizer_shader,
    raytracer::initialize_raytracer_shader,
    shapes::Pentagon,
    wgpu::{initialize_wgpu, update_buffer, Resolution, RGBA},
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
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    is_raytracer_enabled: bool,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    color_uniform: [f32; 4],
    _rasterizer_shader: wgpu::ShaderModule,
    rasterizer_color_uniform_buffer: wgpu::Buffer,
    rasterizer_bind_group: wgpu::BindGroup,
    _rasterizer_render_pipeline_layout: wgpu::PipelineLayout,
    rasterizer_render_pipeline: wgpu::RenderPipeline,
    _raytracer_shader: wgpu::ShaderModule,
    raytracer_color_uniform_buffer: wgpu::Buffer,
    raytracer_resolution_uniform_buffer: wgpu::Buffer,
    raytracer_bind_group: wgpu::BindGroup,
    _raytracer_render_pipeline_layout: wgpu::PipelineLayout,
    raytracer_render_pipeline: wgpu::RenderPipeline,
}

impl<'window> Renderer<'window> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'window Window) -> Renderer<'window> {
        let window = Rc::new(window);
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        let (_instance, surface, _adapter, device, queue, surface_config) =
            initialize_wgpu(&window, &window_size).await;

        let (egui_renderer, egui_state) = initialize_egui(
            &window,
            &device,
            &surface_config,
            window.scale_factor() as f32,
        );

        // Initialize vertex and index buffers
        let shape = Pentagon::new();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(shape.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(shape.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = shape.indices.len() as u32;

        let color_uniform = [1.0, 0.0, 0.0, 1.0];

        let (
            _rasterizer_shader,
            rasterizer_color_uniform_buffer,
            rasterizer_bind_group,
            _rasterizer_render_pipeline_layout,
            rasterizer_render_pipeline,
        ) = initialize_rasterizer_shader(color_uniform, &device, &queue, &surface, &_adapter);

        let (
            _raytracer_shader,
            raytracer_color_uniform_buffer,
            raytracer_resolution_uniform_buffer,
            raytracer_bind_group,
            _raytracer_render_pipeline_layout,
            raytracer_render_pipeline,
        ) = initialize_raytracer_shader(color_uniform, &device, &queue, &surface, &_adapter);

        Self {
            _instance,
            surface,
            _adapter,
            device,
            queue,
            surface_config,
            window,
            egui_renderer,
            egui_state,
            is_raytracer_enabled: false,
            vertex_buffer,
            index_buffer,
            num_indices,
            color_uniform,
            _rasterizer_shader,
            rasterizer_color_uniform_buffer,
            rasterizer_bind_group,
            _rasterizer_render_pipeline_layout,
            rasterizer_render_pipeline,
            _raytracer_shader,
            raytracer_color_uniform_buffer,
            raytracer_bind_group,
            raytracer_resolution_uniform_buffer,
            _raytracer_render_pipeline_layout,
            raytracer_render_pipeline,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Reconfigure the surface with the new size
        self.surface_config.width = new_size.width.max(1);
        self.surface_config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);

        update_buffer(
            &self.queue,
            &self.raytracer_resolution_uniform_buffer,
            Resolution::new(self.surface_config.width, self.surface_config.height),
        );

        // On macos the window needs to be redrawn manually after resizing
        self.window.request_redraw();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
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
                                    RGBA::new(self.color_uniform),
                                );

                                update_buffer(
                                    &self.queue,
                                    &self.raytracer_color_uniform_buffer,
                                    RGBA::new(self.color_uniform),
                                )
                            }

                            ui.checkbox(&mut self.is_raytracer_enabled, "Raytracing");
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
            &mut encoder,
            &egui_primitives,
            &egui_screen_descriptor,
        );

        if self.is_raytracer_enabled {
            let mut raytracer_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            raytracer_rpass.set_pipeline(&self.raytracer_render_pipeline);
            raytracer_rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            raytracer_rpass
                .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            raytracer_rpass.set_bind_group(0, &self.raytracer_bind_group, &[]);
            raytracer_rpass.draw_indexed(0..self.num_indices, 0, 0..1);

            self.egui_renderer.render(
                &mut raytracer_rpass,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        } else {
            let mut rasterizer_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            rasterizer_rpass.set_pipeline(&self.rasterizer_render_pipeline);
            rasterizer_rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rasterizer_rpass
                .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rasterizer_rpass.set_bind_group(0, &self.rasterizer_bind_group, &[]);
            rasterizer_rpass.draw_indexed(0..self.num_indices, 0, 0..1);

            self.egui_renderer.render(
                &mut rasterizer_rpass,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        for id in egui_full_output.textures_delta.free {
            self.egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}
