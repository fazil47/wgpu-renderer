use app::core::events::{RaytracerReset, TransformChanged};
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
    engine.render().unwrap();
    check_or_update_reference(&engine, "app/tests/reference_images/rasterizer.png", 0.02);
}

#[test]
fn raytracer_renders_a_frame() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());

    // Enable the raytracer before rendering
    if let Some(mut config) = engine
        .world
        .get_resource_mut::<app::core::engine::EngineConfiguration>()
    {
        config.is_raytracer_enabled = true;
    }

    engine.render().unwrap();
    check_or_update_reference(&engine, "app/tests/reference_images/raytracer.png", 0.02);
}

#[test]
fn rasterizer_responds_to_transform_changes() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());

    // Render initial frame
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_initial.png",
        0.02,
    );

    // Find the entity named "Suzanne"
    let suzanne = engine
        .world
        .get_entities_with::<app::transform::Name>()
        .into_iter()
        .find(|&e| {
            engine
                .world
                .get_component::<app::transform::Name>(e)
                .map(|n| n.as_str() == "Suzanne")
                .unwrap_or(false)
        })
        .expect("No entity named 'Suzanne' found");

    // Move Suzanne to the right (positive X from camera's POV)
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(suzanne)
            .expect("Suzanne has no Transform");
        transform.position.x += 1.0;
    }

    // Send transform changed event
    engine.world.send_event(TransformChanged(suzanne));

    // Render with the moved mesh
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_moved.png",
        0.02,
    );

    set_camera_to_corner_view(&mut engine);
    engine.render().unwrap();
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/rasterizer_suzanne_moved_corner_view.png",
        0.02,
    );
}

#[test]
fn raytracer_responds_to_transform_changes() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());

    // Enable the raytracer before rendering
    if let Some(mut config) = engine
        .world
        .get_resource_mut::<app::core::engine::EngineConfiguration>()
    {
        config.is_raytracer_enabled = true;
    }

    // Render a few frames to converge past noise
    for _ in 0..3 {
        engine.render().unwrap();
    }
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_initial.png",
        0.02,
    );

    // Find the entity named "Suzanne"
    let suzanne = engine
        .world
        .get_entities_with::<app::transform::Name>()
        .into_iter()
        .find(|&e| {
            engine
                .world
                .get_component::<app::transform::Name>(e)
                .map(|n| n.as_str() == "Suzanne")
                .unwrap_or(false)
        })
        .expect("No entity named 'Suzanne' found");

    // Move Suzanne to the right (positive X from camera's POV)
    {
        let mut transform = engine
            .world
            .get_component_mut::<app::transform::Transform>(suzanne)
            .expect("Suzanne has no Transform");
        transform.position.x += 1.0;
    }

    // Send transform changed and raytracer reset events
    engine.world.send_event(TransformChanged(suzanne));
    engine.world.send_event(RaytracerReset);

    // Render a few frames to converge past noise
    for _ in 0..3 {
        engine.render().unwrap();
    }
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_moved.png",
        0.02,
    );

    set_camera_to_corner_view(&mut engine);
    engine.world.send_event(RaytracerReset);

    // Render a few frames to converge past noise
    for _ in 0..3 {
        engine.render().unwrap();
    }
    check_or_update_reference(
        &engine,
        "app/tests/reference_images/raytracer_suzanne_moved_corner_view.png",
        0.02,
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

    assert!(
        diff_ratio <= tolerance,
        "Image mismatch for '{reference_path}': {:.2}% of pixels differ (tolerance: {:.2}%)",
        diff_ratio * 100.0,
        tolerance * 100.0,
    );
}
