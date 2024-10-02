// TODO: Break up bind groups, see https://toji.dev/webgpu-best-practices/bind-groups.html

@group(0) @binding(0) 
var<storage, read> vertices: array<f32>; // Raw vertex data
@group(0) @binding(1)
var<storage, read> indices: array<u32>;
@group(0) @binding(2)
var<uniform> vertex_stride: u32;
@group(0) @binding(3)
var<uniform> vertex_color_offset: u32;
@group(0) @binding(4)
var<uniform> camera_to_world: mat4x4f;
@group(0) @binding(5)
var<uniform> camera_inverse_projection: mat4x4f;
@group(0) @binding(6)
var result: texture_storage_2d<rgba8unorm, write>;

struct Vertex {
    position: vec4f,
    color: vec4f,
}

struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,
}

fn get_vertex(index: u32) -> Vertex {
    var vertex: Vertex;

    vertex.position = vec4f(
        vertices[index * vertex_stride + 0u],
        vertices[index * vertex_stride + 1u],
        vertices[index * vertex_stride + 2u],
        vertices[index * vertex_stride + 3u],
    );

    vertex.color = vec4f(
        vertices[index * vertex_stride + vertex_color_offset + 0u],
        vertices[index * vertex_stride + vertex_color_offset + 1u],
        vertices[index * vertex_stride + vertex_color_offset + 2u],
        vertices[index * vertex_stride + vertex_color_offset + 3u],
    );

    return vertex;
}

fn get_triangle(index: u32) -> Triangle {
    var triangle: Triangle;

    triangle.a = get_vertex(indices[index * 3u + 0u]);
    triangle.b = get_vertex(indices[index * 3u + 1u]);
    triangle.c = get_vertex(indices[index * 3u + 2u]);

    return triangle;
}

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
