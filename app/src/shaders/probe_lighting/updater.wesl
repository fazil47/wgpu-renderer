import package::probe_lighting::util::{
    SHCoefficients,
    ProbeGridConfig,
    probe_index_to_world_pos,
    probe_index_to_tex_coords,
};
import package::raytracer::util::{
    sample_radiance_with_lighting,
    MAX_BOUNCES
};

@group(3) @binding(0) var probe_atlas_l0: texture_storage_3d<rgba32float, write>;
@group(3) @binding(1) var probe_atlas_l1x: texture_storage_3d<rgba32float, write>;
@group(3) @binding(2) var probe_atlas_l1y: texture_storage_3d<rgba32float, write>;
@group(3) @binding(3) var probe_atlas_l1z: texture_storage_3d<rgba32float, write>;
@group(3) @binding(4) var<uniform> config: ProbeGridConfig;

// Constants for spherical harmonics
const SH_C0: f32 = 0.282095; // 1/(2*sqrt(pi))
const SH_C1: f32 = 0.488603; // sqrt(3/(4*pi))

// Radical inverse for Hammersley sequence
fn radical_inverse_vdc(bits_in: u32, probe_offset: u32) -> f32 {
    var bits = bits_in + probe_offset * 73u; // Add probe-specific offset
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10; // / 0x100000000
}



// Project radiance onto spherical harmonics basis
fn project_to_sh(probe_pos: vec3f, probe_index: u32) -> SHCoefficients {
    var coeffs: SHCoefficients;

    // Initialize coefficients
    coeffs.l0 = vec3f(0.0);
    coeffs.l1x = vec3f(0.0);
    coeffs.l1y = vec3f(0.0);
    coeffs.l1z = vec3f(0.0);

    // Monte Carlo integration over sphere with stable sampling
    let sample_count = 256u;
    let weight = 4.0 * 3.14159265 / f32(sample_count);

    for (var i = 0u; i < sample_count; i++) {
        // Generate well-distributed sample directions using simple stratified sampling
        let fi = f32(i);
        let sample_count_f = f32(sample_count);

        // Use Hammersley sequence for better distribution
        let u1 = (fi + 0.5) / sample_count_f;
        let u2 = radical_inverse_vdc(i, probe_index);

        // Convert uniform samples to spherical coordinates
        let cos_theta = 1.0 - 2.0 * u1;
        let sin_theta_sq = 1.0 - cos_theta * cos_theta;
        let sin_theta = sqrt(max(0.0, sin_theta_sq)); // Clamp to avoid NaN
        let phi = 2.0 * 3.14159265 * u2;

        let direction = vec3f(
            sin_theta * cos(phi),
            sin_theta * sin(phi),
            cos_theta
        );

        // Use probe position and sample index for unique bounce seeds
        let bounce_seed = f32(probe_index) * 0.1 + fi * 0.01;
        let radiance = sample_radiance_with_lighting(probe_pos, direction, MAX_BOUNCES, bounce_seed).rgb;

        // L0 basis function (constant)
        let y00 = SH_C0;
        coeffs.l0.x += radiance.r * y00 * weight;
        coeffs.l0.y += radiance.g * y00 * weight;
        coeffs.l0.z += radiance.b * y00 * weight;

        // L1 basis functions (linear) - correct order and scaling
        let y1_neg1 = SH_C1 * direction.y;  // Y(1,-1) = sqrt(3/4π) * y
        let y1_0 = SH_C1 * direction.z;     // Y(1,0) = sqrt(3/4π) * z
        let y1_pos1 = SH_C1 * direction.x;  // Y(1,1) = sqrt(3/4π) * x

        let l1_weights = vec3f(y1_pos1, y1_neg1, y1_0);

        // Store L1 coefficients in (x, y, z) order for each color channel
        coeffs.l1x += radiance * l1_weights.x * weight;
        coeffs.l1y += radiance * l1_weights.y * weight;
        coeffs.l1z += radiance * l1_weights.z * weight;
    }

    // Apply a small normalization factor to prevent excessive brightness
    let normalization_factor = 1.0;
    coeffs.l0 *= normalization_factor;
    coeffs.l1x *= normalization_factor;
    coeffs.l1y *= normalization_factor;
    coeffs.l1z *= normalization_factor;

    return coeffs;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let probe_index = global_id.x;
    let total_probes = config.dimensions.x * config.dimensions.y * config.dimensions.z;

    if probe_index >= total_probes {
        return;
    }

    let probe_pos = probe_index_to_world_pos(probe_index, config);
    let tex_coords = probe_index_to_tex_coords(probe_index, config);
    let coeffs = project_to_sh(probe_pos, probe_index);

    textureStore(probe_atlas_l0, tex_coords, vec4f(coeffs.l0, 1.0));
    textureStore(probe_atlas_l1x, tex_coords, vec4f(coeffs.l1x, 1.0));
    textureStore(probe_atlas_l1y, tex_coords, vec4f(coeffs.l1y, 1.0));
    textureStore(probe_atlas_l1z, tex_coords, vec4f(coeffs.l1z, 1.0));
}
