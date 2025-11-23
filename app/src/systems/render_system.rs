use crate::core::engine::{EngineConfiguration, RaytracerFrameState, WindowResource};
use crate::rendering::rasterizer::Rasterizer;
use crate::rendering::raytracer::Raytracer;
use crate::rendering::wgpu::WgpuResources;
use crate::ui::UiState;
use crate::ui::egui::RendererEgui;
use ecs::World;

pub fn render_system(world: &mut World) {
    // 1. Get resources
    let wgpu = world.get_resource::<WgpuResources>().unwrap();
    let (
        raytracer_enabled,
        raytracer_show_bvh,
        reset_raytracer,
        target_frame_time,
        raytracer_max_frames,
    ) = {
        let config = world.get_resource::<EngineConfiguration>().unwrap();
        (
            config.is_raytracer_enabled,
            config.show_bvh,
            config.reset_raytracer,
            config.target_frame_time,
            config.raytracer_max_frames,
        )
    };

    // 2. Prepare for rendering
    let surface_texture = match wgpu.surface.get_current_texture() {
        Ok(texture) => texture,
        Err(wgpu::SurfaceError::Outdated) => return,
        Err(wgpu::SurfaceError::Timeout) => return,
        Err(wgpu::SurfaceError::Lost) => return,
        Err(wgpu::SurfaceError::OutOfMemory) => {
            panic!("Out of memory");
        }
        Err(wgpu::SurfaceError::Other) => {
            panic!("Other surface error");
        }
    };

    let surface_texture_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut render_encoder = wgpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    // Check LightDirtyFlag
    let light_dirty = world
        .get_resource::<crate::lighting::LightDirtyFlag>()
        .map(|f| f.0)
        .unwrap_or(false);

    // We need camera and light entities for updates
    let camera_entity = world
        .get_entities_with::<crate::camera::Camera>()
        .first()
        .copied()
        .expect("No camera");
    let sun_light_entity = world
        .get_entities_with::<crate::lighting::DirectionalLight>()
        .first()
        .copied()
        .expect("No sun light");

    // Update Rasterizer (Camera, Light, Probes)
    if let Some(mut rasterizer) = world.get_resource_mut::<Rasterizer>() {
        rasterizer.update_camera(&wgpu.queue, world, camera_entity);

        if light_dirty {
            rasterizer.update_light(&wgpu.queue, world, sun_light_entity);
        }

        rasterizer.update_probes(&wgpu.device, &wgpu.queue);
    }

    if light_dirty {
        // Update lights in Raytracer
        if let Some(raytracer) = world.get_resource::<Raytracer>() {
            raytracer.update_light(&wgpu.queue, world, sun_light_entity);
        }
    }

    // Update Raytracer Camera
    if let Some(raytracer) = world.get_resource::<Raytracer>() {
        raytracer.update_camera(&wgpu.queue, world, camera_entity);
    }

    // 3. Raytracer Compute Pass
    {
        // We need mutable access to RaytracerFrameState
        let mut frame_state = world.get_resource_mut::<RaytracerFrameState>().unwrap();
        let delta_time = world
            .get_resource::<crate::time::Time>()
            .unwrap()
            .delta_time;

        frame_state.accumulator += delta_time;

        let should_compute = reset_raytracer || frame_state.accumulator >= target_frame_time;

        if raytracer_enabled
            && (should_compute
                || (frame_state.frame_count < raytracer_max_frames
                    && frame_state.frames_till_next_compute == 0))
        {
            if should_compute {
                frame_state.accumulator = 0.0;
            }

            // We need mutable access to Raytracer
            if let Some(raytracer) = world.get_resource::<Raytracer>() {
                raytracer.update_frame_count(&wgpu.queue, frame_state.frame_count);

                // Dispatch compute
                let window_size = winit::dpi::PhysicalSize::new(
                    wgpu.surface_config.width,
                    wgpu.surface_config.height,
                );

                raytracer.compute(&window_size, &wgpu.device, &wgpu.queue);

                frame_state.frame_count += 1;
            }
        }

        if reset_raytracer {
            frame_state.frame_count = 0;
            frame_state.frames_till_next_compute = 0;
        }
    }

    // 4. Render Pass (Raster or Raytrace)
    {
        if raytracer_enabled {
            let raytracer = world.get_resource::<Raytracer>().unwrap();
            raytracer.render(
                &mut render_encoder,
                &surface_texture_view,
                raytracer_show_bvh,
            );
        } else {
            // Rasterizer pass
            if let Some(rasterizer) = world.get_resource::<Rasterizer>() {
                // We need default material entity?
                // Let's query for the first entity with Material component.

                let material_entities = world.get_entities_with::<crate::material::Material>();
                let default_material_entity = material_entities.first().copied().unwrap_or(
                    // Fallback if no materials? Should not happen in this scene.
                    // If it happens, we might panic.
                    world.get_all_entities().first().copied().unwrap(),
                );

                rasterizer.render(
                    &mut render_encoder,
                    &surface_texture_view,
                    default_material_entity,
                );

                if rasterizer.should_render_probe_visualization() {
                    rasterizer
                        .render_probe_visualization(&mut render_encoder, &surface_texture_view);
                }
            }
        }
    }

    // 5. UI Render Pass
    // We need to take the output from UiState
    let egui_output = {
        let mut ui_state_mut = world.get_resource_mut::<UiState>().unwrap();
        ui_state_mut.egui_output.take()
    };

    if let Some(output) = egui_output {
        let mut egui = world.get_resource_mut::<RendererEgui>().unwrap();
        let window_resource = world.get_resource::<WindowResource>().unwrap();

        // Tessellate
        let egui_primitives = egui
            .state
            .egui_ctx()
            .tessellate(output.shapes, output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [wgpu.surface_config.width, wgpu.surface_config.height],
            pixels_per_point: window_resource.0.scale_factor() as f32,
        };

        // Update textures
        for (id, image_delta) in output.textures_delta.set {
            egui.renderer
                .update_texture(&wgpu.device, &wgpu.queue, id, &image_delta);
        }

        egui.render(
            &wgpu.device,
            &wgpu.queue,
            &mut render_encoder,
            &surface_texture_view,
            &egui_primitives,
            &screen_descriptor,
        );

        // Free textures
        for id in output.textures_delta.free {
            egui.renderer.free_texture(&id);
        }
    }

    // 6. Submit and Present
    wgpu.queue.submit(std::iter::once(render_encoder.finish()));
    surface_texture.present();
}
