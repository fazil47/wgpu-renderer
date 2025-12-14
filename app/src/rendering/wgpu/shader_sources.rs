/// Embedded shader sources for WASM builds
/// This module contains all shader files embedded at compile time for platforms
/// that don't have filesystem access (like WASM)

#[cfg(target_arch = "wasm32")]
pub fn get_all_shader_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        // Root package module - use "package" for the absolute root
        ("package", include_str!("../../shaders/package.wesl")),
        ("package::util", include_str!("../../shaders/util.wesl")),
        // Probe lighting
        (
            "package::probe_lighting",
            include_str!("../../shaders/probe_lighting.wesl"),
        ),
        (
            "package::probe_lighting::updater",
            include_str!("../../shaders/probe_lighting/updater.wesl"),
        ),
        (
            "package::probe_lighting::util",
            include_str!("../../shaders/probe_lighting/util.wesl"),
        ),
        (
            "package::probe_lighting::visualization",
            include_str!("../../shaders/probe_lighting/visualization.wesl"),
        ),
        // Rasterizer
        (
            "package::rasterizer",
            include_str!("../../shaders/rasterizer.wesl"),
        ),
        (
            "package::rasterizer::main",
            include_str!("../../shaders/rasterizer/main.wesl"),
        ),
        (
            "package::rasterizer::shadow_mapping",
            include_str!("../../shaders/rasterizer/shadow_mapping.wesl"),
        ),
        (
            "package::rasterizer::blit_to_screen",
            include_str!("../../shaders/rasterizer/blit_to_screen.wesl"),
        ),
        // Raytracer
        (
            "package::raytracer",
            include_str!("../../shaders/raytracer.wesl"),
        ),
        (
            "package::raytracer::render",
            include_str!("../../shaders/raytracer/render.wesl"),
        ),
        (
            "package::raytracer::compute",
            include_str!("../../shaders/raytracer/compute.wesl"),
        ),
        (
            "package::raytracer::util",
            include_str!("../../shaders/raytracer/util.wesl"),
        ),
        (
            "package::raytracer::bvh_lines",
            include_str!("../../shaders/raytracer/bvh_lines.wesl"),
        ),
    ]
}
