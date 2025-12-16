@group(0) @binding(0) var<uniform> light_matrix: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(2) transform_0: vec4f,
    @location(3) transform_1: vec4f,
    @location(4) transform_2: vec4f,
    @location(5) transform_3: vec4f,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let transform = mat4x4<f32>(
        input.transform_0,
        input.transform_1,
        input.transform_2,
        input.transform_3,
    );

    output.position = light_matrix * transform * input.position;
    return output;
}