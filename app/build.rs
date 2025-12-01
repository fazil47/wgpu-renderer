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
        &ModulePath::from_str("package::probe_lighting::visualization").unwrap(),
        "probe-visualization",
    );
}
