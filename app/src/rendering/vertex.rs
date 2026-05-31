use std::mem::size_of;

use crate::mesh::Vertex;
use maths::Mat4;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub material_index: f32,
}

impl GpuVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn position(&self) -> maths::Vec3 {
        maths::Vec3::new(self.position[0], self.position[1], self.position[2])
    }

    pub fn from_vertex(vertex: &Vertex, material_index: usize, transform: Mat4) -> Self {
        let position = transform * vertex.position;
        let normal = transform * vertex.normal;
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            material_index: material_index as f32,
        }
    }
}

impl From<Vertex> for GpuVertex {
    fn from(val: Vertex) -> Self {
        GpuVertex {
            position: val.position.to_array(),
            normal: val.normal.to_array(),
            material_index: 0.0,
        }
    }
}
