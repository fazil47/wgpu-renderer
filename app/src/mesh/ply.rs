use ply_rs::ply;
use wgpu::util::DeviceExt;

use crate::wgpu::{Index, Vertex};

use super::Mesh;

pub struct PlyMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}

impl Mesh for PlyMesh {
    fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn get_index_count(&self) -> u32 {
        self.indices.len() as u32
    }

    fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    fn get_indices(&self) -> &[Index] {
        &self.indices
    }
}

impl PlyMesh {
    pub fn new(path: &str) -> Self {
        let parser = ply_rs::parser::Parser::<ply_rs::ply::DefaultElement>::new();
        let mut file = std::fs::File::open(path).unwrap();
        let mesh = parser.read_ply(&mut file).unwrap();

        Self {
            vertices: mesh
                .payload
                .get("vertex")
                .unwrap()
                .into_iter()
                .map(|vertex| Vertex {
                    position: [
                        vertex.get("x").get_f32(),
                        vertex.get("y").get_f32(),
                        vertex.get("z").get_f32(),
                        1.0,
                    ],
                    color: [
                        vertex.get("red").get_u8() as f32 / 255.0,
                        vertex.get("green").get_u8() as f32 / 255.0,
                        vertex.get("blue").get_u8() as f32 / 255.0,
                        vertex.get("alpha").get_u8() as f32 / 255.0,
                    ],
                    normal: [
                        vertex.get("nx").get_f32(),
                        vertex.get("ny").get_f32(),
                        vertex.get("nz").get_f32(),
                        0.0,
                    ],
                })
                .collect::<Vec<Vertex>>(),
            indices: mesh
                .payload
                .get("face")
                .unwrap()
                .into_iter()
                .flat_map(|face| {
                    let vertices = face.get("vertex_indices").get_list_u32();

                    // If the face is a triangle, return the vertices
                    if vertices.len() == 3 {
                        return vertices;
                    }

                    // Else if the face is a quad, split it into two triangles
                    vec![
                        vertices[0],
                        vertices[1],
                        vertices[2],
                        vertices[0],
                        vertices[2],
                        vertices[3],
                    ]
                })
                .collect(),
        }
    }
}

trait GetPlyPropertyValue {
    fn get_f32(&self) -> f32;
    fn get_u8(&self) -> u8;
    fn get_list_u32(&self) -> Vec<u32>;
}

impl GetPlyPropertyValue for Option<&ply::Property> {
    fn get_f32(&self) -> f32 {
        match self {
            Some(ply::Property::Float(value)) => *value,
            _ => panic!("Ply Property is not an f32"),
        }
    }

    fn get_u8(&self) -> u8 {
        match self {
            Some(ply::Property::UChar(value)) => *value,
            _ => panic!("Ply Property is not a u8"),
        }
    }

    fn get_list_u32(&self) -> Vec<u32> {
        match self {
            Some(ply::Property::ListUInt(value)) => value.clone(),
            _ => panic!("Ply Property is not a list of u32 values"),
        }
    }
}
