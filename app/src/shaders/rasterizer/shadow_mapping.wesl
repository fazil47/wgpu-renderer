@group(0) @binding(0) var<uniform> light_matrix: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = light_matrix * input.position;
    return output;
}