use crate::{
    material::{Material, RGBA},
    transform::Transform,
};
use ecs::{Entity, World};
use maths::{Vec3, Vec4};

use super::{Mesh, Vertex};

pub trait StaticMeshExt {
    fn triangle(world: &mut World) -> Entity;
    fn pentagon(world: &mut World) -> Entity;
    fn cube(world: &mut World) -> Entity;
    fn octahedron(world: &mut World) -> Entity;
    fn cornell_box(world: &mut World) -> Vec<Entity>;
    fn sphere(world: &mut World) -> Entity;
}

impl StaticMeshExt for Mesh {
    fn triangle(world: &mut World) -> Entity {
        let material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        let material_entity = world.create_entity();
        world.add_component(material_entity, material.clone());

        let mesh = Mesh::new(
            vec![
                Vertex {
                    position: Vec4::new(0.0, 1.0, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-1.0, -1.0, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(1.0, -1.0, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
            ],
            vec![0, 1, 2].into(),
            Some(material_entity),
        );

        let mesh_entity = world.create_entity();
        world.add_component(mesh_entity, mesh);
        world.add_component(mesh_entity, Transform::default());

        mesh_entity
    }

    fn pentagon(world: &mut World) -> Entity {
        let material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        let material_entity = world.create_entity();
        world.add_component(material_entity, material.clone());

        let mesh = Mesh::new(
            vec![
                Vertex {
                    position: Vec4::new(-0.0868241, 0.49240386, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // A
                Vertex {
                    position: Vec4::new(-0.49513406, 0.06958647, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // B
                Vertex {
                    position: Vec4::new(-0.21918549, -0.44939706, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // C
                Vertex {
                    position: Vec4::new(0.35966998, -0.3473291, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // D
                Vertex {
                    position: Vec4::new(0.44147372, 0.2347359, 0.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // E
            ],
            vec![0, 1, 4, 1, 2, 4, 2, 3, 4].into(),
            Some(material_entity),
        );

        let mesh_entity = world.create_entity();
        world.add_component(mesh_entity, mesh);
        world.add_component(mesh_entity, Transform::default());

        mesh_entity
    }

    fn cube(world: &mut World) -> Entity {
        let material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        let material_entity = world.create_entity();
        world.add_component(material_entity, material.clone());

        let mesh = Mesh::new(
            vec![
                // Front face
                Vertex {
                    position: Vec4::new(-0.5, -0.5, 0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(0.5, -0.5, 0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(0.5, 0.5, 0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-0.5, 0.5, 0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                // Back face
                Vertex {
                    position: Vec4::new(-0.5, -0.5, -0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(0.5, -0.5, -0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(0.5, 0.5, -0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-0.5, 0.5, -0.5, 1.0),
                    normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                },
            ],
            vec![
                0, 1, 2, 2, 3, 0, // Front face
                1, 5, 6, 6, 2, 1, // Right face
                5, 4, 7, 7, 6, 5, // Back face
                4, 0, 3, 3, 7, 4, // Left face
                3, 2, 6, 6, 7, 3, // Top face
                4, 5, 1, 1, 0, 4, // Bottom face
            ]
            .into(),
            Some(material_entity),
        );

        let mesh_entity = world.create_entity();
        world.add_component(mesh_entity, mesh);
        world.add_component(mesh_entity, Transform::default());

        mesh_entity
    }

    fn octahedron(world: &mut World) -> Entity {
        let material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        let material_entity = world.create_entity();
        world.add_component(material_entity, material.clone());

        let mesh = Mesh::new(
            vec![
                Vertex {
                    position: Vec4::new(0.0, 1.0, 0.0, 1.0),
                    normal: Vec4::new(0.0, 1.0, 0.0, 0.0),
                }, // 0: Top
                Vertex {
                    position: Vec4::new(0.0, -1.0, 0.0, 1.0),
                    normal: Vec4::new(0.0, -1.0, 0.0, 0.0),
                }, // 1: Bottom
                Vertex {
                    position: Vec4::new(1.0, 0.0, 0.0, 1.0),
                    normal: Vec4::new(1.0, 0.0, 0.0, 0.0),
                }, // 2: Right
                Vertex {
                    position: Vec4::new(-1.0, 0.0, 0.0, 1.0),
                    normal: Vec4::new(-1.0, 0.0, 0.0, 0.0),
                }, // 3: Left
                Vertex {
                    position: Vec4::new(0.0, 0.0, 1.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                }, // 4: Front
                Vertex {
                    position: Vec4::new(0.0, 0.0, -1.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
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
            ]
            .into(),
            Some(material_entity),
        );

        let mesh_entity = world.create_entity();
        world.add_component(mesh_entity, mesh);
        world.add_component(mesh_entity, Transform::default());

        mesh_entity
    }

    fn cornell_box(world: &mut World) -> Vec<Entity> {
        let left_material = Material::new(RGBA::new([1.0, 0.0, 0.0, 1.0]));
        let left_material_entity = world.create_entity();
        world.add_component(left_material_entity, left_material.clone());

        let left_mesh = Mesh::new(
            vec![
                Vertex {
                    position: Vec4::new(-2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, 2.0, 2.0, 1.0),
                    normal: Vec4::new(1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, -2.0, 2.0, 1.0),
                    normal: Vec4::new(1.0, 0.0, 0.0, 0.0),
                },
            ],
            vec![0, 1, 2, 2, 3, 0].into(),
            Some(left_material_entity),
        );

        let right_material = Material::new(RGBA::new([0.0, 1.0, 0.0, 1.0]));
        let right_material_entity = world.create_entity();
        world.add_component(right_material_entity, right_material.clone());

        let right_mesh = Mesh::new(
            vec![
                Vertex {
                    position: Vec4::new(2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(-1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, -2.0, 2.0, 1.0),
                    normal: Vec4::new(-1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, 2.0, 2.0, 1.0),
                    normal: Vec4::new(-1.0, 0.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(-1.0, 0.0, 0.0, 0.0),
                },
            ],
            vec![0, 1, 2, 2, 3, 0].into(),
            Some(right_material_entity),
        );

        let other_material = Material::new(RGBA::new([1.0, 1.0, 1.0, 1.0]));
        let other_material_entity = world.create_entity();
        world.add_component(other_material_entity, other_material.clone());

        let other_mesh = Mesh::new(
            vec![
                // Back wall
                Vertex {
                    position: Vec4::new(-2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                },
                // Top wall
                Vertex {
                    position: Vec4::new(-2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, -1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, 2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, -1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, 2.0, 2.0, 1.0),
                    normal: Vec4::new(0.0, -1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, 2.0, 2.0, 1.0),
                    normal: Vec4::new(0.0, -1.0, 0.0, 0.0),
                },
                // Bottom wall
                Vertex {
                    position: Vec4::new(-2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(-2.0, -2.0, 2.0, 1.0),
                    normal: Vec4::new(0.0, 1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, -2.0, 2.0, 1.0),
                    normal: Vec4::new(0.0, 1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec4::new(2.0, -2.0, -2.0, 1.0),
                    normal: Vec4::new(0.0, 1.0, 0.0, 0.0),
                },
            ],
            vec![
                0, 1, 2, 2, 3, 0, // Back wall
                4, 5, 6, 6, 7, 4, // Top wall
                8, 9, 10, 10, 11, 8, // Bottom wall
            ]
            .into(),
            Some(other_material_entity),
        );

        let left_mesh_entity = world.create_entity();
        world.add_component(left_mesh_entity, left_mesh);
        world.add_component(left_mesh_entity, Transform::new(Vec3::new(-2.0, 0.0, 0.0)));

        let right_mesh_entity = world.create_entity();
        world.add_component(right_mesh_entity, right_mesh);
        world.add_component(right_mesh_entity, Transform::new(Vec3::new(2.0, 0.0, 0.0)));

        let other_mesh_entity = world.create_entity();
        world.add_component(other_mesh_entity, other_mesh);
        world.add_component(other_mesh_entity, Transform::default());

        vec![left_mesh_entity, right_mesh_entity, other_mesh_entity]
    }

    fn sphere(world: &mut World) -> Entity {
        let material = Material::new(RGBA::new([1.0, 1.0, 1.0, 1.0]));
        let material_entity = world.create_entity();
        world.add_component(material_entity, material.clone());

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
                    position: Vec4::new(x, y, z, 1.0),
                    normal: Vec4::new(nx, ny, nz, 0.0),
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

        let mesh = Mesh::new(vertices, indices.into(), Some(material_entity));
        let mesh_entity = world.create_entity();
        world.add_component(mesh_entity, mesh);
        world.add_component(mesh_entity, Transform::default());

        mesh_entity
    }
}
