use maths::Vec3;

use crate::rendering::mesh::MeshBuffers;
use crate::rendering::raytracer::RaytracerBlasInfo;
use crate::rendering::raytracer::bvh::{Aabb, Bvh, BvhNode, build_blas, build_tlas};

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
    pub infos: Vec<RaytracerBlasInfo>,
    /// Per-mesh BVH objects, used for debug line generation
    pub per_mesh_bvhs: Vec<Bvh>,
}

impl ecs::Resource for BlasBvh {}

/// Builds the BLAS (per-mesh, object-space BVH) from the mesh arena.
pub fn build_scene_blas(mesh_buffers: &MeshBuffers) -> BlasBvh {
    let mut blas_nodes = Vec::new();
    let mut blas_primitive_indices = Vec::new();
    let mut blas_infos = Vec::new();
    let mut per_mesh_bvhs = Vec::new();

    for gpu_mesh in &mesh_buffers.meshes {
        let mesh_vertices = &mesh_buffers.vertices[gpu_mesh.vertex_offset as usize
            ..(gpu_mesh.vertex_offset + gpu_mesh.vertex_count) as usize];
        let mesh_indices = &mesh_buffers.indices[gpu_mesh.index_offset as usize
            ..(gpu_mesh.index_offset + gpu_mesh.index_count) as usize];

        let blas = build_blas(mesh_vertices, mesh_indices);
        let node_offset = blas_nodes.len() as u32;
        let node_count = blas.nodes.len() as u32;
        let primitive_offset = blas_primitive_indices.len() as u32;
        let primitive_count = blas.primitive_indices.len() as u32;
        blas_nodes.extend_from_slice(&blas.nodes);
        blas_primitive_indices.extend_from_slice(&blas.primitive_indices);

        blas_infos.push(RaytracerBlasInfo::new(
            node_offset,
            node_count,
            primitive_offset,
            primitive_count,
            gpu_mesh.vertex_offset,
            gpu_mesh.index_offset,
        ));

        per_mesh_bvhs.push(blas);
    }

    BlasBvh {
        nodes: blas_nodes,
        primitive_indices: blas_primitive_indices,
        infos: blas_infos,
        per_mesh_bvhs,
    }
}

/// Builds the TLAS (world-space BVH over instances) using existing BLAS bounds
/// and current mesh transforms.
pub fn build_scene_tlas(mesh_buffers: &MeshBuffers, blas: &BlasBvh) -> TlasBvh {
    let mut instance_bounds: Vec<Aabb> = Vec::new();
    for (gpu_mesh, blas_info) in mesh_buffers.meshes.iter().zip(blas.infos.iter()) {
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

    let tlas_bvh = build_tlas(&instance_bounds);
    TlasBvh::new(tlas_bvh)
}
