// Probe lighting utilities for fixed 3D grid probe system

struct SHCoefficients {
    // L0 coefficients (DC term) for RGB
    l0: vec3f,
    _padding0: f32,

    // L1 coefficients (linear terms) for RGB
    l1x: vec3f,
    _padding1: f32,
    l1y: vec3f,
    _padding2: f32,
    l1z: vec3f,
    _padding3: f32,
}

struct ProbeGridConfig {
    dimensions: vec3u,
    _padding0: u32,
    world_min: vec3f,
    probe_lighting_mix: f32,
    world_max: vec3f,
    probe_lighting_state: u32,
}

fn is_probe_lighting_disabled(config: ProbeGridConfig) -> bool {
    return config.probe_lighting_state == 0;
}

fn is_probe_lighting_enabled(config: ProbeGridConfig) -> bool {
    return config.probe_lighting_state == 1;
}

fn is_probe_lighting_only(config: ProbeGridConfig) -> bool {
    return config.probe_lighting_state == 2;
}

// Evaluate spherical harmonics L0 + L1 for irradiance computation
fn evaluate_sh(coeffs: SHCoefficients, direction: vec3f) -> vec3f {
    // TODO: This isn't used, see if this is better than what I'm currently using

    // For irradiance, we need to convolve with cosine lobe
    // This changes the SH coefficients by specific factors:
    // L0: multiply by π (A0)
    // L1: multiply by 2π/3 (A1)

    let A0 = 3.14159265; // π
    let A1 = 2.09439510; // 2π/3

    let l0_contrib = coeffs.l0 * A0;
    let l1_contrib = vec3f(
        dot(coeffs.l1x, direction),
        dot(coeffs.l1y, direction),
        dot(coeffs.l1z, direction)
    ) * A1;

    return l0_contrib + l1_contrib;
}

fn probe_index_to_world_pos(index: u32, config: ProbeGridConfig) -> vec3f {
    let dimensions = config.dimensions;
    let z = index / (dimensions.x * dimensions.y);
    let y = (index % (dimensions.x * dimensions.y)) / dimensions.x;
    let x = index % dimensions.x;

    let normalized_pos = vec3f(
        f32(x) / f32(max(dimensions.x - 1u, 1u)),
        f32(y) / f32(max(dimensions.y - 1u, 1u)),
        f32(z) / f32(max(dimensions.z - 1u, 1u))
    );

    let world_size = config.world_max - config.world_min;
    return config.world_min + normalized_pos * world_size;
}

fn probe_index_to_tex_coords(index: u32, config: ProbeGridConfig) -> vec3i {
    let z = index / (config.dimensions.x * config.dimensions.y);
    let y = (index % (config.dimensions.x * config.dimensions.y)) / config.dimensions.x;
    let x = index % config.dimensions.x;
    return vec3i(i32(x), i32(y), i32(z));
}

// Convert world position to grid coordinates
fn world_to_grid_coords(world_pos: vec3f, config: ProbeGridConfig) -> vec3f {
    let world_size = config.world_max - config.world_min;
    let local_pos = world_pos - config.world_min;

    return vec3f(
        (local_pos.x / world_size.x) * f32(max(config.dimensions.x - 1u, 1u)),
        (local_pos.y / world_size.y) * f32(max(config.dimensions.y - 1u, 1u)),
        (local_pos.z / world_size.z) * f32(max(config.dimensions.z - 1u, 1u))
    );
}

fn world_to_texture_coords(world_pos: vec3f, config: ProbeGridConfig) -> vec3f {
    let grid_coords = world_to_grid_coords(world_pos, config);

    return vec3f(
        grid_coords.x / f32(max(config.dimensions.x - 1u, 1u)),
        grid_coords.y / f32(max(config.dimensions.y - 1u, 1u)),
        grid_coords.z / f32(max(config.dimensions.z - 1u, 1u))
    );
}

fn sample_probe_lighting(
    world_pos: vec3f,
    normal: vec3f,
    config: ProbeGridConfig,
    atlas_sampler: sampler,
    atlas_l0: texture_3d<f32>,
    atlas_l1x: texture_3d<f32>,
    atlas_l1y: texture_3d<f32>,
    atlas_l1z: texture_3d<f32>,
) -> vec3f {
    let tex_coords = world_to_texture_coords(world_pos, config);

    let l0 = textureSample(atlas_l0, atlas_sampler, tex_coords).rgb;
    let l1x = textureSample(atlas_l1x, atlas_sampler, tex_coords).rgb;
    let l1y = textureSample(atlas_l1y, atlas_sampler, tex_coords).rgb;
    let l1z = textureSample(atlas_l1z, atlas_sampler, tex_coords).rgb;

    let irradiance = l0 + l1x * normal.x + l1y * normal.y + l1z * normal.z;

    return max(irradiance, vec3f(0.0));
}
