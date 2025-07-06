use crate::rendering::wgpu::{RGBA, Vertex};

use super::{Material, Mesh};

pub trait StaticMeshExt {
    fn triangle() -> Material;
    fn pentagon() -> Material;
    fn cube() -> Material;
    fn octahedron() -> Material;
    fn cornell_box() -> Vec<Material>;
    fn sphere() -> Material;
}

impl StaticMeshExt for Material {
    fn triangle() -> Material {
        let mut material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        material.add_mesh(Mesh::new(
            vec![
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [-1.0, -1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [1.0, -1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
            ],
            vec![0, 1, 2],
        ));

        material
    }

    fn pentagon() -> Material {
        let mut material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        material.add_mesh(Mesh::new(
            vec![
                Vertex {
                    position: [-0.0868241, 0.49240386, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // A
                Vertex {
                    position: [-0.49513406, 0.06958647, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // B
                Vertex {
                    position: [-0.21918549, -0.44939706, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // C
                Vertex {
                    position: [0.35966998, -0.3473291, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // D
                Vertex {
                    position: [0.44147372, 0.2347359, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // E
            ],
            vec![0, 1, 4, 1, 2, 4, 2, 3, 4],
        ));

        material
    }

    fn cube() -> Material {
        let mut material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        material.add_mesh(Mesh::new(
            vec![
                // Front face
                Vertex {
                    position: [-0.5, -0.5, 0.5, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.5, -0.5, 0.5, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5, 0.5, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [-0.5, 0.5, 0.5, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                // Back face
                Vertex {
                    position: [-0.5, -0.5, -0.5, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [0.5, -0.5, -0.5, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [0.5, 0.5, -0.5, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [-0.5, 0.5, -0.5, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                },
            ],
            vec![
                0, 1, 2, 2, 3, 0, // Front face
                1, 5, 6, 6, 2, 1, // Right face
                5, 4, 7, 7, 6, 5, // Back face
                4, 0, 3, 3, 7, 4, // Left face
                3, 2, 6, 6, 7, 3, // Top face
                4, 5, 1, 1, 0, 4, // Bottom face
            ],
        ));

        material
    }

    fn octahedron() -> Material {
        let mut material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        material.add_mesh(Mesh::new(
            vec![
                Vertex {
                    position: [0.0, 1.0, 0.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                }, // 0: Top
                Vertex {
                    position: [0.0, -1.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                }, // 1: Bottom
                Vertex {
                    position: [1.0, 0.0, 0.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                }, // 2: Right
                Vertex {
                    position: [-1.0, 0.0, 0.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                }, // 3: Left
                Vertex {
                    position: [0.0, 0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                }, // 4: Front
                Vertex {
                    position: [0.0, 0.0, -1.0, 1.0],
                    normal: [0.0, 0.0, -1.0, 0.0],
                }, // 5: Back
            ],
            vec![
                0, 4, 2, // Top-Front-Right
                0, 3, 4, // Top-Left-Front
                0, 5, 3, // Top-Back-Left
                0, 2, 5, // Top-Right-Back
                1, 2, 4, // Bottom-Right-Front
                1, 4, 3, // Bottom-Front-Left
                1, 3, 5, // Bottom-Left-Back
                1, 5, 2, // Bottom-Back-Right
            ],
        ));

        material
    }

    fn cornell_box() -> Vec<Material> {
        let mut left_material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        left_material.add_mesh(Mesh::new(
            vec![
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, 2.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, -2.0, 2.0, 1.0],
                    normal: [1.0, 0.0, 0.0, 0.0],
                },
            ],
            vec![0, 1, 2, 2, 3, 0],
        ));

        let mut right_material = Material::new(RGBA::new([0.0, 1.0, 0.0, 1.0]));
        right_material.add_mesh(Mesh::new(
            vec![
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, 2.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, 2.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    normal: [-1.0, 0.0, 0.0, 0.0],
                },
            ],
            vec![0, 1, 2, 2, 3, 0],
        ));

        let mut other_material = Material::new(RGBA::new([1.0, 1.0, 1.0, 1.0]));
        other_material.add_mesh(Mesh::new(
            vec![
                // Back wall
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                // Top wall
                Vertex {
                    position: [-2.0, 2.0, -2.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, -2.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, 2.0, 2.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, 2.0, 2.0, 1.0],
                    normal: [0.0, -1.0, 0.0, 0.0],
                },
                // Bottom wall
                Vertex {
                    position: [-2.0, -2.0, -2.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [-2.0, -2.0, 2.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, 2.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [2.0, -2.0, -2.0, 1.0],
                    normal: [0.0, 1.0, 0.0, 0.0],
                },
            ],
            vec![
                0, 1, 2, 2, 3, 0, // Back wall
                4, 5, 6, 6, 7, 4, // Top wall
                8, 9, 10, 10, 11, 8, // Bottom wall
            ],
        ));

        vec![left_material, right_material, other_material]
        // vec![other_material, right_material, left_material]
        // vec![right_material, left_material, other_material]
    }

    fn sphere() -> Material {
        let mut material = Material::new(RGBA::new([1.0, 1.0, 1.0, 1.0]));

        let radius = 1.0;
        let sectors = 36; // longitude divisions
        let stacks = 18; // latitude divisions

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for i in 0..=stacks {
            let stack_angle =
                std::f32::consts::PI / 2.0 - i as f32 * std::f32::consts::PI / stacks as f32; // from pi/2 to -pi/2
            let xy = radius * stack_angle.cos(); // r * cos(u)
            let z = radius * stack_angle.sin(); // r * sin(u)

            for j in 0..=sectors {
                let sector_angle = j as f32 * 2.0 * std::f32::consts::PI / sectors as f32; // from 0 to 2pi

                // vertex position (x, y, z)
                let x = xy * sector_angle.cos(); // r * cos(u) * cos(v)
                let y = xy * sector_angle.sin(); // r * cos(u) * sin(v)

                // normalized vertex normal (nx, ny, nz)
                let nx = x / radius;
                let ny = y / radius;
                let nz = z / radius;

                vertices.push(Vertex {
                    position: [x, y, z, 1.0],
                    normal: [nx, ny, nz, 0.0],
                });
            }
        }

        // Generate indices
        for i in 0..stacks {
            let k1 = i * (sectors + 1); // beginning of current stack
            let k2 = k1 + sectors + 1; // beginning of next stack

            for j in 0..sectors {
                // 2 triangles per sector excluding first and last stacks
                // k1 => k2 => k1+1
                if i != 0 {
                    indices.push(k1 + j);
                    indices.push(k2 + j);
                    indices.push(k1 + j + 1);
                }

                // k1+1 => k2 => k2+1
                if i != (stacks - 1) {
                    indices.push(k1 + j + 1);
                    indices.push(k2 + j);
                    indices.push(k2 + j + 1);
                }
            }
        }

        material.add_mesh(Mesh::new(vertices, indices));
        material
    }
}
