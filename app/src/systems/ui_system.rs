use crate::core::engine::{
    EngineConfiguration, RaytracerFrameState, SelectedEntity, WindowResource,
};
use crate::lighting::DirectionalLight;
use crate::rendering::rasterizer::Rasterizer;
use crate::ui::UiState;
use crate::ui::egui::RendererEgui;
use crate::ui::mesh_hierarchy::{build_mesh_hierarchy, draw_mesh_hierarchy};
use ecs::World;

pub fn ui_system(world: &mut World) {
    // 1. Extract resources
    let (mut raytracer_enabled, mut raytracer_show_bvh, delta_time, frame_count) = {
        let config = world.get_resource::<EngineConfiguration>().unwrap();
        let stat_delta_time = if let Some(time) = world.get_resource::<crate::time::Time>() {
            time.delta_time
        } else {
            panic!("Time resource not found");
        };
        let frame_count = world
            .get_resource::<RaytracerFrameState>()
            .map(|s| s.frame_count)
            .unwrap_or(0);

        (
            config.is_raytracer_enabled,
            config.show_bvh,
            stat_delta_time,
            frame_count,
        )
    };

    let mesh_hierarchy = build_mesh_hierarchy(world);
    let mut bake_requested = false;
    let mut has_transform_changed = false;
    let mut is_light_dirty = false;

    // 2. Run Egui
    let egui_output = {
        let mut egui = world.get_resource_mut::<RendererEgui>().unwrap();
        let mut selected = world.get_resource_mut::<SelectedEntity>().unwrap();
        let mut rasterizer = world.get_resource_mut::<Rasterizer>().unwrap();
        let window_resource = world.get_resource::<WindowResource>().unwrap();
        let window = &window_resource.0;

        let egui_raw_input = egui.state.take_egui_input(window);

        // Update gizmo camera
        if let Some(camera_entity) = world
            .get_entities_with::<crate::camera::Camera>()
            .first()
            .copied()
        {
            egui.update_camera(world, camera_entity);
        }

        let egui_ctx = egui.state.egui_ctx().clone();

        egui_ctx.run(egui_raw_input, |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    if let Some(entity) = selected.0 {
                        has_transform_changed = egui.select_entity(world, ui, entity);
                    }

                    egui::SidePanel::right("fps_panel")
                        .exact_width(150.0)
                        .show_separator_line(false)
                        .resizable(false)
                        .frame(egui::Frame::new().inner_margin(egui::Margin::same(10)))
                        .show(ctx, |ui| {
                            let delta_time_ms = delta_time * 1000.0;
                            let fps = 1.0 / delta_time;

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
                                // We need to find the sun light entity.
                                // Since we don't have direct access to sun_light_entity ID,
                                // we query for DirectionalLight component.
                                // Assuming single directional light for now.
                                let light_entities = world.get_entities_with::<DirectionalLight>();
                                if let Some(light_entity) = light_entities.first()
                                    && let Some(mut light) =
                                        world.get_component_mut::<DirectionalLight>(*light_entity)
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
                                        is_light_dirty = true;
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

    // 3. Update state based on UI interactions
    if let Some(mut config) = world.get_resource_mut::<EngineConfiguration>() {
        config.is_raytracer_enabled = raytracer_enabled;
        config.show_bvh = raytracer_show_bvh;
    }

    if let Some(mut flags) = world.get_resource_mut::<crate::core::flags::DirtyFlags>() {
        if has_transform_changed {
            flags.raytracer_reset = true;
            flags.static_data = true;
        }
        if bake_requested {
            flags.probe_bake_requested = true;
        }
        if is_light_dirty {
            flags.lights = true;
            flags.raytracer_reset = true;
        }
    }

    // 4. Store UI output in UiState
    if let Some(mut ui_state) = world.get_resource_mut::<UiState>() {
        ui_state.egui_output = Some(egui_output);
        ui_state.fps = 1.0 / delta_time;
        ui_state.frame_time_ms = delta_time * 1000.0;
    }
}
