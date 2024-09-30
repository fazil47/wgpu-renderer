// TODO: Break up bind groups, see https://toji.dev/webgpu-best-practices/bind-groups.html

@group(0) @binding(0)
var<uniform> camera_to_world: mat4x4f;
@group(0) @binding(1)
var<uniform> camera_inverse_projection: mat4x4f;
@group(0) @binding(2)
var result: texture_storage_2d<rgba8unorm, write>;

struct Ray {
    origin: vec3f,
    direction: vec3f,
}

fn create_ray(origin: vec3f, direction: vec3f) -> Ray {
    var ray: Ray;
    ray.origin = origin;
    ray.direction = direction;
    return ray;
}

fn create_camera_ray(uv: vec2f) -> Ray {
    // Transform the camera origin to world space
    let origin: vec3f = (camera_to_world * vec4f(0.0, 0.0, 0.0, 1.0)).xyz;

    // Invert the perspective projection of the view-space position
    var direction = (camera_inverse_projection * vec4f(uv, 0.0, 1.0)).xyz;

    // Transform the direction from camera to world space and normalize
    direction = (camera_to_world * vec4f(direction, 0.0)).xyz;
    direction = normalize(direction);

    return create_ray(origin, direction);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Get the resolution of the result texture
    let dims = vec2f(textureDimensions(result).xy);

    // Transform pixel to [-1, 1] range
    let uv = (vec2f(id.xy) + vec2f(0.5, 0.5)) / dims * 2.0 - 1.0;

    // Get a ray for the UVs
    let ray: Ray = create_camera_ray(uv);

    // Write some colors
    let coords = vec2i(i32(id.x), i32(id.y));
    textureStore(result, coords, vec4f(ray.direction * 0.5 + 0.5, 1.0));
}