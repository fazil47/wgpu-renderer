@group(0) @binding(0)
var<uniform> color_uniform: vec4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(model.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return color_uniform;
}