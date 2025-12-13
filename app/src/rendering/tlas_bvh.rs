use crate::rendering::raytracer::bvh::Bvh;

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
