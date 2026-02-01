import runtime::constants::SHADOW_MAP_SIZE;
import package::probe_lighting::util::{
    ProbeGridConfig,
    sample_probe_lighting,
    is_probe_lighting_disabled,
    is_probe_lighting_only,
};

struct Camera {
    view_proj: mat4x4<f32>,
};

struct LightMatrices {
    view_proj_matrices: array<mat4x4<f32>, 4>,
    cascade_distances: vec4f, // Z distances in camera space where each cascade starts
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
var<uniform> sun_direction: vec4f;
@group(0) @binding(2)
var shadow_map_sampler: sampler_comparison;
@group(0) @binding(3)
var shadow_map: texture_depth_2d_array;
@group(0) @binding(4)
var<uniform> light_matrices: LightMatrices;

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
@group(2) @binding(1)
var<uniform> material_emissive: vec4f;

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

// TODO: implement bilinear PCF where multiple texels are sampled with a single tap by sampling at half texel offsets
fn sample_shadow_pcf(depth: f32, uv: vec2f, cascade_idx: u32) -> f32 {
    let texel_size = 1.0 / f32(SHADOW_MAP_SIZE);
    var shadow = 0.0;
    
    // 5-tap cross pattern
    shadow += textureSampleCompare(shadow_map, shadow_map_sampler, uv, cascade_idx, depth);
    shadow += textureSampleCompare(shadow_map, shadow_map_sampler, uv + vec2f(texel_size, 0.0), cascade_idx, depth);
    shadow += textureSampleCompare(shadow_map, shadow_map_sampler, uv + vec2f(-texel_size, 0.0), cascade_idx, depth);
    shadow += textureSampleCompare(shadow_map, shadow_map_sampler, uv + vec2f(0.0, texel_size), cascade_idx, depth);
    shadow += textureSampleCompare(shadow_map, shadow_map_sampler, uv + vec2f(0.0, -texel_size), cascade_idx, depth);
    
    return shadow / 5.0;
}

fn sample_csm_shadow_pcf(view_depth: f32, world_position: vec3f) -> f32 {
    var cascade_idx = 0u;
    let num_cascades = u32(sun_direction.w);
    for (var i = 0u; i < num_cascades; i += 1u) {
        if (i == num_cascades - 1) {
            cascade_idx = i;
            break;
        }

        let current_cascade_distance = light_matrices.cascade_distances[i];
        let next_cascade_distance = light_matrices.cascade_distances[i + 1];
        if (view_depth > current_cascade_distance && view_depth < next_cascade_distance) {
            cascade_idx = i;
            break;
        }
    }

    let light_matrix = light_matrices.view_proj_matrices[cascade_idx];
    let light_ndc_position = light_matrix * vec4f(world_position, 1.0);
    let light_ndc_depth = light_ndc_position.z;
    let shadow_map_uv = light_ndc_position.xy * vec2f(0.5, -0.5) + vec2f(0.5, 0.5); // transform from [-1, 1] to [-0.5, 0.5] and then [0, 1] (negate y since it goes from 0 to 1 from top to bottom)

    return sample_shadow_pcf(light_ndc_depth, shadow_map_uv, cascade_idx);
}

@fragment
fn fs_main(vert_output: VertexOutput) -> @location(0) vec4f {
    let view_depth = vert_output.position.w; // The view space z coord will be stored clip space w
    let shadow = sample_csm_shadow_pcf(view_depth, vert_output.world_position);

    let direct_light: f32 = max(0.0, dot(vert_output.normal, -sun_direction.xyz)) * shadow;
    let traditional_lighting = vec3f(max(direct_light, 0.01));

    if (is_probe_lighting_disabled(probe_grid_config)) {
        let lit_color = traditional_lighting * material_color.rgb;
        let final_color = lit_color + material_emissive.rgb;
        return vec4f(final_color, material_color.a);
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
    let lit_color = combined_lighting * material_color.rgb;
    let final_color = lit_color + material_emissive.rgb;
    return vec4f(final_color, material_color.a);
}
