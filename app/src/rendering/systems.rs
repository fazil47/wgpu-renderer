use ecs::World;

use crate::core::engine::{EngineConfiguration, RaytracerFrameState, WindowResource};
use crate::ui::UiState;
use crate::ui::egui::RendererEgui;
use crate::{
    camera::Camera,
    lighting::DirectionalLight,
    rendering::{
        mesh::MeshBuffers,
        rasterizer::Rasterizer,
        raytracer::Raytracer,
        wgpu::{RenderTarget, WgpuResources},
    },
};

pub fn render_system(world: &mut World) {
    // 1. Get resources
    let wgpu = world.get_resource::<WgpuResources>().unwrap();
    let (
        raytracer_enabled,
        raytracer_show_bvh,
        target_frame_time,
        raytracer_max_frames,
        debug_shadow_maps,
        shadow_map_cascade_to_debug,
    ) = {
        let config = world.get_resource::<EngineConfiguration>().unwrap();
        (
            config.is_raytracer_enabled,
            config.show_bvh,
            config.target_frame_time,
            config.raytracer_max_frames,
            config.debug_shadow_maps,
            config.shadow_map_cascade_to_debug,
        )
    };

    // 2. Prepare for rendering
    let (surface_texture, render_target_view) = match &wgpu.target {
        RenderTarget::Surface { surface, .. } => {
            let surface_texture = match surface.get_current_texture() {
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
            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            (Some(surface_texture), view)
        }
        RenderTarget::Offscreen { texture, .. } => {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (None, view)
        }
    };

    let mut render_encoder = wgpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    // Check events
    let light_dirty = world.has_events::<crate::core::events::LightsChanged>();
    let reset_raytracer = world.has_events::<crate::core::events::RaytracerReset>();

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

        // TODO: Check camera dirty (CSM depends on camera)
        rasterizer.update_light(&wgpu.queue, world, sun_light_entity);

        rasterizer.update_probes(&wgpu.device, &wgpu.queue);

        rasterizer.update_debug_config(
            &wgpu.device,
            debug_shadow_maps,
            shadow_map_cascade_to_debug,
        );
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

        // If we computed last frame, calculate how many frames to skip based on how long it took
        if frame_state.pending_skip_calculation {
            let delta_time = world
                .get_resource::<crate::time::Time>()
                .unwrap()
                .delta_time;

            // Calculate frames to skip: (compute_time / target_frame_time)
            // We subtract 1 because the current frame is already one frame after the compute
            let skips = (delta_time / target_frame_time).floor() as u32;
            frame_state.frames_to_skip = skips.saturating_sub(1);
            frame_state.pending_skip_calculation = false;
        }

        if reset_raytracer {
            frame_state.frame_count = 0;
            frame_state.frames_to_skip = 0;
            frame_state.pending_skip_calculation = false;
        }

        // Render if enough frames have been skipped to reach target frame rate
        if frame_state.frames_to_skip > 0 {
            frame_state.frames_to_skip -= 1;
        } else {
            // Check if we should compute
            let should_compute = frame_state.frame_count < raytracer_max_frames;

            if raytracer_enabled && should_compute {
                // We need mutable access to Raytracer
                if let Some(raytracer) = world.get_resource::<Raytracer>() {
                    raytracer.update_frame_count(&wgpu.queue, frame_state.frame_count);

                    // Dispatch compute
                    let window_size =
                        winit::dpi::PhysicalSize::new(wgpu.target.width(), wgpu.target.height());

                    raytracer.compute(&window_size, &wgpu.device, &wgpu.queue);

                    frame_state.frame_count += 1;
                    frame_state.pending_skip_calculation = true;
                }
            }
        }
    }

    // 4. Render Pass (Raster or Raytrace)
    {
        if raytracer_enabled {
            let raytracer = world.get_resource::<Raytracer>().unwrap();
            raytracer.render(&mut render_encoder, &render_target_view, raytracer_show_bvh);
        } else {
            // Rasterizer pass
            if let Some(rasterizer) = world.get_resource::<Rasterizer>()
                && let Some(mesh_buffers) = world.get_resource::<MeshBuffers>()
            {
                rasterizer.render(&mut render_encoder, &render_target_view, &mesh_buffers);

                if rasterizer.should_render_probe_visualization() {
                    rasterizer.render_probe_visualization(&mut render_encoder, &render_target_view);
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
            size_in_pixels: [wgpu.target.width(), wgpu.target.height()],
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
            &render_target_view,
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
    if let Some(surface_texture) = surface_texture {
        surface_texture.present();
    }
}

pub fn update_system(world: &mut World) {
    let mesh_added = world.has_events::<crate::core::events::MeshAdded>();
    let mesh_removed = world.has_events::<crate::core::events::MeshRemoved>();
    let transform_changed = world.has_events::<crate::core::events::GlobalTransformChanged>();
    let mesh_changed = mesh_added || mesh_removed;

    if !mesh_changed && !transform_changed {
        return;
    }

    // Process MeshAdded events — append each new mesh to the GPU buffers
    if let Some(added) = world.read_events::<crate::core::events::MeshAdded>() {
        let entities: Vec<_> = added.iter().map(|e| e.0).collect();
        let wgpu = world.get_resource::<WgpuResources>().unwrap();
        if let Some(mut mesh_buffers) = world.get_resource_mut::<MeshBuffers>() {
            for entity in entities {
                if let Err(err) = mesh_buffers.add_mesh(&wgpu.device, &wgpu.queue, entity, world) {
                    log::error!("MeshBuffers add_mesh failed for {entity:?}: {err}");
                }
            }
        }
    }

    // Process MeshRemoved events — mark each mesh slot as empty, collect
    // returned mesh offsets for BLAS removal
    let removed_mesh_offsets: Vec<usize> = world
        .read_events::<crate::core::events::MeshRemoved>()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            world
                .get_resource_mut::<MeshBuffers>()
                .and_then(|mut mb| mb.remove_mesh(e.0))
        })
        .collect();

    // Patch instance transforms for changed entities
    if transform_changed {
        let wgpu = world.get_resource::<WgpuResources>().unwrap();
        if let Some(mut mesh_buffers) = world.get_resource_mut::<MeshBuffers>()
            && let Err(err) = mesh_buffers.update_transforms(&wgpu.queue, world)
        {
            log::error!("MeshBuffers transform update failed: {}", err);
        }
    }

    // Incrementally update BLAS for added/removed meshes
    if mesh_changed {
        let mesh_buffers = world.get_resource::<MeshBuffers>().unwrap();
        let mut blas = world
            .get_resource_mut::<crate::rendering::BlasBvh>()
            .unwrap();

        // New meshes are always appended, so process the new tail entries
        let old_meshes_count = blas.infos.len();
        let new_meshes_count = mesh_buffers.meshes.len();
        for mesh_offset in old_meshes_count..new_meshes_count {
            blas.add_mesh(mesh_offset, &mesh_buffers);
        }

        for offset in removed_mesh_offsets {
            blas.remove_mesh(offset);
        }
    }

    // Always rebuild TLAS when transforms or geometry changed (world-space bounds)
    let tlas = {
        let mesh_buffers = world.get_resource::<MeshBuffers>().unwrap();
        let blas = world.get_resource::<crate::rendering::BlasBvh>().unwrap();
        crate::rendering::build_scene_tlas(&mesh_buffers, &blas)
    };
    world.insert_resource(tlas);

    // TODO: Should be querying for PrimaryCamera instead of just picking the
    // first entity with Camera
    let camera_entity = world
        .get_entities_with::<Camera>()
        .into_iter()
        .next()
        .expect("No camera entity found");

    // TODO: Support more than one directional lights
    let sun_light_entity = world
        .get_entities_with::<DirectionalLight>()
        .into_iter()
        .next()
        .expect("No sun light entity found");

    let wgpu = world.get_resource::<WgpuResources>().unwrap();

    if mesh_changed
        && let Some(mut rasterizer) = world.get_resource_mut::<Rasterizer>()
        && let Err(err) = rasterizer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        )
    {
        log::error!("Rasterizer update failed: {}", err);
    }

    if let Some(mut raytracer) = world.get_resource_mut::<Raytracer>()
        && let Err(err) = raytracer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            world,
            camera_entity,
            sun_light_entity,
        )
    {
        log::error!("Raytracer update failed: {}", err);
    }
}
