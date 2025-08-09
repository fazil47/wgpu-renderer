use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use winit::{
    event::{DeviceEvent, WindowEvent},
    window::Window,
};

use crate::{
    input::CameraController,
    lighting::DirectionalLight,
    mesh::Mesh as RenderMesh,
    rendering::{Camera, MaterialRef, Renderable, Renderer, Transform},
    scene::Scene,
};
use ecs::{EntityId, World};
use maths::Vec3;

#[cfg(not(target_arch = "wasm32"))]
use crate::mesh::gltf::GltfMeshExt;

pub struct Engine {
    // The window must be declared after the wgpu surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    pub window: Arc<Window>,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    pub camera_controller: CameraController,
    pub renderer: Renderer,
    pub world: World,
    pub scene: Scene,
    pub camera_entity: EntityId,
    pub sun_light_entity: EntityId,
    is_light_dirty: bool,
    stat: EngineStatistics,
    config: EngineConfiguration,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Engine {
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        let mut world = World::new();
        let mut scene = Scene::new();
        let camera_controller = CameraController::new(0.8);

        // Create camera entity
        let camera_entity = world.create_entity();
        let camera_position = Vec3::new(0.0, 0.0, 4.0);
        world.add_component(camera_entity, Transform::new(camera_position));
        world.add_component(
            camera_entity,
            Camera::new(
                camera_position,
                -camera_position.normalized(), // look at origin
                window_size.width as f32 / window_size.height as f32,
                45.0,
                0.1,
                100.0,
            ),
        );

        // Create sun light entity
        let sun_light_entity = world.create_entity();
        world.add_component(sun_light_entity, Transform::new(Vec3::ZERO));
        world.add_component(sun_light_entity, DirectionalLight::new(45.0, 45.0));

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(materials) = crate::mesh::Material::from_gltf("assets/cornell-box.glb") {
                // Load materials into Scene and create ECS entities
                scene.load_from_materials(materials, &mut world);
            } else {
                log::warn!("Failed to load GLTF mesh");
            }
        }

        let renderer = Renderer::new(
            window.clone(),
            &window_size,
            &scene,
            &world,
            camera_entity,
            sun_light_entity,
        )
        .await;

        Self {
            window,
            window_size,
            camera_controller,
            renderer,
            world,
            scene,
            camera_entity,
            sun_light_entity,
            is_light_dirty: false,
            stat: EngineStatistics::default(),
            config: EngineConfiguration::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;
        self.stat.frame_count = 0;

        // Update camera aspect ratio
        if let Some(camera) = self.world.get_component::<Camera>(self.camera_entity) {
            camera.borrow_mut().aspect = new_size.width as f32 / new_size.height as f32;
        }

        self.renderer.resize(new_size, &self.world);

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
            self.stat.frame_count += 1;
        }

        // Update delta time
        let current_time = Instant::now();
        self.stat.delta_time = current_time
            .duration_since(self.stat.last_frame_time)
            .as_secs_f32();
        self.stat.last_frame_time = current_time;

        // Extract values needed in closure to avoid borrow conflicts
        let delta_time_ms = self.stat.delta_time * 1000.0;
        let fps = 1.0 / self.stat.delta_time;
        let frame_count = self.stat.frame_count;
        let is_raytracer_enabled = self.config.is_raytracer_enabled;

        // Clone the Rc to avoid borrow conflicts in the closure
        let rasterizer = self.renderer.rasterizer.clone();

        // Store probe baking request and UI change outside the closure
        let mut bake_requested = false;
        let mut ui_changed = false;
        let mut raytracer_enabled = is_raytracer_enabled;

        let egui_output = self
            .renderer
            .setup_egui(&self.window, |egui_ctx: &egui::Context| {
                egui::SidePanel::right("fps_panel")
                    .exact_width(150.0)
                    .show_separator_line(false)
                    .resizable(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                    .show(egui_ctx, |ui| {
                        ui.label(format!("Frame Time: {delta_time_ms:.2}ms"));
                        ui.label(format!("FPS: {fps:.1}"));

                        if raytracer_enabled {
                            ui.label(format!("Frame Count: {frame_count}"));
                        }
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                    .show(egui_ctx, |ui| {
                        // Lighting controls
                        ui.collapsing("Lighting", |ui| {
                            if let Some(light) = self
                                .world
                                .get_component::<DirectionalLight>(self.sun_light_entity)
                            {
                                let mut light_ref = light.borrow_mut();

                                let sun_azi_changed = ui
                                    .add(
                                        egui::Slider::new(&mut light_ref.azimuth, 0.0..=360.0)
                                            .text("Sun Azimuth"),
                                    )
                                    .changed();
                                let sun_alt_changed = ui
                                    .add(
                                        egui::Slider::new(&mut light_ref.altitude, 0.0..=90.0)
                                            .text("Sun Altitude"),
                                    )
                                    .changed();

                                if sun_azi_changed || sun_alt_changed {
                                    light_ref.recalculate();
                                    self.is_light_dirty = true;
                                }
                            }
                        });

                        // Run probe UI and capture baking requests
                        let probe_ui_result = rasterizer.borrow_mut().run_probe_ui(ui);
                        if probe_ui_result.bake_requested {
                            bake_requested = true;
                        }

                        // Run the raytracer when the checkbox is toggled on
                        if ui.checkbox(&mut raytracer_enabled, "Raytracing").changed() {
                            ui_changed = true;
                        }
                    });
            });

        // Handle UI changes outside the closure
        if raytracer_enabled != self.config.is_raytracer_enabled {
            self.config.is_raytracer_enabled = raytracer_enabled;
        }

        // Light dirty handling is now done in the main UI above

        // Handle probe baking outside the closure
        if bake_requested {
            let rasterizer_borrowed = rasterizer.borrow();
            let material_bind_group = self.renderer.raytracer.get_material_bind_group();
            let mesh_bind_group = self.renderer.raytracer.get_mesh_bind_group();

            // Trigger probe baking
            rasterizer_borrowed.bake_probes(
                &self.renderer.wgpu.device,
                &self.renderer.wgpu.queue,
                material_bind_group,
                mesh_bind_group,
            );
            println!("Probe baking completed!");
        }

        self.renderer.render(
            &self.window,
            &self.window_size,
            &self.config,
            &mut self.world,
            self.camera_entity,
            self.sun_light_entity,
            self.stat.frame_count,
            egui_output,
        )?;
        // Set the light dirty flag to false after rendering
        self.is_light_dirty = false;

        Ok(())
    }

    pub fn process_egui_events(&mut self, event: &WindowEvent) -> egui_winit::EventResponse {
        self.renderer
            .egui
            .state
            .on_window_event(&self.window, event)
    }

    pub fn process_events(&mut self, event: &WindowEvent) {
        self.camera_controller.process_events(event);

        if self.camera_controller.is_cursor_locked() {
            self.stat.frame_count = 0;
            self.camera_controller.update_camera(
                &mut self.world,
                self.camera_entity,
                self.stat.delta_time,
            );
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.camera_controller.process_mouse(delta.0, delta.1);

            if self.camera_controller.is_cursor_locked() {
                self.stat.frame_count = 0;
                self.camera_controller.update_camera(
                    &mut self.world,
                    self.camera_entity,
                    self.stat.delta_time,
                );
            }
        }
    }

    pub fn is_light_dirty(&self) -> bool {
        self.is_light_dirty
    }

    pub fn set_light_clean(&mut self) {
        self.is_light_dirty = false;
    }

    /// Get all renderable entities (entities with Transform, RenderMesh, MaterialRef, Renderable)
    pub fn get_renderable_entities(&self) -> Vec<EntityId> {
        self.world
            .get_entities_with_3::<Transform, RenderMesh, MaterialRef>()
            .into_iter()
            .filter(|&entity_id| self.world.has_component::<Renderable>(entity_id))
            .collect()
    }
}

pub struct EngineStatistics {
    pub last_frame_time: Instant,
    pub delta_time: f32,
    pub frame_count: u32,
}

impl Default for EngineStatistics {
    fn default() -> Self {
        Self {
            last_frame_time: Instant::now(),
            delta_time: 0.0,
            frame_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct EngineConfiguration {
    pub target_frame_time: f32,
    pub raytracer_max_frames: u32,
    pub is_raytracer_enabled: bool,
}

impl Default for EngineConfiguration {
    fn default() -> Self {
        Self {
            target_frame_time: 1.0 / 120.0,
            raytracer_max_frames: 256,
            is_raytracer_enabled: false,
        }
    }
}
