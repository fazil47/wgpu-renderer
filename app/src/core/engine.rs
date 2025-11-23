use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use winit::{
    event::{DeviceEvent, WindowEvent},
    window::Window,
};

use crate::{
    camera::Camera,
    input::CameraController,
    lighting::DirectionalLight,
    material::{DefaultMaterialEntity, Material},
    mesh::Mesh,
    time::Time,
    transform::{GlobalTransform, Transform},
    ui::{build_mesh_hierarchy, draw_mesh_hierarchy},
};
use ecs::{Entity, World};
use maths::Vec3;

use crate::mesh::gltf::GltfMeshExt;

pub struct Engine {
    // The window must be declared after the wgpu surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    pub window: Arc<Window>,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub world: World,
    pub static_schedule: ecs::Schedule, // Run only when data changes
    pub frame_schedule: ecs::Schedule,  // Run every frame
    pub camera_entity: Entity,
    pub sun_light_entity: Entity,
    is_light_dirty: bool,
    stat: EngineStatistics,
    config: EngineConfiguration,
}

pub struct SelectedEntity(pub Option<Entity>);
impl ecs::Resource for SelectedEntity {}

#[derive(Default)]
pub struct RaytracerFrameState {
    pub frame_count: u32,
    pub frames_till_next_compute: u32,
}

impl ecs::Resource for RaytracerFrameState {}

pub struct StaticDataDirtyFlag(pub bool);

impl ecs::Resource for StaticDataDirtyFlag {}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Engine {
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        let mut world = World::new();

        let default_material_entity = world.create_entity();
        world.add_component(default_material_entity, Material::default());
        world.insert_resource(DefaultMaterialEntity(default_material_entity));

        let camera_entity = world.create_entity();
        let camera_position = Vec3::new(0.0, 0.0, 4.0);
        let camera_controller = CameraController::new(0.8);
        let camera_transform = Transform::new(camera_position);
        world.add_component(camera_entity, camera_transform);
        world.add_component(
            camera_entity,
            GlobalTransform::from_transform(&camera_transform),
        );
        world.add_component(
            camera_entity,
            Camera::new(
                camera_position,
                -camera_position.normalized(), // look at origin
                window_size.width as f32 / window_size.height as f32,
                45.0,
                0.01,
                10000.0,
            ),
        );

        // Create sun light entity
        let sun_light_entity = world.create_entity();
        let sun_transform = Transform::new(Vec3::ZERO);
        world.add_component(sun_light_entity, sun_transform);
        world.add_component(
            sun_light_entity,
            GlobalTransform::from_transform(&sun_transform),
        );
        world.add_component(sun_light_entity, DirectionalLight::new(45.0, 45.0));

        let gltf_path = "assets/cornell-box.glb";

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(meshes) = Mesh::from_gltf(&mut world, gltf_path) {
                println!("Loaded {} meshes from GLTF", meshes.len());
            } else {
                log::warn!("Failed to load GLTF mesh");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(meshes) = Mesh::from_gltf_url(&mut world, gltf_path).await {
                println!("Loaded {} meshes from GLTF", meshes.len());
            } else {
                log::warn!("Failed to load GLTF mesh");
            }
        }

        world.insert_resource(camera_controller);
        world.insert_resource(Time { delta_time: 0.0 });
        world.insert_resource(StaticDataDirtyFlag(true)); // Initial update required
        world.insert_resource(SelectedEntity(None));
        world.insert_resource(RaytracerFrameState::default());

        let mut static_schedule = ecs::Schedule::new();
        static_schedule.add_system(crate::transform::transform_system);
        static_schedule.add_system(crate::rendering::renderer::renderer_update_system);

        let mut frame_schedule = ecs::Schedule::new();
        frame_schedule.add_system(crate::input::camera_controller::camera_controller_system);

        // Create rendering resources separately
        let wgpu = crate::rendering::wgpu::WgpuResources::new(window.clone(), &window_size).await;
        let egui = crate::ui::egui::RendererEgui::new(
            &window,
            &wgpu.device,
            &wgpu.surface_config,
            window.scale_factor() as f32,
        );

        let mut rasterizer = crate::rendering::rasterizer::Rasterizer::new(&wgpu);
        if let Err(err) = rasterizer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            &world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Failed to update rasterizer render data: {err}");
        }

        let mut raytracer = crate::rendering::raytracer::Raytracer::new(&wgpu, &window_size);
        if let Err(err) = raytracer.update_render_data(
            &wgpu.device,
            &wgpu.queue,
            &world,
            camera_entity,
            sun_light_entity,
        ) {
            eprintln!("Failed to update raytracer render data: {err}");
        }

        world.insert_resource(wgpu);
        world.insert_resource(egui);
        world.insert_resource(rasterizer);
        world.insert_resource(raytracer);

        Self {
            window,
            window_size,
            world,
            static_schedule,
            frame_schedule,
            camera_entity,
            sun_light_entity,
            is_light_dirty: false,
            stat: EngineStatistics::default(),
            config: EngineConfiguration::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;
        self.config.reset_raytracer = true;

        // Update camera aspect ratio
        if let Some(mut camera) = self.world.get_component_mut::<Camera>(self.camera_entity) {
            camera.aspect = new_size.width as f32 / new_size.height as f32;
        }

        let mut wgpu = self
            .world
            .get_resource_mut::<crate::rendering::wgpu::WgpuResources>()
            .unwrap();
        wgpu.resize(&new_size);

        let mut rasterizer = self
            .world
            .get_resource_mut::<crate::rendering::rasterizer::Rasterizer>()
            .unwrap();
        rasterizer.resize(&wgpu);

        let mut raytracer = self
            .world
            .get_resource_mut::<crate::rendering::raytracer::Raytracer>()
            .unwrap();
        raytracer.resize(&new_size, &wgpu);

        // On macOS the window needs to be redrawn manually after resizing
        #[cfg(target_os = "macos")]
        {
            self.window.request_redraw();
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        use crate::rendering::rasterizer::Rasterizer;
        use crate::rendering::raytracer::Raytracer;
        use crate::rendering::wgpu::WgpuResources;
        use crate::ui::egui::RendererEgui;

        // Update delta time
        let current_time = Instant::now();
        self.stat.delta_time = current_time
            .duration_since(self.stat.last_frame_time)
            .as_secs_f32();
        self.stat.last_frame_time = current_time;

        // Update Time resource
        if let Some(mut time) = self.world.get_resource_mut::<Time>() {
            time.delta_time = self.stat.delta_time;
        }

        // Run update schedule (inputs, camera, etc.)
        self.frame_schedule.run(&mut self.world);

        // Extract values for UI
        let delta_time_ms = self.stat.delta_time * 1000.0;
        let fps = 1.0 / self.stat.delta_time;
        let mut raytracer_enabled = self.config.is_raytracer_enabled;
        let mut raytracer_show_bvh = self.config.show_bvh;
        let mut reset_raytracer = self.config.reset_raytracer;
        let mut bake_requested = false;
        let mut has_transform_changed = false;

        let mesh_hierarchy = build_mesh_hierarchy(&self.world);

        // Get frame count
        let frame_count = self
            .world
            .get_resource::<RaytracerFrameState>()
            .map(|s| s.frame_count)
            .unwrap_or(0);

        // Setup and run egui
        let egui_output = {
            let mut egui = self.world.get_resource_mut::<RendererEgui>().unwrap();
            let mut selected = self.world.get_resource_mut::<SelectedEntity>().unwrap();
            let mut rasterizer = self.world.get_resource_mut::<Rasterizer>().unwrap();

            let egui_raw_input = egui.state.take_egui_input(&self.window);
            let egui_ctx = egui.state.egui_ctx().clone();

            egui_ctx.run(egui_raw_input, |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        if let Some(entity) = selected.0 {
                            has_transform_changed = egui.select_entity(&self.world, ui, entity);
                        }

                        egui::SidePanel::right("fps_panel")
                            .exact_width(150.0)
                            .show_separator_line(false)
                            .resizable(false)
                            .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                            .show(ctx, |ui| {
                                ui.label(format!("Frame Time: {delta_time_ms:.2}ms"));
                                ui.label(format!("FPS: {fps:.1}"));

                                if raytracer_enabled {
                                    ui.label(format!("Frame Count: {frame_count}"));
                                }
                            });

                        egui::CentralPanel::default()
                            .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                            .show(ctx, |ui| {
                                ui.collapsing("Meshes", |ui| {
                                    draw_mesh_hierarchy(ui, &mesh_hierarchy, &mut selected.0);
                                });

                                // Lighting controls
                                ui.collapsing("Lighting", |ui| {
                                    if let Some(mut light) =
                                        self.world.get_component_mut::<DirectionalLight>(
                                            self.sun_light_entity,
                                        )
                                    {
                                        let sun_azi_changed = ui
                                            .add(
                                                egui::Slider::new(&mut light.azimuth, 0.0..=360.0)
                                                    .text("Sun Azimuth"),
                                            )
                                            .changed();
                                        let sun_alt_changed = ui
                                            .add(
                                                egui::Slider::new(&mut light.altitude, 0.0..=90.0)
                                                    .text("Sun Altitude"),
                                            )
                                            .changed();

                                        if sun_azi_changed || sun_alt_changed {
                                            light.recalculate();
                                            self.is_light_dirty = true;
                                            reset_raytracer = true;
                                        }
                                    }
                                });

                                // Run probe UI
                                let probe_ui_result = rasterizer.run_probe_ui(ui);
                                if probe_ui_result.bake_requested {
                                    bake_requested = true;
                                }

                                ui.collapsing("Raytracing", |ui| {
                                    ui.checkbox(&mut raytracer_enabled, "Enabled");
                                    ui.checkbox(&mut raytracer_show_bvh, "Show BVH");
                                });
                            });
                    });
            })
        };

        // Handle transform changes
        if has_transform_changed {
            reset_raytracer = true;
            if let Some(mut flag) = self.world.get_resource_mut::<StaticDataDirtyFlag>() {
                flag.0 = true;
            }
        }

        // Run static schedule if needed
        let run_static = self
            .world
            .get_resource::<StaticDataDirtyFlag>()
            .map(|f| f.0)
            .unwrap_or(false);

        if run_static {
            self.static_schedule.run(&mut self.world);
            if let Some(mut flag) = self.world.get_resource_mut::<StaticDataDirtyFlag>() {
                flag.0 = false;
            }
        }

        // Update config
        if raytracer_enabled != self.config.is_raytracer_enabled {
            self.config.is_raytracer_enabled = raytracer_enabled;
        }
        if raytracer_show_bvh != self.config.show_bvh {
            self.config.show_bvh = raytracer_show_bvh;
        }

        // Handle probe baking
        if bake_requested {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let rasterizer = self.world.get_resource::<Rasterizer>().unwrap();
            let raytracer = self.world.get_resource::<Raytracer>().unwrap();
            let material_bind_group = raytracer.get_material_bind_group();
            let mesh_bind_group = raytracer.get_mesh_bind_group();
            rasterizer.bake_probes(
                &wgpu.device,
                &wgpu.queue,
                material_bind_group,
                mesh_bind_group,
            );
        }

        // === RENDERING PHASE ===
        let start_of_render = Instant::now();

        // Update camera and lights
        {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let mut egui = self.world.get_resource_mut::<RendererEgui>().unwrap();
            egui.update_camera(&self.world, self.camera_entity);
            drop(egui);

            let rasterizer = self.world.get_resource::<Rasterizer>().unwrap();
            rasterizer.update_camera(&wgpu.queue, &self.world, self.camera_entity);
            rasterizer.update_light(&wgpu.queue, &self.world, self.sun_light_entity);
            drop(rasterizer);

            let raytracer = self.world.get_resource::<Raytracer>().unwrap();
            raytracer.update_camera(&wgpu.queue, &self.world, self.camera_entity);
            raytracer.update_light(&wgpu.queue, &self.world, self.sun_light_entity);
        }

        // Check if probe grid configuration changed
        {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let mut rasterizer = self.world.get_resource_mut::<Rasterizer>().unwrap();
            if rasterizer.is_probe_dirty() {
                rasterizer.update_probes(&wgpu.device, &wgpu.queue);
                rasterizer.clear_probe_dirty();
            }
        }

        // Raytracer compute
        {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let raytracer = self.world.get_resource::<Raytracer>().unwrap();
            let mut frame_state = self
                .world
                .get_resource_mut::<RaytracerFrameState>()
                .unwrap();

            if self.config.is_raytracer_enabled
                && (reset_raytracer
                    || (frame_state.frame_count < self.config.raytracer_max_frames
                        && frame_state.frames_till_next_compute == 0))
            {
                raytracer.update_frame_count(&wgpu.queue, frame_state.frame_count);
                raytracer.compute(&self.window_size, &wgpu.device, &wgpu.queue);

                if reset_raytracer {
                    frame_state.frame_count = 0;
                } else {
                    frame_state.frame_count += 1;
                }
            } else if reset_raytracer {
                frame_state.frame_count = 0;
            }
        }

        // Get surface and create encoder
        let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
        let surface_texture = wgpu.surface.get_current_texture()?;
        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder =
            wgpu.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Command Encoder"),
                });
        drop(wgpu);

        // Tessellate egui
        let egui_primitives = {
            let egui = self.world.get_resource::<RendererEgui>().unwrap();
            egui.state
                .egui_ctx()
                .tessellate(egui_output.shapes, egui_output.pixels_per_point)
        };

        let egui_screen_descriptor = {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [wgpu.surface_config.width, wgpu.surface_config.height],
                pixels_per_point: self.window.scale_factor() as f32,
            }
        };

        // Update egui textures
        {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let mut egui = self.world.get_resource_mut::<RendererEgui>().unwrap();
            for (id, image_delta) in egui_output.textures_delta.set {
                egui.renderer
                    .update_texture(&wgpu.device, &wgpu.queue, id, &image_delta);
            }
        }

        // Render scene
        {
            if self.config.is_raytracer_enabled {
                let raytracer = self.world.get_resource::<Raytracer>().unwrap();
                raytracer.render(
                    &mut render_encoder,
                    &surface_texture_view,
                    self.config.show_bvh,
                );
            } else {
                let default_material_entity = self
                    .world
                    .get_resource::<DefaultMaterialEntity>()
                    .unwrap()
                    .0;
                let rasterizer = self.world.get_resource::<Rasterizer>().unwrap();
                rasterizer.render(
                    &mut render_encoder,
                    &surface_texture_view,
                    default_material_entity,
                );

                // Render probe visualization if enabled
                if rasterizer.should_render_probe_visualization() {
                    rasterizer
                        .render_probe_visualization(&mut render_encoder, &surface_texture_view);
                }
            }

            // Render egui
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            let mut egui = self.world.get_resource_mut::<RendererEgui>().unwrap();
            egui.render(
                &wgpu.device,
                &wgpu.queue,
                &mut render_encoder,
                &surface_texture_view,
                &egui_primitives,
                &egui_screen_descriptor,
            );
        }

        // Submit and present
        {
            let wgpu = self.world.get_resource::<WgpuResources>().unwrap();
            wgpu.queue.submit(Some(render_encoder.finish()));
        }
        surface_texture.present();

        // Free egui textures
        {
            let mut egui = self.world.get_resource_mut::<RendererEgui>().unwrap();
            for id in egui_output.textures_delta.free {
                egui.renderer.free_texture(&id);
            }
        }

        // Update frame throttling for raytracer
        if self.config.is_raytracer_enabled {
            const FRAMES_TO_WAIT_THRESHOLD: u32 = 2;
            let frames_to_wait = (start_of_render.elapsed().as_secs_f32()
                / self.config.target_frame_time)
                .ceil() as u32;

            let mut frame_state = self
                .world
                .get_resource_mut::<RaytracerFrameState>()
                .unwrap();
            if frames_to_wait > FRAMES_TO_WAIT_THRESHOLD {
                frame_state.frames_till_next_compute = frames_to_wait;
            } else if frame_state.frames_till_next_compute > 0 {
                frame_state.frames_till_next_compute -= 1;
            }
        } else {
            let mut frame_state = self
                .world
                .get_resource_mut::<RaytracerFrameState>()
                .unwrap();
            frame_state.frames_till_next_compute = 0;
        }

        // Reset flags
        self.is_light_dirty = false;
        self.config.reset_raytracer = false;

        Ok(())
    }

    pub fn process_egui_events(
        &mut self,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        let mut egui = self
            .world
            .get_resource_mut::<crate::ui::egui::RendererEgui>()
            .unwrap();
        egui.state.on_window_event(&self.window, event)
    }

    pub fn process_events(&mut self, event: &WindowEvent) {
        let mut camera_controller = self.world.get_resource_mut::<CameraController>().unwrap();
        camera_controller.process_events(event);

        if camera_controller.is_cursor_locked() && camera_controller.has_camera_moved() {
            self.config.reset_raytracer = true;
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            let mut camera_controller = self.world.get_resource_mut::<CameraController>().unwrap();
            camera_controller.process_mouse(delta.0, delta.1);

            if camera_controller.is_cursor_locked() {
                self.config.reset_raytracer = true;
            }
        }
    }

    pub fn is_light_dirty(&self) -> bool {
        self.is_light_dirty
    }

    pub fn set_light_clean(&mut self) {
        self.is_light_dirty = false;
    }
}

pub struct EngineStatistics {
    pub last_frame_time: Instant,
    pub delta_time: f32,
}

impl Default for EngineStatistics {
    fn default() -> Self {
        Self {
            last_frame_time: Instant::now(),
            delta_time: 0.0,
        }
    }
}

pub struct EngineConfiguration {
    pub target_frame_time: f32, // in seconds
    pub raytracer_max_frames: u32,
    pub is_raytracer_enabled: bool,
    pub show_bvh: bool,
    reset_raytracer: bool,
}

impl Default for EngineConfiguration {
    fn default() -> Self {
        Self {
            target_frame_time: 1.0 / 60.0,
            raytracer_max_frames: 256,
            is_raytracer_enabled: false,
            show_bvh: false,
            reset_raytracer: false,
        }
    }
}
