use wgpu::util::DeviceExt;

use crate::rendering::wgpu::{
    Index, RAYTRACE_MATERIAL_STRIDE, RAYTRACE_VERTEX_MATERIAL_ID_OFFSET,
    RAYTRACE_VERTEX_NORMAL_OFFSET, RAYTRACE_VERTEX_STRIDE, RGBA, RaytracerMaterial,
    RaytracerVertex, Vertex,
};
use crate::wgpu_utils::WgpuExt;

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

    fn get_vertices(&self) -> Vec<Vertex> {
        self.meshes
            .iter()
            .flat_map(|mesh| &mesh.vertices)
            .copied()
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

    fn get_raytracer_vertices(&self, material_id: usize) -> Vec<RaytracerVertex> {
        self.meshes
            .iter()
            .flat_map(|mesh| {
                mesh.vertices
                    .iter()
                    .map(|vertex| RaytracerVertex::from_vertex(vertex, material_id))
            })
            .collect()
    }

    fn get_raytracer_indices(&self, vertices_offset: &mut u32) -> Vec<Index> {
        let mut indices = Vec::new();

        for mesh in &self.meshes {
            let mesh_indices = &mesh.indices;

            // Add indices with the vertices offset
            indices.extend(mesh_indices.iter().map(|&index| index + *vertices_offset));

            // Update vertices offset for the next mesh
            *vertices_offset += mesh.vertices.len() as u32;
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

pub trait RaytracerExt {
    fn create_materials_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_material_stride_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_vertices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_vertex_stride_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_vertex_normal_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_vertex_material_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
    fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer;
}

impl RaytracerExt for Vec<Material> {
    fn create_materials_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let raytracer_materials: Vec<RaytracerMaterial> =
            self.iter().map(RaytracerMaterial::from_material).collect();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials Buffer"),
            contents: bytemuck::cast_slice(&raytracer_materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_material_stride_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Stride Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[RAYTRACE_MATERIAL_STRIDE]),
        })
    }

    fn create_vertices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut all_vertices: Vec<RaytracerVertex> = Vec::new();
        for (material_id, material) in self.iter().enumerate() {
            let vertices = material.get_raytracer_vertices(material_id);
            all_vertices.extend(vertices);
        }

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytracer Vertices Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_vertex_stride_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Stride Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[RAYTRACE_VERTEX_STRIDE]),
        })
    }
    fn create_vertex_normal_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Normal Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[RAYTRACE_VERTEX_NORMAL_OFFSET]),
        })
    }
    fn create_vertex_material_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Material Offset Uniform Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[RAYTRACE_VERTEX_MATERIAL_ID_OFFSET]),
        })
    }

    fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut all_indices: Vec<Index> = Vec::new();
        let mut vertices_offset: u32 = 0;
        for material in self {
            all_indices.extend(material.get_raytracer_indices(&mut vertices_offset));
        }

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytracer Indices Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }
}

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<Index>,
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

    pub fn into_parts(self) -> (Vec<Vertex>, Vec<Index>) {
        (self.vertices, self.indices)
    }
}

pub mod gltf;
pub mod static_mesh;
