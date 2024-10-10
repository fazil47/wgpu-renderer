use crate::wgpu::Vertex;

pub struct Triangle<'tri> {
    pub vertices: &'tri [Vertex],
    pub indices: &'tri [u32],
}

impl<'tri> Triangle<'tri> {
    pub fn new() -> Self {
        Self {
            vertices: &[
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                Vertex {
                    position: [1.0, -1.0, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                },
            ],
            indices: &[0, 1, 2],
        }
    }
}

pub struct Pentagon<'pent> {
    pub vertices: &'pent [Vertex],
    pub indices: &'pent [u32],
}

impl<'pent> Pentagon<'pent> {
    pub fn new() -> Self {
        Self {
            vertices: &[
                Vertex {
                    position: [-0.0868241, 0.49240386, 0.0, 1.0],
                    color: [0.5, 0.0, 0.0, 1.0],
                }, // A
                Vertex {
                    position: [-0.49513406, 0.06958647, 0.0, 1.0],
                    color: [0.0, 0.5, 0.0, 1.0],
                }, // B
                Vertex {
                    position: [-0.21918549, -0.44939706, 0.0, 1.0],
                    color: [0.0, 0.0, 0.5, 1.0],
                }, // C
                Vertex {
                    position: [0.35966998, -0.3473291, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                }, // D
                Vertex {
                    position: [0.44147372, 0.2347359, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                }, // E
            ],
            indices: &[0, 1, 4, 1, 2, 4, 2, 3, 4],
        }
    }
}

pub struct Cube<'cube> {
    pub vertices: &'cube [Vertex],
    pub indices: &'cube [u32],
}

impl<'cube> Cube<'cube> {
    pub fn new() -> Self {
        Self {
            vertices: &[
                // Front face
                Vertex {
                    position: [-0.5, -0.5, 0.5, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5, 0.5, 1.0],
                    color: [1.0, 0.5, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, 0.5, 0.5, 1.0],
                    color: [1.0, 1.0, 0.0, 1.0],
                },
                Vertex {
                    position: [-0.5, 0.5, 0.5, 1.0],
                    color: [0.5, 1.0, 0.0, 1.0],
                },
                // Back face
                Vertex {
                    position: [-0.5, -0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 0.5, 1.0],
                },
                Vertex {
                    position: [0.5, 0.5, -0.5, 1.0],
                    color: [0.0, 1.0, 1.0, 1.0],
                },
                Vertex {
                    position: [-0.5, 0.5, -0.5, 1.0],
                    color: [0.0, 0.5, 1.0, 1.0],
                },
            ],
            indices: &[
                0, 1, 2, 2, 3, 0, // Front face
                1, 5, 6, 6, 2, 1, // Right face
                5, 4, 7, 7, 6, 5, // Back face
                4, 0, 3, 3, 7, 4, // Left face
                3, 2, 6, 6, 7, 3, // Top face
                4, 5, 1, 1, 0, 4, // Bottom face
            ],
        }
    }
}

pub struct Octahedron<'oct> {
    pub vertices: &'oct [Vertex],
    pub indices: &'oct [u32],
}

impl<'oct> Octahedron<'oct> {
    pub fn new() -> Self {
        Self {
            vertices: &[
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                }, // 0: Top
                Vertex {
                    position: [0.0, -1.0, 0.0, 1.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                }, // 1: Bottom
                Vertex {
                    position: [1.0, 0.0, 0.0, 1.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                }, // 2: Right
                Vertex {
                    position: [-1.0, 0.0, 0.0, 1.0],
                    color: [1.0, 1.0, 0.0, 1.0],
                }, // 3: Left
                Vertex {
                    position: [0.0, 0.0, 1.0, 1.0],
                    color: [1.0, 0.0, 1.0, 1.0],
                }, // 4: Front
                Vertex {
                    position: [0.0, 0.0, -1.0, 1.0],
                    color: [0.0, 1.0, 1.0, 1.0],
                }, // 5: Back
            ],
            indices: &[
                0, 4, 2, // Top-Front-Right
                0, 3, 4, // Top-Left-Front
                0, 5, 3, // Top-Back-Left
                0, 2, 5, // Top-Right-Back
                1, 2, 4, // Bottom-Right-Front
                1, 4, 3, // Bottom-Front-Left
                1, 3, 5, // Bottom-Left-Back
                1, 5, 2, // Bottom-Back-Right
            ],
        }
    }
}
