@group(0) @binding(0)
var<uniform> color_uniform: vec4<f32>;

@group(0) @binding(1)
var<uniform> resolution_uniform: vec2<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>, // Clip-space position
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(model.position, 1.0);
    out.uv = vec2<f32>(out.position.xy) * 0.5 + 0.5;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4<f32> {
    // return color_uniform;
    let coord: vec2<f32> = floor(vert_output.uv * resolution_uniform);

    return vec4<f32>(coord / resolution_uniform, 0.0, 1.0);
}