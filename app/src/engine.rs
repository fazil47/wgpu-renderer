use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use winit::{event::WindowEvent, window::Window};

use crate::{camera::CameraController, renderer::Renderer, scene::Scene};

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
    scene: Scene,
    stat: EngineStatistics,
    config: EngineConfiguration,
}

impl Engine {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: Arc<Window>) -> Engine {
        let mut window_size = window.inner_size();
        window_size.width = window_size.width.max(1);
        window_size.height = window_size.height.max(1);

        #[allow(unused_mut)]
        let mut scene = Scene::new(&window_size);
        let camera_controller = CameraController::new(0.8);

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(materials) = crate::mesh::Material::from_gltf("assets/cornell-box.glb") {
                scene.materials = materials;
            } else {
                log::warn!("Failed to load GLTF mesh");
            }
        }

        let renderer = Renderer::new(window.clone(), &window_size, &mut scene).await;

        Self {
            window,
            window_size,
            camera_controller,
            renderer,
            scene,
            stat: EngineStatistics::default(),
            config: EngineConfiguration::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_size = new_size;
        self.stat.frame_count = 0;

        // Update camera
        self.scene
            .set_aspect(new_size.width as f32 / new_size.height as f32);
        self.renderer.resize(new_size, &self.scene);

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

        // Clone the Rc to avoid borrow conflicts in the closure
        let rasterizer = self.renderer.rasterizer.clone();

        // Store probe baking request outside the closure
        let mut bake_requested = false;

        let egui_output = self
            .renderer
            .setup_egui(&self.window, |egui_ctx: &egui::Context| {
                egui::SidePanel::right("fps_panel")
                    .exact_width(150.0)
                    .show_separator_line(false)
                    .resizable(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                    .show(egui_ctx, |ui| {
                        ui.label(format!(
                            "Frame Time: {:.2}ms",
                            self.stat.delta_time * 1000.0
                        ));
                        ui.label(format!("FPS: {:.1}", 1.0 / self.stat.delta_time));

                        if self.config.is_raytracer_enabled {
                            ui.label(format!("Frame Count: {}", self.stat.frame_count));
                        }
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                    .show(egui_ctx, |ui| {
                        if self.scene.run_ui(ui) {
                            self.stat.frame_count = 0;
                        }

                        // Run probe UI and capture baking requests
                        let probe_ui_result = rasterizer.borrow_mut().run_probe_ui(ui);
                        if probe_ui_result.bake_requested {
                            bake_requested = true;
                        }

                        // Run the raytracer when the checkbox is toggled on
                        ui.checkbox(&mut self.config.is_raytracer_enabled, "Raytracing");
                    });
            });

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
            &mut self.scene,
            self.stat.frame_count,
            egui_output,
        )?;
        // Set the light dirty flag to false after rendering
        self.scene.set_light_clean();

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
            self.camera_controller
                .update_camera(&mut self.scene, self.stat.delta_time);
        }
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
