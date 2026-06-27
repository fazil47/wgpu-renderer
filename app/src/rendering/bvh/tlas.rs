use maths::Vec3;

use crate::rendering::{
    BlasBvh,
    bvh::{Aabb, Bvh, BvhPrimitive, build_bvh},
    mesh::MeshBuffers,
};

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
