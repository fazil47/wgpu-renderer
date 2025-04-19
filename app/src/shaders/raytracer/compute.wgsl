const K_EPSILON: f32 = 1e-6;
const FLT_MAX: f32 = 1e12;
const MAX_BOUNCES: u32 = 8;
const SUN_INTENSITY: f32 = 1.0;

@group(0) @binding(0)
var<storage, read> materials: array<f32>; // Raw material data
@group(0) @binding(1)
var<uniform> material_stride: u32;

@group(1) @binding(0)
var<storage, read> vertices: array<f32>; // Raw vertex data
@group(1) @binding(1)
var<uniform> vertex_stride: u32;
@group(1) @binding(2)
var<uniform> vertex_normal_offset: u32;
@group(1) @binding(3)
var<uniform> vertex_material_offset: u32;
@group(1) @binding(4)
var<storage, read> indices: array<u32>;

@group(2) @binding(0)
var result: texture_storage_2d<rgba8unorm, read_write>;

@group(3) @binding(0)
var<uniform> sun_direction: vec3f;
@group(3) @binding(1)
var<uniform> camera_to_world: mat4x4f;
@group(3) @binding(2)
var<uniform> camera_inverse_projection: mat4x4f;
@group(3) @binding(3)
var<uniform> frame_count: u32;

struct Material {
    color: vec4f,
}

struct Vertex {
    position: vec4f,
    normal: vec4f,
    color: vec4f,
}

struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,
}

fn get_material(index: u32) -> Material {
    var material: Material;

    material.color = vec4f(
        materials[index * material_stride + 0u],
        materials[index * material_stride + 1u],
        materials[index * material_stride + 2u],
        materials[index * material_stride + 3u],
    );

    return material;
}

fn get_vertex(index: u32) -> Vertex {
    var vertex: Vertex;

    vertex.position = vec4f(
        vertices[index * vertex_stride + 0u],
        vertices[index * vertex_stride + 1u],
        vertices[index * vertex_stride + 2u],
        vertices[index * vertex_stride + 3u],
    );

    vertex.normal = vec4f(
        vertices[index * vertex_stride + vertex_normal_offset + 0u],
        vertices[index * vertex_stride + vertex_normal_offset + 1u],
        vertices[index * vertex_stride + vertex_normal_offset + 2u],
        vertices[index * vertex_stride + vertex_normal_offset + 3u],
    );

    let material_index: u32 = u32(vertices[index * vertex_stride + vertex_material_offset]);
    let material = get_material(material_index);
    vertex.color = material.color;

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

fn get_unnormalized_triangle_normal(triangle: Triangle) -> vec3f {
    let edge1 = triangle.b.position.xyz - triangle.a.position.xyz;
    let edge2 = triangle.c.position.xyz - triangle.a.position.xyz;
    return cross(edge1, edge2);
}

struct HitInfo {
    did_hit: bool,
    t: f32, // Distance to the intersection point from the ray origin
    i: u32, // Index of the intersected triangle
    p: vec3f, // Intersection point
    normal: vec3f, // Normal at the intersection point,
    u: f32, // Barycentric coordinates u
    v: f32, // Barycentric coordinates v
    w: f32, // Barycentric coordinates w
}

// Reference: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/ray-triangle-intersection-geometric-solution.html
fn get_triangle_intersection(triangle: Triangle, ray: Ray) -> HitInfo {
    var hit_info: HitInfo;

    let unnormalized_tri_normal: vec3f = get_unnormalized_triangle_normal(triangle);
    let area = length(unnormalized_tri_normal);
    let tri_normal = normalize(unnormalized_tri_normal);
    hit_info.normal = tri_normal;

    // Step 1: Does the ray intersect the triangle's plane?

    let tri_plane_dot_ray: f32 = dot(tri_normal, ray.direction);

    if (abs(tri_plane_dot_ray) < K_EPSILON) {
        hit_info.did_hit = false;   // Ray is parallel to the triangle's plane
        return hit_info;
    } else if (tri_plane_dot_ray > 0.0) {
        // TODO: Make backface culling optional
        hit_info.did_hit = false;   // Ray is facing away from the triangle's plane
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
    let u: f32 = dot(tri_normal, c);

    if (u < 0.0) {
        hit_info.did_hit = false;   // Intersection point is to the right of edge 1
        return hit_info;
    }

    // Edge 2
    let edge2: vec3f = triangle.a.position.xyz - triangle.c.position.xyz;
    let vp2: vec3f = p - triangle.c.position.xyz;
    c = cross(edge2, vp2);
    let v: f32 = dot(tri_normal, c);

    if (v < 0.0) {
        hit_info.did_hit = false;   // Intersection point is to the right of edge 2
        return hit_info;
    }

    hit_info.did_hit = true;
    hit_info.t = t;
    hit_info.u = u / area;
    hit_info.v = v / area;
    hit_info.w = 1.0 - hit_info.u - hit_info.v;

    return hit_info;
}

// Reference: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection.html
fn get_triangle_intersection_mt(triangle: Triangle, ray: Ray) -> HitInfo {
    var hit_info: HitInfo;

    let e0: vec3f = triangle.a.position.xyz - triangle.c.position.xyz;
    let e1: vec3f = triangle.b.position.xyz - triangle.c.position.xyz;
    let pvec: vec3f = cross(ray.direction, e1);
    let det: f32 = dot(e0, pvec);

    hit_info.normal = normalize(cross(e0, e1));

    // If the determinant is less than zero, the triangle is backfacing
    // TODO: Make backface culling optional
    if (det < K_EPSILON) {
        hit_info.did_hit = false;
        return hit_info;
    }

    // If the determinant is close to zero, the ray is parallel to the triangle
    if (abs(det) < K_EPSILON) {
        hit_info.did_hit = false;
        return hit_info;
    }

    let inv_det: f32 = 1.0 / det;

    let tvec: vec3f = ray.origin - triangle.c.position.xyz;
    let u: f32 = dot(tvec, pvec) * inv_det;
    if (u < 0 || u > 1) {
        hit_info.did_hit = false;
        return hit_info;
    }

    let qvec: vec3f = cross(tvec, e0);
    let v: f32 = dot(ray.direction, qvec) * inv_det;
    if (v < 0 || u + v > 1) {
        hit_info.did_hit = false;
        return hit_info;
    }

    let t: f32 = dot(e1, qvec) * inv_det;

    if (t < 0) {
        // Intersection point is behind the ray
        hit_info.did_hit = false;
        return hit_info;
    }

    hit_info.did_hit = true;
    hit_info.t = t;
    hit_info.u = u;
    hit_info.v = v;
    hit_info.w = 1.0 - u - v;
    hit_info.p = ray.origin + ray.direction * t;

    return hit_info;
}

fn get_interpolated_color(hit_info: HitInfo) -> vec4f {
    let triangle: Triangle = get_triangle(hit_info.i);
    return triangle.a.color * hit_info.u + triangle.b.color * hit_info.v + triangle.c.color * hit_info.w;
}

fn get_sky_color(ray: Ray) -> vec4f {
    // The sky is black except for the sun
    var sun_intensity = max(0.0, dot(sun_direction, ray.direction));
    sun_intensity = pow(sun_intensity, 32.0);

    return vec4f(sun_intensity, sun_intensity, sun_intensity, 1.0);
}

fn trace_triangles(ray: Ray) -> HitInfo {
    let num_triangles = u32(arrayLength(&indices) / 3u);

    var nearest_hit_info: HitInfo;
    nearest_hit_info.did_hit = false;
    nearest_hit_info.t = FLT_MAX;

    for (var i: u32 = 0u; i < num_triangles; i = i + 1u) {
        let triangle: Triangle = get_triangle(i);
        let hit_info: HitInfo = get_triangle_intersection_mt(triangle, ray);

        if (hit_info.did_hit) {
            if (hit_info.t < nearest_hit_info.t) {
                nearest_hit_info = hit_info;
                nearest_hit_info.i = i;
            }
        }
    }

    return nearest_hit_info;
}

fn get_ray_color(ray: Ray) -> vec4f {
    return vec4f(ray.direction * 0.5 + 0.5, 1.0);
}

fn random_in_hemisphere(normal: vec3f, seed: vec3f) -> vec3f {
    // Generate a random direction in hemisphere above the surface
    let theta = 2.0 * 3.14159 * fract(sin(dot(seed, vec3f(12.9898, 78.233, 45.164))) * 43758.5453);
    let phi = acos(2.0 * fract(sin(dot(seed, vec3f(63.7264, 10.873, 98.234))) * 43758.5453) - 1.0);

    let x = sin(phi) * cos(theta);
    let y = sin(phi) * sin(theta);
    let z = cos(phi);
    let random_dir = normalize(vec3f(x, y, z));

    // Ensure the direction is in the correct hemisphere
    return normalize(normal + random_dir);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Get the current pixel coordinates
    let coords = vec2i(i32(id.x), i32(id.y));

    if (frame_count == 0) {
        // Clear the result texture on the first frame
        textureStore(result, coords, vec4f(0.0, 0.0, 0.0, 1.0));
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
    let prev_color = textureLoad(result, coords);

    // Trace the ray against the triangles
    var ray_color: vec4f = vec4f(0.0);
    var ray_throughput: vec4f = vec4f(1.0);

    for (var bounce = 0u; bounce <= MAX_BOUNCES; bounce += 1u) {
        if (bounce == MAX_BOUNCES) {
            // Set ray color as black if the ray has bounced too many times
            ray_color = vec4f(0.0);
            break;
        }

        let hit_info: HitInfo = trace_triangles(ray);

        if (hit_info.did_hit) {
            let tri_color = get_interpolated_color(hit_info);
            ray_color += tri_color * ray_throughput;
            ray_throughput *= tri_color;

            // Use frame number in random seed for temporal variation
            ray = create_ray(
                hit_info.p + hit_info.normal * 0.001, // Move the origin slightly to avoid self-intersection
                random_in_hemisphere(hit_info.normal, hit_info.p + vec3f(f32(frame_count) * 0.1))
            );
        } else {
            if (bounce == 0u) {
                // If ray misses all triangles, return the sky color
                ray_color = get_sky_color(ray);
            } else {
                let ray_sun_intensity = max(0.0, dot(sun_direction, ray.direction)) * SUN_INTENSITY;
                ray_color *= ray_sun_intensity * ray_throughput;
            }

            break;
        }
    }

    ray_color = clamp(ray_color, vec4f(0.0), vec4f(1.0));

    // Blend with previous frame
    let blend_factor = 1.0 / pow(f32(frame_count + 1), 1.05);
    let final_color = mix(prev_color, ray_color, blend_factor);

    textureStore(result, coords, final_color);

    // textureStore(result, coords, get_ray_color(ray)); // Used to debug the camera
}
