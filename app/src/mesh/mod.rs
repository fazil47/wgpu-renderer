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
            .map(|vertex| vertex.clone())
            .collect();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let all_indices: Vec<Index> = self
            .iter()
            .flat_map(|mesh| mesh.get_indices())
            .map(|index| index.clone())
            .collect();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }
}

pub mod ply;
pub mod static_mesh;
