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
    lighting::{DirectionalLight, LightDirtyFlag},
    material::{DefaultMaterialEntity, Material},
    mesh::Mesh,
    time::Time,
    transform::{GlobalTransform, Transform},
    ui::UiState,
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
    pub input_schedule: ecs::Schedule,
    pub update_schedule: ecs::Schedule,
    pub render_schedule: ecs::Schedule,
    pub cleanup_schedule: ecs::Schedule,
    pub camera_entity: Entity,
    pub sun_light_entity: Entity,
    stat: EngineStatistics,
}

pub struct SelectedEntity(pub Option<Entity>);
impl ecs::Resource for SelectedEntity {}

#[derive(Default)]
pub struct RaytracerFrameState {
    pub frame_count: u32,
    pub frames_to_skip: u32,
    pub pending_skip_calculation: bool,
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
        world.insert_resource(Time {
            delta_time: 0.0,
            elapsed_time: 0.0,
        });
        world.insert_resource(StaticDataDirtyFlag(true)); // Initial update required
        world.insert_resource(SelectedEntity(None));
        world.insert_resource(RaytracerFrameState::default());

        let mut input_schedule = ecs::Schedule::new();
        input_schedule.add_system(crate::input::camera_controller::camera_controller_system);

        let mut update_schedule = ecs::Schedule::new();
        update_schedule.add_system(crate::systems::ui_system::ui_system);
        update_schedule.add_system(crate::transform::calculate_global_position_system);
        update_schedule.add_system(crate::rendering::renderer::renderer_update_system);

        let mut render_schedule = ecs::Schedule::new();
        render_schedule.add_system(crate::systems::probe_baking_system::probe_baking_system);
        render_schedule.add_system(crate::systems::render_system::render_system);

        let mut cleanup_schedule = ecs::Schedule::new();
        cleanup_schedule.add_system(crate::systems::scene_system::reset_dirty_flags_system);

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
        world.insert_resource(WindowResource(window.clone()));
        world.insert_resource(StaticDataDirtyFlag(true));
        world.insert_resource(LightDirtyFlag(true));
        world.insert_resource(SelectedEntity(None));
        world.insert_resource(RaytracerFrameState::default());
        world.insert_resource(EngineConfiguration::default());
        world.insert_resource(UiState::default());

        Self {
            window,
            window_size,
            world,
            input_schedule,
            update_schedule,
            render_schedule,
            cleanup_schedule,
            camera_entity,
            sun_light_entity,
            stat: EngineStatistics::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;
        if let Some(mut config) = self.world.get_resource_mut::<EngineConfiguration>() {
            config.reset_raytracer = true;
        }

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
        // Update delta time
        let current_time = Instant::now();
        self.stat.delta_time = current_time
            .duration_since(self.stat.last_frame_time)
            .as_secs_f32();
        self.stat.last_frame_time = current_time;

        // Update Time resource
        if let Some(mut time) = self.world.get_resource_mut::<Time>() {
            time.delta_time = self.stat.delta_time;
            time.elapsed_time += self.stat.delta_time;
        }

        // Run schedules
        self.input_schedule.run(&mut self.world);
        self.update_schedule.run(&mut self.world);
        self.render_schedule.run(&mut self.world);
        self.cleanup_schedule.run(&mut self.world);

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

        if camera_controller.is_cursor_locked()
            && camera_controller.has_camera_moved()
            && let Some(mut config) = self.world.get_resource_mut::<EngineConfiguration>()
        {
            config.reset_raytracer = true;
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            let mut camera_controller = self.world.get_resource_mut::<CameraController>().unwrap();
            camera_controller.process_mouse(delta.0, delta.1);

            if camera_controller.is_cursor_locked()
                && let Some(mut config) = self.world.get_resource_mut::<EngineConfiguration>()
            {
                config.reset_raytracer = true;
            }
        }
    }

    pub fn get_target_frame_time(&self) -> f32 {
        self.world
            .get_resource::<EngineConfiguration>()
            .unwrap()
            .target_frame_time
    }

    pub fn reset_raytracer(&mut self) {
        if let Some(mut config) = self.world.get_resource_mut::<EngineConfiguration>() {
            config.reset_raytracer = true;
        }
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
    pub reset_raytracer: bool,
}

impl ecs::Resource for EngineConfiguration {}

pub struct WindowResource(pub std::sync::Arc<winit::window::Window>);
impl ecs::Resource for WindowResource {}

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
