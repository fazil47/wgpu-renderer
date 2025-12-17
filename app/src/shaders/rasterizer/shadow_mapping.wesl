@group(0) @binding(0) var<uniform> light_matrix: mat4x4<f32>;
@group(0) @binding(1) var<uniform> light_direction: vec4f;

const NORMAL_BIAS: f32 = -0.00; // Even the tiniest amount is giving me light leaks

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) normal: vec4f,
    @location(2) transform_0: vec4f,
    @location(3) transform_1: vec4f,
    @location(4) transform_2: vec4f,
    @location(5) transform_3: vec4f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
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

    let world_pos = (transform * input.position).xyz;
    let world_normal = normalize((transform * vec4f(input.normal.xyz, 0.0)).xyz);

    // Apply normal bias
    let NdotL = clamp(dot(world_normal, -light_direction.xyz), 0.0, 1.0);
    let bias_scale = 1.0 - NdotL;
    let biased_pos = world_pos + world_normal * NORMAL_BIAS * bias_scale;

    output.position = light_matrix * vec4f(biased_pos, 1.0);
    return output;
}