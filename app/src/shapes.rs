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
