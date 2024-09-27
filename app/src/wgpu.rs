use std::rc::Rc;

use bytemuck::NoUninit;
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{egui::initialize_egui, shapes::Pentagon};

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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RGBA {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl RGBA {
    pub fn new(rgba: [f32; 4]) -> Self {
        Self {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        }
    }
}

unsafe impl NoUninit for RGBA {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Resolution {
    width: f32,
    height: f32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
        }
    }
}

unsafe impl NoUninit for Resolution {}

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

pub fn update_buffer<T: NoUninit>(queue: &wgpu::Queue, wgpu_buffer: &wgpu::Buffer, value: T) {
    queue.write_buffer(&wgpu_buffer, 0, bytemuck::cast_slice(&[value]));
}

pub async fn initialize_wgpu<'window>(
    window: &'window Window,
    window_size: &winit::dpi::PhysicalSize<u32>,
) -> (
    wgpu::Instance,
    wgpu::Surface<'window>,
    wgpu::Adapter,
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
) {
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let surface_config = surface
        .get_default_config(&adapter, window_size.width, window_size.height)
        .expect("Failed to get default surface configuration");
    surface.configure(&device, &surface_config);

    (instance, surface, adapter, device, queue, surface_config)
}

pub fn initialize_rasterizer_shader(
    color_uniform: [f32; 4],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::ShaderModule,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::PipelineLayout,
    wgpu::RenderPipeline,
) {
    // Load the shaders from disk
    let rasterizer_shader =
        device.create_shader_module(wgpu::include_wgsl!("shaders/rasterizer/main.wgsl"));

    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Color Uniform Buffer"),
        size: std::mem::size_of::<RGBA>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Update the color buffer with the initial color
    update_buffer(queue, &color_uniform_buffer, RGBA::new(color_uniform));

    let color_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rasterizer Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rasterizer Bind Group"),
        layout: &color_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &color_uniform_buffer,
                offset: 0,
                size: None,
            }),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&color_bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let rasterizer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rasterizer Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rasterizer_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rasterizer_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

    (
        rasterizer_shader,
        color_uniform_buffer,
        color_bind_group,
        pipeline_layout,
        rasterizer_render_pipeline,
    )
}

pub fn initialize_raytracer_shader(
    color_uniform: [f32; 4],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
) -> (
    wgpu::ShaderModule,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::PipelineLayout,
    wgpu::RenderPipeline,
) {
    // Load the shaders from disk
    let raytracer_shader =
        device.create_shader_module(wgpu::include_wgsl!("shaders/raytracer/main.wgsl"));

    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Color Uniform Buffer"),
        size: std::mem::size_of::<RGBA>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Update the color buffer with the initial color
    update_buffer(queue, &color_uniform_buffer, RGBA::new(color_uniform));

    let resolution_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Resolution Uniform Buffer"),
        size: std::mem::size_of::<Resolution>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let color_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let raytracer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Raytracer Bind Group"),
        layout: &color_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: color_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: resolution_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let raytracer_render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&color_bind_group_layout],
            push_constant_ranges: &[],
        });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let raytracer_render_pipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raytracer Render Pipeline"),
            layout: Some(&raytracer_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &raytracer_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &raytracer_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

    (
        raytracer_shader,
        color_uniform_buffer,
        resolution_uniform_buffer,
        raytracer_bind_group,
        raytracer_render_pipeline_layout,
        raytracer_render_pipeline,
    )
}
