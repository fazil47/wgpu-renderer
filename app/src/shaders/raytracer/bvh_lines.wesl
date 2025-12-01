struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> instances: array<InstanceData>;

struct InstanceData {
    world_matrix_0: vec4f,
    world_matrix_1: vec4f,
    world_matrix_2: vec4f,
    world_matrix_3: vec4f,
    blas_index: u32,
    // We use 3 u32s instead of vec3u to avoid 16-byte alignment padding.
    // Rust struct is 80 bytes, vec3u would make this 96 bytes.
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct VertexInput {
    @location(0) position: vec4f,
    @location(1) color: vec4f,
    @location(2) instance_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var world_position = input.position;
    if input.instance_index != 0xffffffffu {
        let instance = instances[input.instance_index];
        let world_from_local = mat4x4<f32>(
            instance.world_matrix_0,
            instance.world_matrix_1,
            instance.world_matrix_2,
            instance.world_matrix_3,
        );
        world_position = world_from_local * input.position;
    }

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return input.color;
}
