@group(0) @binding(0)
var<uniform> color_uniform: vec4<f32>;

@group(0) @binding(1)
var<uniform> resolution_uniform: vec2<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>, // Clip-space position
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(out.position.xy) * 0.5 + 0.5;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4<f32> {
    // return color_uniform;
    let coord: vec2<f32> = floor(vert_output.uv * resolution_uniform);

    return vec4<f32>(coord / resolution_uniform, 0.0, 1.0);
}