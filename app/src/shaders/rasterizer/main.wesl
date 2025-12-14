import package::probe_lighting::util::{
    ProbeGridConfig,
    sample_probe_lighting,
    is_probe_lighting_disabled,
    is_probe_lighting_only,
};

struct Camera {
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) normal: vec4f,
    @location(2) transform_0: vec4f,
    @location(3) transform_1: vec4f,
    @location(4) transform_2: vec4f,
    @location(5) transform_3: vec4f,
};

struct VertexOutput {
    @builtin(position) position: vec4f, // Clip space position
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
}

@group(0) @binding(0)
var<uniform> camera_uniform: Camera;
@group(0) @binding(1)
var<uniform> sun_direction: vec3f;

@group(1) @binding(0)
var<uniform> probe_grid_config: ProbeGridConfig;
@group(1) @binding(1)
var probe_atlas_sampler: sampler;
@group(1) @binding(2)
var probe_atlas_l0: texture_3d<f32>;
@group(1) @binding(3)
var probe_atlas_l1x: texture_3d<f32>;
@group(1) @binding(4)
var probe_atlas_l1y: texture_3d<f32>;
@group(1) @binding(5)
var probe_atlas_l1z: texture_3d<f32>;

@group(2) @binding(0)
var<uniform> material_color: vec4f;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let model_transform = mat4x4<f32>(
        model.transform_0,
        model.transform_1,
        model.transform_2,
        model.transform_3,
    );

    // Apply model transform to position and normal
    let world_pos = model_transform * model.position;
    let world_normal = model_transform * model.normal;

    out.position = camera_uniform.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    out.normal = world_normal.xyz;

    return out;
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    // Traditional direct lighting with a minimum intensity of 0.01
    let direct_light: f32 = max(0.0, dot(vert_output.normal, -sun_direction));
    let traditional_lighting = vec3f(max(direct_light, 0.01));

    if (is_probe_lighting_disabled(probe_grid_config)) {
        return vec4f(traditional_lighting * material_color.rgb, material_color.a);
    }

    // Probe-based global illumination
    let probe_lighting = sample_probe_lighting(
        vert_output.world_position,
        vert_output.normal,
        probe_grid_config,
        probe_atlas_sampler,
        probe_atlas_l0,
        probe_atlas_l1x,
        probe_atlas_l1y,
        probe_atlas_l1z,
    );

    if (is_probe_lighting_only(probe_grid_config)) {
        return vec4f(probe_lighting, 1.0);
    }

    let combined_lighting = mix(traditional_lighting, probe_lighting, probe_grid_config.probe_lighting_mix);
    return vec4f(combined_lighting * material_color.rgb, material_color.a);
}
