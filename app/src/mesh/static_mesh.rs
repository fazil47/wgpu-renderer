use wgpu::util::DeviceExt;

use crate::wgpu::{Index, Vertex};

use super::Mesh;

pub struct StaticMesh<'mesh> {
    pub vertices: &'mesh [Vertex],
    pub indices: &'mesh [Index],
}

impl<'mesh> Mesh for StaticMesh<'mesh> {
    fn create_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(self.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(self.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn get_index_count(&self) -> u32 {
        self.indices.len() as u32
    }

    fn get_vertices(&self) -> &[Vertex] {
        self.vertices
    }

    fn get_indices(&self) -> &[Index] {
        self.indices
    }
}

impl<'mesh> StaticMesh<'mesh> {
    pub fn new(vertices: &'mesh [Vertex], indices: &'mesh [u32]) -> StaticMesh<'mesh> {
        StaticMesh { vertices, indices }
    }

    pub fn triangle() -> Self {
        Self::new(
            &[
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [1.0, -1.0, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
            ],
            &[0, 1, 2],
        )
    }

    pub fn pentagon() -> Self {
        Self::new(
            &[
                Vertex {
                    position: [-0.0868241, 0.49240386, 0.0, 1.0],
                    color: [0.5, 0.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // A
                Vertex {
                    position: [-0.49513406, 0.06958647, 0.0, 1.0],
                    color: [0.0, 0.5, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // B
                Vertex {
                    position: [-0.21918549, -0.44939706, 0.0, 1.0],
                    color: [0.0, 0.0, 0.5, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // C
                Vertex {
                    position: [0.35966998, -0.3473291, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // D
                Vertex {
                    position: [0.44147372, 0.2347359, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // E
            ],
            &[0, 1, 4, 1, 2, 4, 2, 3, 4],
        )
    }

    pub fn cube() -> Self {
        Self::new(
            &[
                // Front face
                Vertex {
                    position: [-0.5, -0.5, 0.5, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.5, -0.5, 0.5, 1.0],
                    color: [1.0, 0.5, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5, 0.5, 1.0],
                    color: [1.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [-0.5, 0.5, 0.5, 1.0],
                    color: [0.5, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                // Back face
                Vertex {
                    position: [-0.5, -0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [0.5, -0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 0.5, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [-0.5, 0.5, -0.5, 1.0],
                    color: [0.0, 0.5, 1.0, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
            ],
            &[
                0, 1, 2, 2, 3, 0, // Front face
                1, 5, 6, 6, 2, 1, // Right face
                5, 4, 7, 7, 6, 5, // Back face
                4, 0, 3, 3, 7, 4, // Left face
                3, 2, 6, 6, 7, 3, // Top face
                4, 5, 1, 1, 0, 4, // Bottom face
            ],
        )
    }

    pub fn octahedron() -> Self {
        Self::new(
            &[
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                }, // 0: Top
                Vertex {
                    position: [0.0, -1.0, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                }, // 1: Bottom
                Vertex {
                    position: [1.0, 0.0, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                }, // 2: Right
                Vertex {
                    position: [-1.0, 0.0, 0.0, 1.0],
                    color: [1.0, 1.0, 0.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                }, // 3: Left
                Vertex {
                    position: [0.0, 0.0, 1.0, 1.0],
                    color: [1.0, 0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // 4: Front
                Vertex {
                    position: [0.0, 0.0, -1.0, 1.0],
                    color: [0.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                }, // 5: Back
            ],
            &[
                0, 4, 2, // Top-Front-Right
                0, 3, 4, // Top-Left-Front
                0, 5, 3, // Top-Back-Left
                0, 2, 5, // Top-Right-Back
                1, 2, 4, // Bottom-Right-Front
                1, 4, 3, // Bottom-Front-Left
                1, 3, 5, // Bottom-Left-Back
                1, 5, 2, // Bottom-Back-Right
            ],
        )
    }

    pub fn cornell_box() -> Self {
        Self::new(
            &[
                // Left wall (red)
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    color: [0.63, 0.065, 0.05, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    color: [0.63, 0.065, 0.05, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, 2.0, 1.0],
                    color: [0.63, 0.065, 0.05, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, -2.0, 2.0, 1.0],
                    color: [0.63, 0.065, 0.05, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                // Right wall (green)
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    color: [0.14, 0.45, 0.091, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, 2.0, 1.0],
                    color: [0.14, 0.45, 0.091, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, 2.0, 1.0],
                    color: [0.14, 0.45, 0.091, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    color: [0.14, 0.45, 0.091, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                // Back wall (white)
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                // Top wall (white)
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, 2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, 2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                // Bottom wall (white)
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, -2.0, 2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, 2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    color: [0.725, 0.71, 0.68, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
            ],
            &[
                0, 1, 2, 2, 3, 0, // Left wall
                4, 5, 6, 6, 7, 4, // Right wall
                8, 9, 10, 10, 11, 8, // Back wall
                12, 13, 14, 14, 15, 12, // Top wall
                16, 17, 18, 18, 19, 16, // Bottom wall
            ],
        )
    }
}
