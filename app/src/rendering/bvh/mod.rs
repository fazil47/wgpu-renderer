use maths::{Mat4, Vec3};
use std::cmp::Ordering;

use crate::rendering::GpuVertex;

pub mod blas;
pub mod debug;
pub mod tlas;

pub const BVH_LEAF_SIZE: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhNode {
    pub bounds_min: [f32; 4],
    pub bounds_max: [f32; 4],
    pub left_child: u32,
    pub right_child: u32,
    pub first_primitive: u32,
    pub primitive_count: u32,
}

impl BvhNode {
    fn new_placeholder() -> Self {
        Self::default()
    }

    fn new_leaf(bounds: Aabb, first_primitive: u32, primitive_count: usize) -> Self {
        Self {
            bounds_min: bounds.min.to_array4(),
            bounds_max: bounds.max.to_array4(),
            left_child: u32::MAX,
            right_child: u32::MAX,
            first_primitive,
            primitive_count: primitive_count as u32,
        }
    }

    fn new_interior(bounds: Aabb, left_child: u32, right_child: u32) -> Self {
        Self {
            bounds_min: bounds.min.to_array4(),
            bounds_max: bounds.max.to_array4(),
            left_child,
            right_child,
            first_primitive: 0,
            primitive_count: 0,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.left_child == u32::MAX && self.right_child == u32::MAX
    }
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

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn grow_with(&mut self, bounds: &Aabb) {
        self.min = Vec3::min(self.min, bounds.min);
        self.max = Vec3::max(self.max, bounds.max);
    }

    pub fn transform(&self, transform: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for corner in corners {
            let world4 = transform * corner.extend(1.0);
            let world = Vec3::new(world4.x, world4.y, world4.z);
            min = Vec3::min(min, world);
            max = Vec3::max(max, world);
        }

        Self { min, max }
    }

    fn get_longest_axis(&self) -> Axis {
        let extent = self.max - self.min;
        let mut axis = Axis::X;

        if extent.y > extent.x {
            axis = Axis::Y;
        }

        if extent.z > axis.get_vector_component(&extent) {
            axis = Axis::Z;
        }

        axis
    }
}

#[derive(Clone, Copy, Debug)]
struct BvhPrimitive {
    index: u32,
    aabb: Aabb,
    centroid: Vec3,
}

impl BvhPrimitive {
    fn from_triangle(triangle_index: u32, vertices: &[GpuVertex], indices: &[u32]) -> Self {
        let base = triangle_index as usize * 3;
        let i0 = indices[base] as usize;
        let i1 = indices[base + 1] as usize;
        let i2 = indices[base + 2] as usize;

        let p0 = vertices[i0].position();
        let p1 = vertices[i1].position();
        let p2 = vertices[i2].position();

        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        min = Vec3::min(min, p0);
        min = Vec3::min(min, p1);
        min = Vec3::min(min, p2);

        max = Vec3::max(max, p0);
        max = Vec3::max(max, p1);
        max = Vec3::max(max, p2);

        let aabb = Aabb::new(min, max);

        Self {
            index: triangle_index,
            aabb,
            centroid: (min + max) * 0.5,
        }
    }
}

enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn get_vector_component(&self, vector: &Vec3) -> f32 {
        match &self {
            Axis::X => vector.x,
            Axis::Y => vector.y,
            Axis::Z => vector.z,
        }
    }
}

fn build_bvh(mut primitives: Vec<BvhPrimitive>) -> Bvh {
    // The number of nodes in the worst case (when BvhNode::primitive_count for each leaf is 1)
    // is 2 * primitives.len() - 1, because only the leaf nodes store primitives.
    let mut nodes = Vec::with_capacity(primitives.len() * 2);
    let mut primitive_indices = Vec::with_capacity(primitives.len());

    /// Creates the BVH recursively in pre-order (parent, left, right) by adding
    /// nodes to the mutable vector of nodes. Pass in an empty nodes vector and
    /// the root node will be the first element.
    fn recursive_build(
        primitives: &mut [BvhPrimitive],
        nodes: &mut Vec<BvhNode>,
        primitive_indices: &mut Vec<u32>,
    ) -> u32 {
        let node_index = nodes.len();
        nodes.push(BvhNode::new_placeholder());

        let mut bounds = Aabb::empty();
        for primitive in primitives.iter() {
            bounds.grow_with(&primitive.aabb);
        }

        if primitives.len() <= BVH_LEAF_SIZE {
            let first_primitive = primitive_indices.len() as u32;
            for primitive in primitives.iter() {
                primitive_indices.push(primitive.index);
            }

            nodes[node_index] = BvhNode::new_leaf(bounds, first_primitive, primitives.len());
            return node_index as u32;
        }

        // Choose the axis with the largest extent to split along
        let axis = bounds.get_longest_axis();

        let mid = primitives.len() / 2;
        primitives.select_nth_unstable_by(mid, |a, b| {
            let a_axis = axis.get_vector_component(&a.centroid);
            let b_axis = axis.get_vector_component(&b.centroid);
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

            nodes[node_index] = BvhNode::new_leaf(bounds, first_primitive, primitives.len());
            return node_index as u32;
        }

        let left_child = recursive_build(left, nodes, primitive_indices);
        let right_child = recursive_build(right, nodes, primitive_indices);

        nodes[node_index] = BvhNode::new_interior(bounds, left_child, right_child);
        node_index as u32
    }

    recursive_build(&mut primitives, &mut nodes, &mut primitive_indices);

    Bvh {
        nodes,
        primitive_indices,
    }
}
