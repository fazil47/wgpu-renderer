const K_EPSILON: f32 = 1e-6;

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

fn get_triangle_normal(triangle: Triangle) -> vec3f {
    let edge1 = triangle.b.position.xyz - triangle.a.position.xyz;
    let edge2 = triangle.c.position.xyz - triangle.a.position.xyz;
    return normalize(cross(edge1, edge2)); // TODO: Should I normalize?
}

struct HitInfo {
    did_hit: bool,
    t: f32, // Distance to the intersection point from the ray origin
    p: vec3f, // Intersection point
    normal: vec3f, // Normal at the intersection point
}

fn does_ray_intersect_triangle_plane(ray: Ray, triangle: Triangle) -> HitInfo {
    var hit_info: HitInfo;

    let tri_normal: vec3f = get_triangle_normal(triangle);
    hit_info.normal = tri_normal;

    // Step 1: Does the ray intersect the triangle's plane?

    // Check if the ray is parallel to the triangle
    let tri_plane_dot_ray: f32 = dot(tri_normal, ray.direction);

    if (abs(tri_plane_dot_ray) < K_EPSILON) {
        hit_info.did_hit = false;   // Ray is parallel to the triangle's plane
        return hit_info;
    }

    let d: f32 = dot(tri_normal, triangle.a.position.xyz); // Distance to the plane from the origin

    let t: f32 = -(dot(tri_normal, ray.origin) + d) / tri_plane_dot_ray; // Distance to the intersection point from ray origin

    if (t < 0.0) {
        hit_info.did_hit = false;   // Intersection point is behind the ray
        return hit_info;
    }

    let p: vec3f = ray.origin + ray.direction * t; // Intersection point
    hit_info.p = p;

    // Step 2: Is the intersection point inside the triangle?

    var c: vec3f; // Vector perpendicular to triangle's plane

    // Edge 0
    let edge0: vec3f = triangle.b.position.xyz - triangle.a.position.xyz;
    let vp0: vec3f = p - triangle.a.position.xyz;
    c = cross(edge0, vp0);

    if (dot(tri_normal, c) < 0.0) {
        hit_info.did_hit = false;   // Intersection point is to the right of edge 0
        return hit_info;
    }

    // Edge 1
    let edge1: vec3f = triangle.c.position.xyz - triangle.b.position.xyz;
    let vp1: vec3f = p - triangle.b.position.xyz;
    c = cross(edge1, vp1);

    if (dot(tri_normal, c) < 0.0) {
        hit_info.did_hit = false;   // Intersection point is to the right of edge 1
        return hit_info;
    }

    // Edge 2
    let edge2: vec3f = triangle.a.position.xyz - triangle.c.position.xyz;
    let vp2: vec3f = p - triangle.c.position.xyz;
    c = cross(edge2, vp2);

    if (dot(tri_normal, c) < 0.0) {
        hit_info.did_hit = false;   // Intersection point is to the right of edge 2
        return hit_info;
    }

    hit_info.did_hit = true;
    hit_info.t = t;

    return hit_info;
}

fn get_ray_color(ray: Ray) -> vec4f {
    return vec4f(ray.direction * 0.5 + 0.5, 1.0);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Get the resolution of the result texture
    let dims = vec2f(textureDimensions(result).xy);

    // Transform texture coordinate to [-1, 1] range from [0, 1]
    // Then the coordinates will go from -1 to 1 down the Y-axis
    // We want to go from -1 to 1 up the Y-axis to match WebGPU's render coordinate system
    let uv = ((vec2f(id.xy) + 0.5) / dims * 2.0 - 1.0) * vec2f(1.0, -1.0);

    // Get a ray for the UVs
    let ray: Ray = create_camera_ray(uv);

    // Write some colors
    let coords = vec2i(i32(id.x), i32(id.y));

    let triangle: Triangle = get_triangle(0u);

    let hit_info: HitInfo = does_ray_intersect_triangle_plane(ray, triangle);

    if (hit_info.did_hit) {
        // TODO: Use barycentric coordinates to interpolate the color
        textureStore(result, coords, triangle.a.color);
        return;
    }

    textureStore(result, coords, vec4f(0.0, 0.0, 0.0, 1.0)); // Background color

    // textureStore(result, coords, get_ray_color(ray)); // Used to debug the camera
}