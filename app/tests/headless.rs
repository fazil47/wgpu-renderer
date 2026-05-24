/// Shader paths in the engine are relative to the workspace root, not the package root.
/// cargo test sets CWD to the package root (app/), so we need to go up one level.
fn setup() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    std::env::set_current_dir(workspace_root).unwrap();
}

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
}
