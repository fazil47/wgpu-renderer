use std::cmp::Ordering;

use maths::{Mat4, Vec3};

use crate::rendering::GpuVertex;
use crate::rendering::mesh::MeshBuffers;

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

    pub fn is_leaf(&self) -> bool {
        self.left_child == u32::MAX && self.right_child == u32::MAX
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

fn build_bvh(mut primitives: Vec<BvhPrimitive>) -> Bvh {
    // The number of nodes in the worst case (when BvhNode::primitive_count for each leaf is 1)
    // is 2 * primitives.len() - 1, because only the leaf nodes store primitives.
    let mut nodes = Vec::with_capacity(primitives.len() * 2);
    let mut primitive_indices = Vec::with_capacity(primitives.len());

    fn recursive_build(
        primitives: &mut [BvhPrimitive],
        nodes: &mut Vec<BvhNode>,
        primitive_indices: &mut Vec<u32>,
    ) -> u32 {
        // TODO: This function is confusing because a node is first pushed and
        // then overwritten, but without first inserting the raytracer breaks
        let node_index = nodes.len() as u32;
        nodes.push(BvhNode::default());

        let mut bounds = Aabb::empty();
        for primitive in primitives.iter() {
            bounds.grow_with(&primitive.aabb);
        }

        if primitives.len() <= BVH_LEAF_SIZE {
            let first_primitive = primitive_indices.len() as u32;
            for primitive in primitives.iter() {
                primitive_indices.push(primitive.index);
            }

            nodes[node_index as usize] =
                BvhNode::new_leaf(bounds, first_primitive, primitives.len());
            return node_index;
        }

        // TODO: There has to be a clearer way of choosing the axis with the
        // largest extent

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
            nodes[node_index as usize] =
                BvhNode::new_leaf(bounds, first_primitive, primitives.len());
            return node_index;
        }

        let left_child = recursive_build(left, nodes, primitive_indices);
        let right_child = recursive_build(right, nodes, primitive_indices);

        nodes[node_index as usize] = BvhNode::new_interior(bounds, left_child, right_child);
        node_index
    }

    recursive_build(&mut primitives, &mut nodes, &mut primitive_indices);

    Bvh {
        nodes,
        primitive_indices,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BvhDebugLine {
    pub start: Vec3,
    pub end: Vec3,
    pub is_leaf: bool,
}

pub fn build_bvh_debug_lines(bvh: &Bvh) -> Vec<BvhDebugLine> {
    let mut lines = Vec::new();
    if bvh.nodes.is_empty() {
        return lines;
    }

    const EDGE_INDICES: [(usize, usize); 12] = [
        (0, 1), // bottom face: x edges
        (1, 3), // bottom face: diagonal advancing in y
        (3, 2), // bottom face: x opposite edge
        (2, 0), // bottom face: closing the loop
        (4, 5), // top face: x edges
        (5, 7), // top face: diagonal advancing in y
        (7, 6), // top face: x opposite edge
        (6, 4), // top face: closing the loop
        (0, 4), // pillars connecting bottom to top (x min, y min)
        (1, 5), // pillars (x max, y min)
        (2, 6), // pillars (x min, y max)
        (3, 7), // pillars (x max, y max)
    ];

    for node in &bvh.nodes {
        let min = Vec3::new(node.bounds_min[0], node.bounds_min[1], node.bounds_min[2]);
        let max = Vec3::new(node.bounds_max[0], node.bounds_max[1], node.bounds_max[2]);

        if !min.x.is_finite()
            || !min.y.is_finite()
            || !min.z.is_finite()
            || !max.x.is_finite()
            || !max.y.is_finite()
            || !max.z.is_finite()
        {
            continue;
        }

        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
        ];

        for &(start, end) in EDGE_INDICES.iter() {
            lines.push(BvhDebugLine {
                start: corners[start],
                end: corners[end],
                is_leaf: node.is_leaf(),
            });
        }
    }

    lines
}

pub fn build_blas(vertices: &[GpuVertex], indices: &[u32]) -> Bvh {
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

    build_bvh(primitives)
}

pub fn build_tlas(bounds: &[Aabb]) -> Bvh {
    if bounds.is_empty() {
        return Bvh::default();
    }

    let primitives: Vec<BvhPrimitive> = bounds
        .iter()
        .enumerate()
        .map(|(index, aabb)| BvhPrimitive {
            index: index as u32,
            aabb: *aabb,
            centroid: (aabb.min + aabb.max) * 0.5,
        })
        .collect();

    build_bvh(primitives)
}

// Since there will be one BLAS per unique mesh, we need to store the BLAS info for each of those meshes
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlasInfo {
    pub node_offset: u32,
    pub node_count: u32,
    pub primitive_offset: u32,
    pub primitive_count: u32,
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub _padding: [u32; 2],
}

impl BlasInfo {
    pub fn new(
        node_offset: u32,
        node_count: u32,
        primitive_offset: u32,
        primitive_count: u32,
        vertex_offset: u32,
        index_offset: u32,
    ) -> Self {
        Self {
            node_offset,
            node_count,
            primitive_offset,
            primitive_count,
            vertex_offset,
            index_offset,
            _padding: [0; 2],
        }
    }
}

/// CPU-side TLAS (Top-Level Acceleration Structure) BVH resource
/// Stores the scene's bounding volume hierarchy for quick access
#[derive(Debug, Default)]
pub struct TlasBvh {
    pub bvh: Bvh,
}

impl ecs::Resource for TlasBvh {}

impl TlasBvh {
    pub fn new(bvh: Bvh) -> Self {
        Self { bvh }
    }
}

/// CPU-side BLAS (Bottom-Level Acceleration Structure) BVH resource
/// Caches the per-mesh BVH data so it doesn't need to be rebuilt when only transforms change
#[derive(Debug, Default)]
pub struct BlasBvh {
    pub nodes: Vec<BvhNode>,
    pub primitive_indices: Vec<u32>,
    pub infos: Vec<BlasInfo>,
    /// Per-mesh BVH objects, used for debug line generation
    pub per_mesh_bvhs: Vec<Bvh>,
}

impl ecs::Resource for BlasBvh {}

impl BlasBvh {
    /// Append BVH data for a single mesh at the given offset in `MeshBuffers`.
    pub fn add_mesh(&mut self, mesh_offset: usize, mesh_buffers: &MeshBuffers) {
        let gpu_mesh = &mesh_buffers.meshes[mesh_offset];

        // Pad with empty entries if needed so infos stays 1:1 with meshes
        while self.infos.len() <= mesh_offset {
            self.infos.push(BlasInfo::default());
            self.per_mesh_bvhs.push(Bvh::default());
        }

        if gpu_mesh.index_count == 0 {
            return;
        }

        let mesh_vertices = &mesh_buffers.vertices[gpu_mesh.vertex_offset as usize
            ..(gpu_mesh.vertex_offset + gpu_mesh.vertex_count) as usize];
        let mesh_indices = &mesh_buffers.indices[gpu_mesh.index_offset as usize
            ..(gpu_mesh.index_offset + gpu_mesh.index_count) as usize];

        let blas = build_blas(mesh_vertices, mesh_indices);
        let node_offset = self.nodes.len() as u32;
        let node_count = blas.nodes.len() as u32;
        let primitive_offset = self.primitive_indices.len() as u32;
        let primitive_count = blas.primitive_indices.len() as u32;
        self.nodes.extend_from_slice(&blas.nodes);
        self.primitive_indices
            .extend_from_slice(&blas.primitive_indices);

        self.infos[mesh_offset] = BlasInfo::new(
            node_offset,
            node_count,
            primitive_offset,
            primitive_count,
            gpu_mesh.vertex_offset,
            gpu_mesh.index_offset,
        );
        self.per_mesh_bvhs[mesh_offset] = blas;
    }

    /// Mark a mesh slot as removed by zeroing its counts.
    /// The node/primitive data is left as a hole — no compaction.
    pub fn remove_mesh(&mut self, mesh_offset: usize) {
        if mesh_offset < self.infos.len() {
            self.infos[mesh_offset].node_count = 0;
            self.infos[mesh_offset].primitive_count = 0;
        }
    }
}

/// Gathers world-space AABBs for each mesh instance.
fn compute_instance_bounds(mesh_buffers: &MeshBuffers, blas: &BlasBvh) -> Vec<Aabb> {
    let mut instance_bounds = Vec::new();
    for (gpu_mesh, blas_info) in mesh_buffers.meshes.iter().zip(blas.infos.iter()) {
        if gpu_mesh.index_count == 0 {
            continue;
        }

        let bounds = if blas_info.node_count == 0 {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            let node = &blas.nodes[blas_info.node_offset as usize];
            (
                Vec3::new(node.bounds_min[0], node.bounds_min[1], node.bounds_min[2]),
                Vec3::new(node.bounds_max[0], node.bounds_max[1], node.bounds_max[2]),
            )
        };
        let aabb = Aabb::new(bounds.0, bounds.1).transform(gpu_mesh.transform);
        instance_bounds.push(aabb);
    }

    instance_bounds
}

/// Manages the in-flight TLAS calculation running on a background thread.
/// On wasm32, threads are not available so the build runs synchronously.
#[derive(Default)]
pub struct TlasBuildTask {
    #[cfg(not(target_arch = "wasm32"))]
    in_flight: Option<std::thread::JoinHandle<TlasBvh>>,
    #[cfg(not(target_arch = "wasm32"))]
    queued: Option<Vec<Aabb>>,
    #[cfg(target_arch = "wasm32")]
    result: Option<TlasBvh>,
}

impl ecs::Resource for TlasBuildTask {}

impl TlasBuildTask {
    /// Returns `true` if a background build has finished.
    pub fn is_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.in_flight.as_ref().is_some_and(|h| h.is_finished())
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.result.is_some()
        }
    }

    /// Returns `true` if a build is currently in flight.
    pub fn is_building(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.in_flight.is_some()
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.result.is_some()
        }
    }

    /// Takes the completed result and spawns the queued build if any.
    /// Only call after `is_finished()` returns `true`.
    pub fn take_result(&mut self) -> TlasBvh {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let result = self
                .in_flight
                .take()
                .expect("No pending TLAS build")
                .join()
                .expect("TLAS build thread panicked");

            if let Some(instance_bounds) = self.queued.take() {
                self.spawn(instance_bounds);
            }

            result
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.result.take().expect("No pending TLAS build")
        }
    }

    /// Requests a TLAS build. Spawns immediately if idle, otherwise queues the data.
    pub fn request_build(&mut self, mesh_buffers: &MeshBuffers, blas: &BlasBvh) {
        let instance_bounds = compute_instance_bounds(mesh_buffers, blas);

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.spawn(instance_bounds);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.result = Some(TlasBvh::new(build_tlas(&instance_bounds)));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(&mut self, instance_bounds: Vec<Aabb>) {
        if self.in_flight.is_some() {
            // TODO: Would be nice to calculate instance_bounds when needed and
            // not store it in self.queued
            self.queued = Some(instance_bounds);
            return;
        }

        self.in_flight = Some(
            std::thread::Builder::new()
                .name("tlas-build".into())
                .spawn(move || {
                    let tlas_bvh = build_tlas(&instance_bounds);
                    TlasBvh::new(tlas_bvh)
                })
                .expect("Failed to spawn TLAS build thread"),
        );
    }
}
