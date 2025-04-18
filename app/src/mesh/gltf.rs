use super::Mesh;
use crate::wgpu::{Index, Vertex};
use std::path::Path;
use maths::{Mat4, Vec4};
use wgpu::util::DeviceExt;

pub struct GltfMesh {
    vertices: Vec<Vertex>,
    indices: Vec<Index>,
}

impl Mesh for GltfMesh {
    fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GLTF Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GLTF Index Buffer"),
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

impl GltfMesh {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let (document, buffers, _) = gltf::import(path).unwrap();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Track base index for correct indexing across primitives
        let mut base_index = 0;

        for mesh in document.meshes() {
            // Find the node that references this mesh to get its transform
            let mesh_transform_raw = document
                .nodes()
                .find(|node| node.mesh().map_or(false, |m| m.index() == mesh.index()))
                .map(|node| node.transform().matrix());
            let mesh_transform = if let Some(mat) = mesh_transform_raw {
                Mat4::from_matrix(mat)
            } else {
                Mat4::IDENTITY
            };

            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                if let Some(positions) = reader.read_positions() {
                    let positions_vec: Vec<_> = positions.collect();
                    let vertex_count = positions_vec.len();

                    let colors = reader
                        .read_colors(0)
                        .map(|c| c.into_rgba_f32().collect::<Vec<_>>());
                    let normals = reader.read_normals().map(|n| n.collect::<Vec<_>>());

                    for i in 0..vertex_count {
                        let position = Vec4::from_array3(positions_vec[i]);

                        // Transform position by mesh_transform
                        let transformed_pos = mesh_transform * position;

                        let color = colors
                            .as_ref()
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or([1.0, 1.0, 1.0, 1.0]);

                        let normal_raw = normals
                            .as_ref()
                            .and_then(|n| n.get(i))
                            .copied();
                        let normal = if let Some(n) = normal_raw {
                            Vec4::from_array3(n)
                        } else {
                            Vec4::UP
                        };

                        // Transform normal vector by mesh_transform
                        let mut transformed_normal = mesh_transform * normal;
                        transformed_normal.normalize();

                        vertices.push(Vertex {
                            position: transformed_pos.to_array(),
                            color,
                            normal: transformed_normal.to_array(),
                        });
                    }

                    // Handle indices
                    if let Some(indices_reader) = reader.read_indices() {
                        indices.extend(indices_reader.into_u32().map(|i| i + base_index));
                    } else {
                        // If no indices provided, create a default triangulation
                        for i in 0..vertex_count as u32 / 3 {
                            indices.push(base_index + i * 3);
                            indices.push(base_index + i * 3 + 1);
                            indices.push(base_index + i * 3 + 2);
                        }
                    }

                    base_index += vertex_count as u32;
                }
            }
        }

        Self { vertices, indices }
    }
}
