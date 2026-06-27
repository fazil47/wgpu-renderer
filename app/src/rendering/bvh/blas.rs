use crate::rendering::{
    GpuVertex,
    bvh::{Bvh, BvhNode, BvhPrimitive, build_bvh},
    mesh::MeshBuffers,
};

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
