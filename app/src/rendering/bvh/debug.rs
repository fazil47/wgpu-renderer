use maths::Vec3;

use crate::rendering::bvh::Bvh;

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
