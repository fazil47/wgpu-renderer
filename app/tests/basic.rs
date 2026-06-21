use app::core::Engine;
use app::core::events::RaytracerReset;
use app::mesh::Mesh;
use app::mesh::static_mesh::StaticMeshExt;
use app::rendering::TlasBuildTask;
use maths::Vec3;

#[test]
fn headless_engine_can_be_created() {
    setup();
    let _engine = pollster::block_on(app::core::Engine::new_headless());
}

#[test]
fn rasterizer_renders_a_frame() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());
    engine.add_mesh("assets/cornell-box.glb").unwrap();
    engine.render().unwrap();
    check_or_update_reference(&engine, "app/tests/reference_images/rasterizer.png", 0.05);
}

#[test]
fn raytracer_renders_a_frame() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());
    engine.add_mesh("assets/cornell-box.glb").unwrap();

    // Enable the raytracer before rendering
    if let Some(mut config) = engine
        .world
        .get_resource_mut::<app::core::engine::EngineConfiguration>()
    {
        config.is_raytracer_enabled = true;
    }

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(&engine, "app/tests/reference_images/raytracer.png", 0.05);
}

#[test]
fn rasterizer_responds_to_transform_changes() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());
    engine.add_mesh("assets/cornell-box.glb").unwrap();

    // Render initial frame
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_initial.png",
        0.05,
    );

    // Move the floor a bit to the left
    let floor = find_entity_by_name(&engine.world, "Floor");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(floor)
            .expect("Floor has no Transform");
        transform.position.x -= 0.5;
    }

    // Move the ceiling a bit down
    let ceiling = find_entity_by_name(&engine.world, "Ceiling");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(ceiling)
            .expect("Ceiling has no Transform");
        transform.position.y -= 0.5;
    }

    // Move Suzanne to the right (positive X from camera's POV)
    let suzanne = find_entity_by_name(&engine.world, "Suzanne");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(suzanne)
            .expect("Suzanne has no Transform");
        transform.position.x += 1.0;
    }

    // Change detection will fire TransformChanged automatically next frame
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_moved.png",
        0.05,
    );

    set_camera_to_corner_view(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_moved_corner_view.png",
        0.05,
    );
}

#[test]
fn raytracer_responds_to_transform_changes() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());
    engine.add_mesh("assets/cornell-box.glb").unwrap();

    // Enable the raytracer before rendering
    if let Some(mut config) = engine
        .world
        .get_resource_mut::<app::core::engine::EngineConfiguration>()
    {
        config.is_raytracer_enabled = true;
    }

    // Render initial frame
    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_initial.png",
        0.05,
    );

    // Move the floor a bit to the left
    let floor = find_entity_by_name(&engine.world, "Floor");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(floor)
            .expect("Floor has no Transform");
        transform.position.x -= 0.5;
    }

    // Move the ceiling a bit down
    let ceiling = find_entity_by_name(&engine.world, "Ceiling");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(ceiling)
            .expect("Ceiling has no Transform");
        transform.position.y -= 0.5;
    }

    // Move Suzanne to the right (positive X from camera's POV)
    let suzanne = find_entity_by_name(&engine.world, "Suzanne");
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(suzanne)
            .expect("Suzanne has no Transform");
        transform.position.x += 1.0;
    }

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_moved.png",
        0.05,
    );

    set_camera_to_corner_view(&mut engine);
    engine.world.send_event(RaytracerReset);

    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_moved_corner_view.png",
        0.05,
    );
}

#[test]
fn rasterizer_mesh_add_and_remove_lifecycle() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());

    // Add cornell-box
    let cornell_entities = engine.add_mesh("assets/cornell-box.glb").unwrap();
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_lifecycle_add_cornell.png",
        0.05,
    );

    // Add cube
    let cube_entity = Mesh::cube(&mut engine.world);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_lifecycle_add_cube.png",
        0.05,
    );

    // Remove cube
    engine.remove_mesh(cube_entity);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_lifecycle_remove_cube.png",
        0.05,
    );

    // Remove cornell-box
    for entity in cornell_entities {
        engine.remove_mesh(entity);
    }
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_lifecycle_remove_cornell.png",
        0.05,
    );
}

#[test]
fn raytracer_mesh_add_and_remove_lifecycle() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());

    // Enable the raytracer
    if let Some(mut config) = engine
        .world
        .get_resource_mut::<app::core::engine::EngineConfiguration>()
    {
        config.is_raytracer_enabled = true;
    }

    // Add cornell-box
    let cornell_entities = engine.add_mesh("assets/cornell-box.glb").unwrap();

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_lifecycle_add_cornell.png",
        0.05,
    );

    // Add cube
    let cube_entity = Mesh::cube(&mut engine.world);

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_lifecycle_add_cube.png",
        0.05,
    );

    // Remove cube
    engine.remove_mesh(cube_entity);

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_lifecycle_remove_cube.png",
        0.05,
    );

    // Remove cornell-box
    for entity in cornell_entities {
        engine.remove_mesh(entity);
    }

    wait_until_tlas_ready(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_lifecycle_remove_cornell.png",
        0.05,
    );
}

// Helpers

/// Shader paths in the engine are relative to the workspace root, not the package root.
/// cargo test sets CWD to the package root (app/), so we need to go up one level.
fn setup() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    std::env::set_current_dir(workspace_root).unwrap();
}

fn set_camera_to_corner_view(engine: &mut app::core::Engine) {
    let mut camera = engine
        .world
        .get_component_mut::<app::camera::Camera>(engine.camera_entity)
        .expect("Camera entity has no Camera component");

    camera.eye = Vec3::new(2.5, 0.9, 3.2);
    camera.forward = (Vec3::new(0.0, 0.0, 0.0) - camera.eye).normalized();
    camera.up = Vec3::Y;
}

fn find_entity_by_name(world: &ecs::World, name: &str) -> ecs::Entity {
    world
        .get_entities_with::<app::transform::Name>()
        .into_iter()
        .find(|&e| {
            world
                .get_component::<app::transform::Name>(e)
                .map(|n| n.as_str() == name)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| panic!("No entity named '{name}' found"))
}

/// Compare rendered pixels against a reference image, or update it if UPDATE_REFERENCES is set.
/// Tolerance is the maximum allowed fraction of differing pixels (0.0–1.0).
fn check_or_update_reference(engine: &app::core::Engine, reference_path: &str, tolerance: f64) {
    let wgpu = engine
        .world
        .get_resource::<app::rendering::wgpu::WgpuResources>()
        .unwrap();
    let pixels = wgpu.target.read_pixels(&wgpu.device, &wgpu.queue);
    let width = wgpu.target.width();
    let height = wgpu.target.height();

    if std::env::var("UPDATE_REFERENCES").is_ok() {
        image::save_buffer(
            reference_path,
            &pixels,
            width,
            height,
            image::ColorType::Rgba8,
        )
        .unwrap();
        return;
    }

    let reference = image::open(reference_path)
        .unwrap_or_else(|e| {
            panic!(
                "Failed to open reference image '{reference_path}': {e}. \
                 Run with UPDATE_REFERENCES=1 to generate it."
            )
        })
        .to_rgba8();

    assert_eq!(
        (width, height),
        reference.dimensions(),
        "Rendered size doesn't match reference"
    );

    let total_pixels = (width * height) as usize;
    let differing_pixels = pixels
        .chunks_exact(4)
        .zip(reference.as_raw().chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    let diff_ratio = differing_pixels as f64 / total_pixels as f64;

    let failure_path = std::path::Path::new("app/tests/reference_image_failures")
        .join(std::path::Path::new(reference_path).file_name().unwrap());

    if diff_ratio > tolerance {
        std::fs::create_dir_all(failure_path.parent().unwrap()).unwrap();
        image::save_buffer(
            &failure_path,
            &pixels,
            width,
            height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }

    assert!(
        diff_ratio <= tolerance,
        "Image mismatch for '{reference_path}': {:.2}% of pixels differ (tolerance: {:.2}%). \
         Wrote actual image to '{}'",
        diff_ratio * 100.0,
        tolerance * 100.0,
        failure_path.display(),
    );
}

fn wait_until_tlas_ready(engine: &mut Engine) {
    let max_frames = 5;

    engine.render().unwrap();

    let tlas_builder = engine.world.get_resource::<TlasBuildTask>();
    if tlas_builder.is_none_or(|builder| !builder.is_building()) {
        return;
    }

    for _ in 0..max_frames {
        let tlas_builder = engine.world.get_resource::<TlasBuildTask>();
        if tlas_builder.is_some_and(|builder| builder.is_finished()) {
            return;
        }

        engine.render().unwrap();
    }

    panic!("TLAS readiness check timed out after {max_frames} frames")
}
