fn main() {
    let is_wasm = std::env::var("TARGET").unwrap_or_default().contains("wasm");

    let mut resolver = wesl::Wesl::new("src/shaders");
    resolver.set_feature("wasm".into(), is_wasm);
    resolver.build_artefact("rasterizer/main.wesl", "rasterizer-main");
    resolver.build_artefact("raytracer/render.wesl", "raytracer-render");
    resolver.build_artefact("raytracer/compute.wesl", "raytracer-compute");
}
