use wgpu::util::DeviceExt;

use crate::wgpu::{Index, Vertex};

pub trait Mesh {
    fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn get_index_count(&self) -> u32;
    fn get_vertices(&self) -> &[Vertex];
    fn get_indices(&self) -> &[Index];
}

pub trait CombinedMeshExt {
    fn create_vertices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
}

impl CombinedMeshExt for Vec<Box<dyn Mesh>> {
    fn create_vertices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let all_vertices: Vec<Vertex> = self
            .iter()
            .flat_map(|mesh| mesh.get_vertices())
            .cloned()
            .collect();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut all_indices: Vec<Index> = Vec::new();
        let mut vertices_offset: u32 = 0;

        for mesh in self {
            let mesh_indices = mesh.get_indices();

            // Add indices with the vertices offset
            all_indices.extend(mesh_indices.iter().map(|&index| index + vertices_offset));

            // Update vertices offset for the next mesh
            vertices_offset += mesh.get_vertices().len() as u32;
        }

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Combined Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }
}

pub mod gltf;
pub mod ply;
pub mod static_mesh;
