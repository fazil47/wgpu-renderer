struct Camera {
    view_proj: mat4x4f,
};

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
};

struct VertexOutput {
    @builtin(position) position: vec4f, // Clip space position
    @location(0) color: vec4f,
}

@group(0) @binding(0)
var<uniform> camera_uniform: Camera;

@group(0) @binding(1)
var<uniform> color_uniform: vec4f;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = camera_uniform.view_proj * model.position;
    out.color = model.color;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    return color_uniform * vert_output.color;
}
