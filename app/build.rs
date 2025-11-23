use std::str::FromStr;

use wesl::ModulePath;

fn main() {
    let is_wasm = std::env::var("TARGET").unwrap_or_default().contains("wasm");

    let mut resolver = wesl::Wesl::new("src/shaders");
    resolver.set_feature("wasm", is_wasm);
    resolver.build_artifact(
        &ModulePath::from_str("package::rasterizer::main").unwrap(),
        "rasterizer-main",
    );
    resolver.build_artifact(
        &ModulePath::from_str("package::raytracer::render").unwrap(),
        "raytracer-render",
    );
    resolver.build_artifact(
        &ModulePath::from_str("package::raytracer::compute").unwrap(),
        "raytracer-compute",
    );
    resolver.build_artifact(
        &ModulePath::from_str("package::raytracer::bvh_lines").unwrap(),
        "raytracer-bvh-lines",
    );
    resolver.build_artifact(
        &ModulePath::from_str("package::probe_lighting::updater").unwrap(),
        "probe-updater",
    );
    resolver.build_artifact(
        &ModulePath::from_str("package::probe_lighting::visualization").unwrap(),
        "probe-visualization",
    );
}
