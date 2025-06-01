import super::super::util::{
    encodeRGBA,
    decodeRGBA,
};
import super::util::{
    create_ray,
    get_triangle_intersection_mt,
    get_interpolated_color,
    get_sky_color,
    random_in_hemisphere,
    get_triangle_data,
    get_vertex,
    get_material,
    Ray,
    HitInfo,
    SceneParams,
    Material,
    Vertex,
    Triangle,
    MAX_BOUNCES,
    CLEAR_COLOR,
    FLT_MAX,
    SUN_INTENSITY,
    materials,
    material_stride,
    vertices,
    vertex_stride,
    vertex_normal_offset,
    vertex_material_offset,
    indices,
    sun_direction,
};

@group(3) @binding(0) @if(wasm) var result: texture_storage_2d<r32float, read_write>;
@group(3) @binding(0) @else var result: texture_storage_2d<rgba8unorm, read_write>;
@group(3) @binding(1) var<uniform> camera_to_world: mat4x4f;
@group(3) @binding(2) var<uniform> camera_inverse_projection: mat4x4f;
@group(3) @binding(3) var<uniform> frame_count: u32;



fn trace_triangles(ray: Ray) -> HitInfo {
    let num_triangles = u32(arrayLength(&indices) / 3u);

    var nearest_hit_info: HitInfo;
    nearest_hit_info.did_hit = false;
    nearest_hit_info.t = FLT_MAX;

    for (var i: u32 = 0u; i < num_triangles; i = i + 1u) {
        let triangle: Triangle = get_triangle_data(i);
        let hit_info: HitInfo = get_triangle_intersection_mt(triangle, ray);

        if hit_info.did_hit {
            if hit_info.t < nearest_hit_info.t {
                nearest_hit_info = hit_info;
                nearest_hit_info.i = i;
            }
        }
    }

    return nearest_hit_info;
}

fn get_interpolated_color_from_hit(hit_info: HitInfo) -> vec4f {
    let triangle: Triangle = get_triangle_data(hit_info.i);
    return vec4f(get_interpolated_color(hit_info, triangle), 1.0);
}

fn create_camera_ray(uv: vec2f) -> Ray {
    let origin: vec3f = (camera_to_world * vec4f(0.0, 0.0, 0.0, 1.0)).xyz;
    var direction = (camera_inverse_projection * vec4f(uv, 0.0, 1.0)).xyz;
    direction = (camera_to_world * vec4f(direction, 0.0)).xyz;
    direction = normalize(direction);
    return create_ray(origin, direction);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Get the current pixel coordinates
    let coords = vec2i(i32(id.x), i32(id.y));

    if frame_count == 0 {
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

    // Get a ray for the UVs
    var ray: Ray = create_camera_ray(uv);

    // Load the previous frame's color
    var prev_color: vec4f;
    @if(wasm) {
        prev_color = decodeRGBA(textureLoad(result, coords).r);
    } @else {
        prev_color = textureLoad(result, coords);
    }

    let scene_params = SceneParams(
        material_stride,
        vertex_stride,
        vertex_normal_offset,
        vertex_material_offset,
        sun_direction,
    );

    // Trace the ray against the triangles
    var ray_color: vec4f = vec4f(0.0);
    var ray_throughput: vec4f = vec4f(1.0);

    for (var bounce = 0u; bounce <= MAX_BOUNCES; bounce += 1u) {
        if bounce == MAX_BOUNCES {
            // Set ray color as black if the ray has bounced too many times
            ray_color = vec4f(0.0);
            break;
        }

        let hit_info: HitInfo = trace_triangles(ray);
        if hit_info.did_hit {
            let tri_color = get_interpolated_color_from_hit(hit_info);
            ray_color += tri_color * ray_throughput;
            ray_throughput *= tri_color;

            // Use frame number in random seed for temporal variation
            ray = create_ray(
                hit_info.p + hit_info.normal * 0.001, // Move the origin slightly to avoid self-intersection
                random_in_hemisphere(hit_info.normal, hit_info.p + vec3f(f32(frame_count) * 0.1))
            );
        } else {
            if bounce == 0u {
                // If ray misses all triangles, return the sky color
                ray_color = get_sky_color(ray, scene_params);
            } else {
                let ray_sun_intensity = max(0.0, dot(scene_params.sun_direction, ray.direction)) * SUN_INTENSITY;
                ray_color *= ray_sun_intensity * ray_throughput;
            }

            break;
        }
    }

    ray_color = clamp(ray_color, vec4f(0.0), vec4f(1.0));

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
