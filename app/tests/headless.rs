#[test]
fn engine_creates_headless() {
    setup();
    let _engine = pollster::block_on(app::core::Engine::new_headless());
}

#[test]
fn engine_renders_frame_headless() {
    setup();
    let mut engine = pollster::block_on(app::core::Engine::new_headless());
    engine.render().unwrap();
    check_or_update_reference(&engine, "app/tests/reference_images/rasterizer.png", 0.02);
}

#[test]
fn engine_renders_raytracer_headless() {
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

// Helpers

/// Shader paths in the engine are relative to the workspace root, not the package root.
/// cargo test sets CWD to the package root (app/), so we need to go up one level.
fn setup() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    std::env::set_current_dir(workspace_root).unwrap();
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
