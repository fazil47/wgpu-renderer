use app::{
    egui::initialize_egui,
    utils::load_icon,
    wgpu::{initialize_shader, initialize_wgpu, update_color_buffer, RGBA},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut window_size = window.inner_size();
    window_size.width = window_size.width.max(1);
    window_size.height = window_size.height.max(1);
    let pixels_per_point = window.scale_factor() as f32;

    let (instance, surface, adapter, device, queue, mut surface_config) =
        initialize_wgpu(&window, &window_size).await;

    let (
        shader,
        mut color_uniform,
        color_uniform_buffer,
        color_bind_group,
        pipeline_layout,
        render_pipeline,
    ) = initialize_shader(&device, &queue, &surface, &adapter);

    let (mut egui_renderer, mut egui_state) =
        initialize_egui(&window, &device, &surface_config, pixels_per_point);

    let window = &window;

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
                        let frame = surface.get_current_texture();

                        if frame.is_err() {
                            return;
                        }

                        let frame = frame.unwrap();
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
                                        if ui
                                            .color_edit_button_rgba_unmultiplied(&mut color_uniform)
                                            .changed()
                                        {
                                            update_color_buffer(
                                                &queue,
                                                &color_uniform_buffer,
                                                &RGBA::new(color_uniform),
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let icon = load_icon(std::path::Path::new("assets/icon.png"));
        builder = builder.with_window_icon(Some(icon));
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
