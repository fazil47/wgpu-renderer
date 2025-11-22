import super::super::util::{
    encodeRGBA,
    decodeRGBA,
};
import super::util::{
    sample_radiance_with_lighting,
    MAX_BOUNCES,
    MAX_LOW_FRAME_BOUNCES,
    LOW_BOUNCE_FRAMES,
    CLEAR_COLOR,
};

@group(3) @binding(0) @if(wasm) var result: texture_storage_2d<r32float, read_write>;
@group(3) @binding(0) @else var result: texture_storage_2d<rgba8unorm, read_write>;
@group(3) @binding(1) var<uniform> camera_to_world: mat4x4<f32>;
@group(3) @binding(2) var<uniform> camera_inverse_projection: mat4x4<f32>;
@group(3) @binding(3) var<uniform> frame_count: u32;

fn get_camera_position(uv: vec2f) -> vec3f {
    return (camera_to_world * vec4f(0.0, 0.0, 0.0, 1.0)).xyz;
}

fn get_camera_direction(uv: vec2f) -> vec3f {
    var direction = (camera_inverse_projection * vec4f(uv, 0.0, 1.0)).xyz;
    direction = (camera_to_world * vec4f(direction, 0.0)).xyz;
    return normalize(direction);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Get the current pixel coordinates
    let coords = vec2i(i32(id.x), i32(id.y));

    if frame_count == 0u {
        // Clear the result texture on the first frame
        var clear_color: vec4f;
        @if(wasm) {
            clear_color = vec4f(encodeRGBA(CLEAR_COLOR));
        } @else {
            clear_color = CLEAR_COLOR;
        }

        textureStore(result, coords, clear_color);
    }

    // Get the resolution of the result texture
    let dims = vec2f(textureDimensions(result).xy);

    // Transform texture coordinate to [-1, 1] range from [0, 1]
    // Then the coordinates will go from -1 to 1 down the Y-axis
    // We want to go from -1 to 1 up the Y-axis to match WebGPU's render coordinate system
    // Reference: https://github.com/gfx-rs/wgpu?tab=readme-ov-file#coordinate-systems
    let uv = ((vec2f(id.xy) + 0.5) / dims * 2.0 - 1.0) * vec2f(1.0, -1.0);

    // Load the previous frame's color
    var prev_color: vec4f;
    @if(wasm) {
        prev_color = decodeRGBA(textureLoad(result, coords).r);
    } @else {
        prev_color = textureLoad(result, coords);
    }

    var max_bounces = MAX_BOUNCES;
    if frame_count < LOW_BOUNCE_FRAMES {
        max_bounces = MAX_LOW_FRAME_BOUNCES;
    }

    var camera_position = get_camera_position(uv);
    var camera_direction = get_camera_direction(uv);
    var ray_color: vec4f = sample_radiance_with_lighting(camera_position, camera_direction, max_bounces, f32(frame_count) * 0.1);

    // Blend with previous frame
    let blend_factor = 1.0 / pow(f32(frame_count + 1), 1.05);
    let mix_color = mix(prev_color, ray_color, blend_factor);

    var final_color: vec4f;
    @if(wasm) {
        final_color = vec4f(encodeRGBA(mix_color));
    } @else {
        final_color = mix_color;
    }

    textureStore(result, coords, final_color);

    // textureStore(result, coords, get_ray_color(ray)); // Used to debug the camera
}
