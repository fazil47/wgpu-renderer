struct Camera {
    view_proj: mat4x4f,
};

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
};

@group(0) @binding(0)
var<uniform> camera_uniform: Camera;

@group(0) @binding(1)
var<uniform> color_uniform: vec4f;

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4f {
    return camera_uniform.view_proj * vec4f(model.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return color_uniform;
}
