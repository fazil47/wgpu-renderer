import super::util::{ProbeGridConfig, sample_probe_lighting};

struct Camera {
    view_proj: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) normal: vec4f,
};

struct InstanceInput {
    @location(2) instance_position: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

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

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let world_position = model.position.xyz + instance.instance_position;
    out.clip_position = camera.view_proj * vec4f(world_position, 1.0);
    out.world_position = world_position;
    out.normal = model.normal.xyz;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let color = sample_probe_lighting(
        input.world_position,
        input.normal,
        probe_grid_config,
        probe_atlas_sampler,
        probe_atlas_l0,
        probe_atlas_l1x,
        probe_atlas_l1y,
        probe_atlas_l1z,
    );
    return vec4f(color, 1.0);
}
