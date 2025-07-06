const MAX_BOUNCES: u32 = 8u;
const MAX_LOW_FRAME_BOUNCES: u32 = 2u;
const LOW_BOUNCE_FRAMES: u32 = 4u;
const CLEAR_COLOR: vec4f = vec4f(0.0, 0.0, 0.0, 1.0);
const FLT_MAX: f32 = 1e12;
const K_EPSILON: f32 = 1e-6;
const SUN_INTENSITY: f32 = 1.0;

@group(0) @binding(0) var<storage, read> materials: array<f32>;
@group(0) @binding(1) var<uniform> material_stride: u32;

@group(1) @binding(0) var<storage, read> vertices: array<f32>;
@group(1) @binding(1) var<uniform> vertex_stride: u32;
@group(1) @binding(2) var<uniform> vertex_normal_offset: u32;
@group(1) @binding(3) var<uniform> vertex_material_offset: u32;
@group(1) @binding(4) var<storage, read> indices: array<u32>;

@group(2) @binding(0) var<uniform> sun_direction: vec3f;

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

struct Ray {
    origin: vec3f,
    direction: vec3f,
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

struct SceneParams {
    material_stride: u32,
    vertex_stride: u32,
    vertex_normal_offset: u32,
    vertex_material_offset: u32,
    sun_direction: vec3f,
}

fn create_ray(origin: vec3f, direction: vec3f) -> Ray {
    var ray: Ray;
    ray.origin = origin;
    ray.direction = direction;
    return ray;
}

fn get_triangle_intersection_mt(triangle: Triangle, ray: Ray) -> HitInfo {
    var hit_info: HitInfo;

    let e0: vec3f = triangle.a.position.xyz - triangle.c.position.xyz;
    let e1: vec3f = triangle.b.position.xyz - triangle.c.position.xyz;
    let pvec: vec3f = cross(ray.direction, e1);
    let det: f32 = dot(e0, pvec);

    hit_info.normal = normalize(cross(e0, e1));

    if det < K_EPSILON {
        hit_info.did_hit = false;
        return hit_info;
    }

    if abs(det) < K_EPSILON {
        hit_info.did_hit = false;
        return hit_info;
    }

    let inv_det: f32 = 1.0 / det;

    let tvec: vec3f = ray.origin - triangle.c.position.xyz;
    let u: f32 = dot(tvec, pvec) * inv_det;
    if u < 0 || u > 1 {
        hit_info.did_hit = false;
        return hit_info;
    }

    let qvec: vec3f = cross(tvec, e0);
    let v: f32 = dot(ray.direction, qvec) * inv_det;
    if v < 0 || u + v > 1 {
        hit_info.did_hit = false;
        return hit_info;
    }

    let t: f32 = dot(e1, qvec) * inv_det;

    if t < 0.001 {
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

fn get_interpolated_color(hit_info: HitInfo, triangle: Triangle) -> vec3f {
    let color = triangle.a.color * hit_info.u + triangle.b.color * hit_info.v + triangle.c.color * hit_info.w;
    return color.rgb;
}

fn get_sky_color(ray: Ray, params: SceneParams) -> vec4f {
    var sun_intensity = max(0.0, dot(params.sun_direction, ray.direction));
    sun_intensity = pow(sun_intensity, 32.0);
    return vec4f(sun_intensity, sun_intensity, sun_intensity, 1.0);
}

fn random_in_hemisphere(normal: vec3f, seed: vec3f) -> vec3f {
    let theta = 2.0 * 3.14159 * fract(sin(dot(seed, vec3f(12.9898, 78.233, 45.164))) * 43758.5453);
    let phi = acos(2.0 * fract(sin(dot(seed, vec3f(63.7264, 10.873, 98.234))) * 43758.5453) - 1.0);

    let x = sin(phi) * cos(theta);
    let y = sin(phi) * sin(theta);
    let z = cos(phi);
    let random_dir = normalize(vec3f(x, y, z));

    return normalize(normal + random_dir);
}

fn get_triangle_data(index: u32) -> Triangle {
    let params = SceneParams(
        material_stride,
        vertex_stride,
        vertex_normal_offset,
        vertex_material_offset,
        sun_direction,
    );
    var triangle: Triangle;
    triangle.a = get_vertex(indices[index * 3u + 0u], params);
    triangle.b = get_vertex(indices[index * 3u + 1u], params);
    triangle.c = get_vertex(indices[index * 3u + 2u], params);
    return triangle;
}

fn get_vertex(index: u32, params: SceneParams) -> Vertex {
    var vertex: Vertex;
    vertex.position = vec4f(
        vertices[index * params.vertex_stride + 0u],
        vertices[index * params.vertex_stride + 1u],
        vertices[index * params.vertex_stride + 2u],
        vertices[index * params.vertex_stride + 3u],
    );
    vertex.normal = vec4f(
        vertices[index * params.vertex_stride + params.vertex_normal_offset + 0u],
        vertices[index * params.vertex_stride + params.vertex_normal_offset + 1u],
        vertices[index * params.vertex_stride + params.vertex_normal_offset + 2u],
        vertices[index * params.vertex_stride + params.vertex_normal_offset + 3u],
    );
    let material_index: u32 = u32(vertices[index * params.vertex_stride + params.vertex_material_offset]);
    let material = get_material(material_index);
    vertex.color = material.color;
    return vertex;
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

fn sample_radiance_with_lighting(origin: vec3f, direction: vec3f, max_bounces: u32, bounce_seed: f32) -> vec4f {
    let scene_params = SceneParams(
        material_stride,
        vertex_stride,
        vertex_normal_offset,
        vertex_material_offset,
        sun_direction,
    );

    var ray = create_ray(origin, direction);
    var ray_color = vec4f(0.0);
    var ray_throughput = vec4f(1.0);

    for (var bounce = 0u; bounce <= max_bounces; bounce += 1u) {
        if bounce == max_bounces {
            // Set ray color as black if the ray has bounced too many times
            ray_color = vec4f(0.0);
            break;
        }

        let hit_info: HitInfo = trace_triangles(ray);
        if hit_info.did_hit {
            let tri_color = get_interpolated_color_from_hit(hit_info);
            ray_color += tri_color * ray_throughput;
            ray_throughput *= tri_color;

            ray = create_ray(
                hit_info.p + hit_info.normal * 0.001, // Move the origin slightly to avoid self-intersection
                random_in_hemisphere(hit_info.normal, hit_info.p + bounce_seed)
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

    return clamp(ray_color, vec4f(0.0), vec4f(1.0));
}
