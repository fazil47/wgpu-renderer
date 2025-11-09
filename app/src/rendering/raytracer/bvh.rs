use std::cmp::Ordering;

use maths::Vec3;

use super::RaytracerVertex;

pub const BVH_LEAF_SIZE: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BvhNode {
    pub bounds_min: [f32; 4],
    pub bounds_max: [f32; 4],
    pub left_child: u32,
    pub right_child: u32,
    pub first_primitive: u32,
    pub primitive_count: u32,
}

#[derive(Debug, Default)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    pub primitive_indices: Vec<u32>,
}

impl Bvh {
    fn empty() -> Self {
        Self::default()
    }
}

struct BvhPrimitive {
    index: u32,
    bounds_min: Vec3,
    bounds_max: Vec3,
    centroid: Vec3,
}

impl BvhPrimitive {
    fn from_triangle(triangle_index: u32, vertices: &[RaytracerVertex], indices: &[u32]) -> Self {
        let base = triangle_index as usize * 3;
        let i0 = indices[base] as usize;
        let i1 = indices[base + 1] as usize;
        let i2 = indices[base + 2] as usize;

        let p0 = vertex_position(&vertices[i0]);
        let p1 = vertex_position(&vertices[i1]);
        let p2 = vertex_position(&vertices[i2]);

        let mut bounds_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut bounds_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        bounds_min = vec3_min(bounds_min, p0);
        bounds_min = vec3_min(bounds_min, p1);
        bounds_min = vec3_min(bounds_min, p2);

        bounds_max = vec3_max(bounds_max, p0);
        bounds_max = vec3_max(bounds_max, p1);
        bounds_max = vec3_max(bounds_max, p2);

        let centroid = (bounds_min + bounds_max) * 0.5;

        Self {
            index: triangle_index,
            bounds_min,
            bounds_max,
            centroid,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    fn grow_with(&mut self, bounds: &Aabb) {
        self.min = vec3_min(self.min, bounds.min);
        self.max = vec3_max(self.max, bounds.max);
    }

    fn from_primitive(primitive: &BvhPrimitive) -> Self {
        Self {
            min: primitive.bounds_min,
            max: primitive.bounds_max,
        }
    }
}

pub fn build_bvh(vertices: &[RaytracerVertex], indices: &[u32]) -> Bvh {
    if indices.is_empty() {
        return Bvh::empty();
    }

    debug_assert_eq!(indices.len() % 3, 0);
    let triangle_count = indices.len() / 3;
    let mut primitives = Vec::with_capacity(triangle_count);
    for triangle_index in 0..triangle_count {
        primitives.push(BvhPrimitive::from_triangle(
            triangle_index as u32,
            vertices,
            indices,
        ));
    }

    // The number of nodes in the worst case (when BvhNode::primitive_count for each leaf is 1)
    // is 2 * triangle_count - 1, because only the leaf nodes store triangles.
    let mut nodes = Vec::with_capacity(triangle_count * 2);
    let mut primitive_indices = Vec::with_capacity(triangle_count);
    build_bvh_recursive(&mut primitives, &mut nodes, &mut primitive_indices);

    Bvh {
        nodes,
        primitive_indices,
    }
}

fn build_bvh_recursive(
    primitives: &mut [BvhPrimitive],
    nodes: &mut Vec<BvhNode>,
    primitive_indices: &mut Vec<u32>,
) -> u32 {
    let node_index = nodes.len() as u32;
    nodes.push(BvhNode::default());

    let mut bounds = Aabb::empty();
    for primitive in primitives.iter() {
        bounds.grow_with(&Aabb::from_primitive(primitive));
    }

    if primitives.len() <= BVH_LEAF_SIZE {
        let first_primitive = primitive_indices.len() as u32;
        for primitive in primitives.iter() {
            primitive_indices.push(primitive.index);
        }

        nodes[node_index as usize] = BvhNode::new_leaf(bounds, first_primitive, primitives.len());
        return node_index;
    }

    // Choose the axis with the largest extent to split along
    let extent = bounds.max - bounds.min;
    let mut axis = 0;
    if extent.y > extent.x {
        axis = 1;
    }
    if extent.z > extent_component(extent, axis) {
        axis = 2;
    }

    let mid = primitives.len() / 2;
    primitives.select_nth_unstable_by(mid, |a, b| {
        let a_axis = centroid_component(a.centroid, axis);
        let b_axis = centroid_component(b.centroid, axis);
        a_axis.partial_cmp(&b_axis).unwrap_or(Ordering::Equal)
    });

    let (left, right) = primitives.split_at_mut(mid);

    // This will only happen when the length of primitives is 1 (first index, mid and last index all 0)
    // or 2 (first index is 0, mid and last index both 1). But in practice, that should not happen due to the
    // leaf size check above.
    if left.is_empty() || right.is_empty() {
        let first_primitive = primitive_indices.len() as u32;
        for primitive in primitives.iter() {
            primitive_indices.push(primitive.index);
        }
        nodes[node_index as usize] = BvhNode::new_leaf(bounds, first_primitive, primitives.len());
        return node_index;
    }

    let left_child = build_bvh_recursive(left, nodes, primitive_indices);
    let right_child = build_bvh_recursive(right, nodes, primitive_indices);

    nodes[node_index as usize] = BvhNode::new_interior(bounds, left_child, right_child);
    node_index
}

impl BvhNode {
    fn new_leaf(bounds: Aabb, first_primitive: u32, primitive_count: usize) -> Self {
        Self {
            bounds_min: padded(bounds.min),
            bounds_max: padded(bounds.max),
            left_child: u32::MAX,
            right_child: u32::MAX,
            first_primitive,
            primitive_count: primitive_count as u32,
        }
    }

    fn new_interior(bounds: Aabb, left_child: u32, right_child: u32) -> Self {
        Self {
            bounds_min: padded(bounds.min),
            bounds_max: padded(bounds.max),
            left_child,
            right_child,
            first_primitive: 0,
            primitive_count: 0,
        }
    }
}

fn padded(vec: Vec3) -> [f32; 4] {
    [vec.x, vec.y, vec.z, 0.0]
}

fn centroid_component(vec: Vec3, axis: usize) -> f32 {
    match axis {
        0 => vec.x,
        1 => vec.y,
        _ => vec.z,
    }
}

fn extent_component(vec: Vec3, axis: usize) -> f32 {
    match axis {
        0 => vec.x,
        1 => vec.y,
        _ => vec.z,
    }
}

fn vec3_min(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
}

fn vec3_max(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
}

fn vertex_position(vertex: &RaytracerVertex) -> Vec3 {
    Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2])
}
