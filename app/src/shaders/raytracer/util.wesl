import runtime::constants::MATERIAL_STRIDE;
import runtime::constants::VERTEX_STRIDE;
import runtime::constants::VERTEX_NORMAL_OFFSET;
import runtime::constants::VERTEX_MATERIAL_OFFSET;

const MAX_BOUNCES: u32 = 3u;
const MAX_LOW_FRAME_BOUNCES: u32 = 2u;
const LOW_BOUNCE_FRAMES: u32 = 4u;
const CLEAR_COLOR: vec4f = vec4f(0.0, 0.0, 0.0, 1.0);
const FLT_MAX: f32 = 1e12;
const K_EPSILON: f32 = 1e-6;
const SUN_INTENSITY: f32 = 1.0;

struct BvhNode {
    bounds_min: vec4f,
    bounds_max: vec4f,
    left_child: u32,
    right_child: u32,
    first_primitive: u32,
    primitive_count: u32,
}

struct BlasInfo {
    node_offset: u32,       // Start index in the global 'blas_nodes' buffer
    node_count: u32,        // How many nodes this mesh's BVH has
    primitive_offset: u32,  // Start index in the global 'blas_primitive_indices' buffer
    primitive_count: u32,   // How many triangles this mesh has
    vertex_offset: u32,     // Start index in the global 'vertices' buffer
    index_offset: u32,      // Start index in the global 'indices' buffer
    _pad0: u32,
    _pad1: u32,
}

struct InstanceData {
    world_from_local_0: vec4f,
    world_from_local_1: vec4f,
    world_from_local_2: vec4f,
    world_from_local_3: vec4f,
    local_from_world_0: vec4f,
    local_from_world_1: vec4f,
    local_from_world_2: vec4f,
    local_from_world_3: vec4f,
    blas_index: u32,
    // We use 3 u32s instead of vec3u to avoid 16-byte alignment padding.
    // Rust struct is 144 bytes, vec3u would make this 160 bytes.
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> materials: array<f32>;

@group(1) @binding(0) var<storage, read> vertices: array<f32>;
@group(1) @binding(1) var<storage, read> indices: array<u32>;
@group(1) @binding(2) var<storage, read> blas_nodes: array<BvhNode>;
@group(1) @binding(3) var<storage, read> blas_primitive_indices: array<u32>;
@group(1) @binding(4) var<storage, read> blas_info: array<BlasInfo>;
@group(1) @binding(5) var<storage, read> tlas_nodes: array<BvhNode>;
@group(1) @binding(6) var<storage, read> instances: array<InstanceData>;
@group(1) @binding(7) var<storage, read> tlas_primitive_indices: array<u32>;

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
    instance_index: u32,
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

    // The ray is parallel to the triangle plane
    if abs(det) < K_EPSILON {
        hit_info.did_hit = false;
        return hit_info;
    }

    let inv_det: f32 = 1.0 / det;

    if det < 0.0 {
        hit_info.normal = -hit_info.normal;
    }

    let tvec: vec3f = ray.origin - triangle.c.position.xyz;
    let u: f32 = dot(tvec, pvec) * inv_det;
    if u < 0.0 || u > 1.0 {
        hit_info.did_hit = false;
        return hit_info;
    }

    let qvec: vec3f = cross(tvec, e0);
    let v: f32 = dot(ray.direction, qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
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

fn get_sky_color(ray: Ray) -> vec4f {
    var sun_intensity = max(0.0, dot(sun_direction, ray.direction));
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

// Gets the triangle at index `primitive_index` of the mesh's triangle list
// and retrieves its data from the global triangles data.
fn get_triangle_data(mesh: BlasInfo, primitive_index: u32) -> Triangle {
    let triangle_index = blas_primitive_indices[mesh.primitive_offset + primitive_index];
    let tri_index = mesh.index_offset + triangle_index * 3u;

    var triangle: Triangle;
    triangle.a = get_vertex(indices[tri_index + 0u], mesh);
    triangle.b = get_vertex(indices[tri_index + 1u], mesh);
    triangle.c = get_vertex(indices[tri_index + 2u], mesh);

    return triangle;
}

fn get_vertex(index: u32, mesh: BlasInfo) -> Vertex {
    let vertex_index = mesh.vertex_offset + index;
    let base = vertex_index * VERTEX_STRIDE;

    var vertex: Vertex;
    vertex.position = vec4f(
        vertices[base + 0u],
        vertices[base + 1u],
        vertices[base + 2u],
        vertices[base + 3u],
    );
    vertex.normal = vec4f(
        vertices[base + VERTEX_NORMAL_OFFSET + 0u],
        vertices[base + VERTEX_NORMAL_OFFSET + 1u],
        vertices[base + VERTEX_NORMAL_OFFSET + 2u],
        vertices[base + VERTEX_NORMAL_OFFSET + 3u],
    );
    let material_index: u32 = u32(vertices[base + VERTEX_MATERIAL_OFFSET]);
    let material = get_material(material_index);
    vertex.color = material.color;
    return vertex;
}

fn get_material(index: u32) -> Material {
    var material: Material;
    material.color = vec4f(
        materials[index * MATERIAL_STRIDE + 0u],
        materials[index * MATERIAL_STRIDE + 1u],
        materials[index * MATERIAL_STRIDE + 2u],
        materials[index * MATERIAL_STRIDE + 3u],
    );
    return material;
}

fn intersects_aabb(ray: Ray, bounds_min: vec3f, bounds_max: vec3f, max_dist: f32) -> bool {
    var t_min = 0.0;
    var t_max = max_dist;

    for (var axis: u32 = 0u; axis < 3u; axis += 1u) {
        // For a ray, point = origin + direction * t
        // bounds_min[axis] = ray.origin[axis] + ray.direction[axis] * t0;
        // bounds_max[axis] = ray.origin[axis] + ray.direction[axis] * t1;

        let origin = ray.origin[axis];
        let direction = ray.direction[axis];
        let inv_dir = 1.0 / direction;
        var t0 = (bounds_min[axis] - origin) * inv_dir;
        var t1 = (bounds_max[axis] - origin) * inv_dir;

        if t0 > t1 {
            let tmp = t0;
            t0 = t1;
            t1 = tmp;
        }

        t_min = max(t_min, t0);
        t_max = min(t_max, t1);

        if t_max <= t_min {
            return false;
        }
    }

    return true;
}

fn push_stack(stack: ptr<function, array<u32, 64u>>, stack_size: ptr<function, u32>, value: u32) {
    (*stack)[*stack_size] = value;
    *stack_size += 1u;
}

fn pop_stack(stack: ptr<function, array<u32, 64u>>, stack_size: ptr<function, u32>) -> u32 {
    *stack_size -= 1u;
    return (*stack)[*stack_size];
}

fn should_visit(node: BvhNode, ray: Ray, t_max: f32) -> bool {
    return intersects_aabb(ray, node.bounds_min.xyz, node.bounds_max.xyz, t_max);
}

fn push_children(
    node: BvhNode,
    node_offset: u32,
    stack: ptr<function, array<u32, 64u>>,
    stack_size: ptr<function, u32>
) {
    if node.left_child != 0xffffffffu {
        push_stack(stack, stack_size, node_offset + node.left_child);
    }

    if node.right_child != 0xffffffffu {
        push_stack(stack, stack_size, node_offset + node.right_child);
    }
}

fn trace_blas(ray: Ray, mesh: BlasInfo, t_max: f32) -> HitInfo {
    var stack: array<u32, 64u>;
    var stack_size: u32 = 0u;

    if mesh.node_count == 0u || arrayLength(&blas_nodes) == 0u {
        return HitInfo(false, FLT_MAX, 0u, vec3f(0.0), vec3f(0.0), 0.0, 0.0, 0.0, 0u);
    }

    push_stack(&stack, &stack_size, mesh.node_offset);

    var nearest_hit_info: HitInfo;
    nearest_hit_info.did_hit = false;
    nearest_hit_info.t = t_max;

    while (stack_size > 0u) {
        let node_index = pop_stack(&stack, &stack_size);
        let node = blas_nodes[node_index];

        if !should_visit(node, ray, nearest_hit_info.t) {
            continue;
        }

        if node.primitive_count > 0u {
            // Leaf node, test primitives
            let start = node.first_primitive;
            let end = start + node.primitive_count;
            for (var i = start; i < end; i = i + 1u) {
                let triangle: Triangle = get_triangle_data(mesh, i);
                let hit_info: HitInfo = get_triangle_intersection_mt(triangle, ray);

                if hit_info.did_hit && hit_info.t < nearest_hit_info.t {
                    nearest_hit_info = hit_info;
                    nearest_hit_info.i = i;
                }
            }
        } else {
            // Interior node, push children
            push_children(node, mesh.node_offset, &stack, &stack_size);
        }
    }

    return nearest_hit_info;
}

fn trace_tlas(ray_world: Ray) -> HitInfo {
    var stack: array<u32, 64u>;
    var stack_size: u32 = 0u;

    if arrayLength(&tlas_nodes) == 0u {
        return HitInfo(false, FLT_MAX, 0u, vec3f(0.0), vec3f(0.0), 0.0, 0.0, 0.0, 0u);
    }

    push_stack(&stack, &stack_size, 0u);

    var nearest_hit_info = HitInfo(false, FLT_MAX, 0u, vec3f(0.0), vec3f(0.0), 0.0, 0.0, 0.0, 0u);

    while (stack_size > 0u) {
        let node_index = pop_stack(&stack, &stack_size);
        let node = tlas_nodes[node_index];

        if !should_visit(node, ray_world, nearest_hit_info.t) {
            continue;
        }

        if node.primitive_count > 0u {
            // Leaf node, test primitives
            let start = node.first_primitive;
            let end = start + node.primitive_count;
            for (var i = start; i < end; i = i + 1u) {
                let instance_index = tlas_primitive_indices[i];
                let instance = instances[instance_index];
                let mesh = blas_info[instance.blas_index];
                if mesh.primitive_count == 0u {
                    continue;
                }

                let local_from_world = mat4x4<f32>(
                    instance.local_from_world_0,
                    instance.local_from_world_1,
                    instance.local_from_world_2,
                    instance.local_from_world_3,
                );
                let world_from_local = mat4x4<f32>(
                    instance.world_from_local_0,
                    instance.world_from_local_1,
                    instance.world_from_local_2,
                    instance.world_from_local_3,
                );

                let origin_local = (local_from_world * vec4f(ray_world.origin, 1.0)).xyz;
                let direction_local = (local_from_world * vec4f(ray_world.direction, 0.0)).xyz;
                let local_ray = create_ray(origin_local, normalize(direction_local));

                let hit_local = trace_blas(local_ray, mesh, nearest_hit_info.t);
                if hit_local.did_hit {
                    let world_hit = (world_from_local * vec4f(hit_local.p, 1.0)).xyz;
                    let world_normal = normalize((vec4f(hit_local.normal, 0.0) * local_from_world).xyz);
                    let world_t = length(world_hit - ray_world.origin);

                    if world_t < nearest_hit_info.t {
                        nearest_hit_info = HitInfo(
                            true,
                            world_t,
                            hit_local.i,
                            world_hit,
                            world_normal,
                            hit_local.u,
                            hit_local.v,
                            hit_local.w,
                            instance_index
                        );
                    }
                }
            }
        } else {
            // Interior node, push children
            push_children(node, 0u, &stack, &stack_size);
        }
    }

    return nearest_hit_info;
}

fn get_interpolated_color_from_hit(hit_info: HitInfo, mesh: BlasInfo) -> vec4f {
    let triangle: Triangle = get_triangle_data(mesh, hit_info.i);
    return vec4f(get_interpolated_color(hit_info, triangle), 1.0);
}

fn sample_radiance_with_lighting(origin: vec3f, direction: vec3f, max_bounces: u32, bounce_seed: f32) -> vec4f {
    var ray = create_ray(origin, direction);
    var ray_color = vec4f(0.0);
    var ray_throughput = vec4f(1.0);

    for (var bounce = 0u; bounce <= max_bounces; bounce += 1u) {
        if bounce == max_bounces {
            // Set ray color as black if the ray has bounced too many times
            ray_color = vec4f(0.0);
            break;
        }

        let hit_info: HitInfo = trace_tlas(ray);
        if hit_info.did_hit {
            let mesh = blas_info[instances[hit_info.instance_index].blas_index];
            let tri_color = get_interpolated_color_from_hit(hit_info, mesh);
            ray_color += tri_color * ray_throughput;
            ray_throughput *= tri_color;

            ray = create_ray(
                hit_info.p + hit_info.normal * 0.001, // Move the origin slightly to avoid self-intersection
                random_in_hemisphere(hit_info.normal, hit_info.p + bounce_seed)
            );
        } else {
            if bounce == 0u {
                // If ray misses all triangles, return the sky color
                ray_color = get_sky_color(ray);
            } else {
                let ray_sun_intensity = max(0.0, dot(sun_direction, ray.direction)) * SUN_INTENSITY;
                ray_color *= ray_sun_intensity * ray_throughput;
            }

            break;
        }
    }

    return clamp(ray_color, vec4f(0.0), vec4f(1.0));
}
