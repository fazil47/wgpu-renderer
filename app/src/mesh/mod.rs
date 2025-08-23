use ecs::Component;
use maths::Vec4;

use crate::rendering::wgpu_utils::WgpuExt;
use crate::rendering::{
    rasterizer::GpuVertex,
    wgpu_utils::{Index, RGBA},
};

#[derive(Clone)]
pub struct Material {
    pub color: RGBA,
    pub meshes: Vec<Mesh>,
}

impl Material {
    pub fn new(color: RGBA) -> Self {
        Self {
            color,
            meshes: Vec::new(),
        }
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    fn get_vertices(&self) -> Vec<GpuVertex> {
        self.meshes
            .iter()
            .flat_map(|mesh| &mesh.vertices)
            .copied()
            .map(|v| v.into())
            .collect()
    }

    fn get_indices(&self) -> Vec<Index> {
        let mut indices = Vec::new();
        let mut vertices_offset: u32 = 0;

        for mesh in &self.meshes {
            let mesh_indices = &mesh.indices;

            // Add indices with the vertices offset
            indices.extend(mesh_indices.iter().map(|&index| index + vertices_offset));

            // Update vertices offset for the next mesh
            vertices_offset += mesh.vertices.len() as u32;
        }

        indices
    }

    pub fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device
            .buffer()
            .label("Vertex Buffer")
            .usage(wgpu::BufferUsages::VERTEX)
            .vertex(&self.get_vertices())
    }

    pub fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device
            .buffer()
            .label("Index Buffer")
            .usage(wgpu::BufferUsages::INDEX)
            .index(&self.get_indices())
    }

    pub fn get_index_count(&self) -> u32 {
        self.meshes
            .iter()
            .map(|mesh| mesh.indices.len())
            .sum::<usize>() as u32
    }
}

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
}

impl From<Vertex> for GpuVertex {
    fn from(val: Vertex) -> Self {
        GpuVertex {
            position: val.position.to_array(),
            normal: val.normal.to_array(),
        }
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[Index] {
        &self.indices
    }
}

impl Component for Mesh {}

#[derive(Clone)]
pub struct GpuMesh {
    vertices: Vec<GpuVertex>,
    indices: Vec<Index>,
}

impl GpuMesh {
    pub fn new(vertices: Vec<GpuVertex>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }

    pub fn vertices(&self) -> &[GpuVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[Index] {
        &self.indices
    }

    pub fn into_parts(self) -> (Vec<GpuVertex>, Vec<Index>) {
        (self.vertices, self.indices)
    }
}

pub mod gltf;
pub mod static_mesh;
