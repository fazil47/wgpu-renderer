struct Camera {
    view_proj: mat4x4f,
};

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
    @location(2) normal: vec3f,
};

struct VertexOutput {
    @builtin(position) position: vec4f, // Clip space position
    @location(0) color: vec4f,
    @location(1) normal: vec3f,
}

@group(0) @binding(0)
var<uniform> camera_uniform: Camera;
@group(0) @binding(1)
var<uniform> color_uniform: vec4f;
@group(0) @binding(2)
var<uniform> sun_direction: vec3f;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = camera_uniform.view_proj * model.position;
    out.color = model.color;
    out.normal = model.normal;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    let direct_light: f32 = max(0.0, dot(vert_output.normal, sun_direction));
    let ambient_light: f32 = 0.05;
    let total_light: f32 = direct_light + ambient_light;
    let color = vert_output.color * vec4f(total_light, total_light, total_light, 1.0);

    return color_uniform * color;
}
