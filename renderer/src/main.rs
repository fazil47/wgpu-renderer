use encase::{ShaderType, UniformBuffer};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

#[derive(ShaderType)]
struct RGB {
    r: f32,
    g: f32,
    b: f32,
    _padding: f32,
}

impl RGB {
    fn new(rgb: [f32; 3]) -> Self {
        Self {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
            _padding: 0.0,
        }
    }
}

fn update_color_buffer(queue: &wgpu::Queue, wgpu_buffer: &wgpu::Buffer, color: &RGB) {
    let mut encase_buffer = UniformBuffer::new(Vec::new());
    encase_buffer.write(color).unwrap();
    queue.write_buffer(&wgpu_buffer, 0, encase_buffer.as_ref());
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let instance = wgpu::Instance::default();

    let surface = instance.create_surface(&window).unwrap();
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

    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));

    let mut color_uniform = [1.0, 0.0, 0.0];
    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Color Uniform Buffer"),
        size: std::mem::size_of::<RGB>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    update_color_buffer(&queue, &color_uniform_buffer, &RGB::new(color_uniform));

    let color_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Color Bind Group Layout"),
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
        label: Some("Color Bind Group"),
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

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: Default::default(),
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut surface_config = surface
        .get_default_config(&adapter, size.width, size.height)
        .unwrap();
    surface.configure(&device, &surface_config);

    let mut egui_renderer = egui_wgpu::Renderer::new(&device, surface_config.format, None, 1);
    let egui_ctx = egui::Context::default();

    let window = &window;
    let egui_viewport_id = egui_ctx.viewport_id();
    let pixels_per_point = window.scale_factor() as f32;
    let mut egui_state = egui_winit::State::new(
        egui_ctx,
        egui_viewport_id,
        window,
        Some(pixels_per_point),
        None,
    );

    event_loop
        .run(move |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (&instance, &adapter, &shader, &pipeline_layout);

            if let Event::WindowEvent {
                window_id: _,
                event: window_event,
            } = event
            {
                let egui_event_response = egui_state.on_window_event(window, &window_event);

                if egui_event_response.repaint {
                    window.request_redraw();
                }

                if egui_event_response.consumed {
                    return;
                }

                match window_event {
                    WindowEvent::Resized(new_size) => {
                        // Reconfigure the surface with the new size
                        surface_config.width = new_size.width.max(1);
                        surface_config.height = new_size.height.max(1);
                        surface.configure(&device, &surface_config);
                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        let frame = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Command Encoder"),
                            });

                        let egui_raw_input = egui_state.take_egui_input(window);
                        let egui_full_output = egui_state.egui_ctx().run(
                            egui_raw_input,
                            |egui_ctx: &egui::Context| {
                                egui::CentralPanel::default()
                                    .frame(
                                        egui::Frame::none().inner_margin(egui::Margin::same(10.0)),
                                    )
                                    .show(egui_ctx, |ui| {
                                        if ui.color_edit_button_rgb(&mut color_uniform).changed() {
                                            update_color_buffer(
                                                &queue,
                                                &color_uniform_buffer,
                                                &RGB::new(color_uniform),
                                            );
                                        }
                                    });
                            },
                        );
                        let egui_primitives = egui_state
                            .egui_ctx()
                            .tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
                        let egui_screen_descriptor = egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [surface_config.width, surface_config.height],
                            pixels_per_point: pixels_per_point,
                        };

                        for (id, image_delta) in egui_full_output.textures_delta.set {
                            egui_renderer.update_texture(&device, &queue, id, &image_delta);
                        }

                        egui_renderer.update_buffers(
                            &device,
                            &queue,
                            &mut encoder,
                            &egui_primitives,
                            &egui_screen_descriptor,
                        );

                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                            rpass.set_pipeline(&render_pipeline);

                            rpass.set_bind_group(0, &color_bind_group, &[]);

                            rpass.draw(0..3, 0..1);

                            egui_renderer.render(
                                &mut rpass,
                                &egui_primitives,
                                &egui_screen_descriptor,
                            );
                        }

                        queue.submit(Some(encoder.finish()));
                        frame.present();

                        for id in egui_full_output.textures_delta.free {
                            egui_renderer.free_texture(&id);
                        }
                    }
                    WindowEvent::CloseRequested => target.exit(),
                    _ => {}
                };
            }
        })
        .unwrap();
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    #[allow(unused_mut)]
    let mut builder = winit::window::WindowBuilder::new();
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        builder = builder.with_canvas(Some(canvas));
    }
    let window = builder.build(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
