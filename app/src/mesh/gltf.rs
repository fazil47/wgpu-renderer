use super::{Mesh, Vertex};
use crate::{
    material::{Material, RGBA},
    transform::Transform,
};
use ecs::{Entity, World};
use gltf::material::AlphaMode;
use maths::{Mat4, Vec3, Vec4};
use std::{collections::HashMap, path::Path};

pub trait GltfMeshExt {
    fn from_gltf<P: AsRef<Path>>(world: &mut World, path: P) -> Result<Vec<Entity>, String>;
}

impl GltfMeshExt for Mesh {
    fn from_gltf<P: AsRef<Path>>(world: &mut World, path: P) -> Result<Vec<Entity>, String> {
        let (document, buffers, _) = gltf::import(path).unwrap();
        let mut materials = Vec::new();
        let mut meshes = Vec::new();

        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();
            let base_color = pbr.base_color_factor();
            let alpha_mode = material.alpha_mode();

            let alpha = match alpha_mode {
                AlphaMode::Opaque => 1.0,
                AlphaMode::Mask => 1.0,
                AlphaMode::Blend => base_color[3],
            };

            let rgba = RGBA::new([base_color[0], base_color[1], base_color[2], alpha]);
            let material = Material::new(rgba);
            let material_entity = world.create_entity();
            world.add_component(material_entity, material.clone());

            materials.push(material_entity);
        }

        for mesh in document.meshes() {
            // Find the node that references this mesh to get its transform
            let mesh_transform_raw = document
                .nodes()
                .find(|node| node.mesh().is_some_and(|m| m.index() == mesh.index()))
                .map(|node| node.transform().matrix());
            let mesh_transform = if let Some(mat) = mesh_transform_raw {
                Mat4::from_matrix(mat)
            } else {
                Mat4::IDENTITY
            };
            let mesh_transform_component = Transform {
                position: mesh_transform.extract_translation(),
                rotation: mesh_transform.extract_rotation(),
                scale: mesh_transform.extract_scale(),
                parent: None,
            };

            // Group mesh primitives by material (None for primitives without materials)
            let mut primitives_by_material: HashMap<Option<usize>, Vec<gltf::Primitive>> =
                HashMap::new();
            for primitive in mesh.primitives() {
                let material_id = primitive.material().index();
                primitives_by_material
                    .entry(material_id)
                    .or_default()
                    .push(primitive);
            }

            for (material_id, primitives) in primitives_by_material {
                if let Some(material_id) = material_id
                    && material_id >= materials.len()
                {
                    return Err(format!("Material index out of bounds: {material_id}"));
                }

                let mut mesh_vertices = Vec::new();
                let mut mesh_indices = None;
                let mut base_index = 0;

                for primitive in primitives {
                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                    let normals = reader.read_normals().map(|n| n.collect::<Vec<[f32; 3]>>());

                    if let Some(positions) = reader.read_positions() {
                        let positions_vec: Vec<_> = positions.collect();
                        let vertex_count = positions_vec.len();

                        for (i, position) in positions_vec.iter().enumerate().take(vertex_count) {
                            let position = Vec4::from_point(Vec3::from_array(position));

                            let normal_raw = normals.as_ref().and_then(|n| n.get(i)).copied();
                            let normal = if let Some(n) = normal_raw {
                                Vec4::from_direction(Vec3::from_array(&n))
                            } else {
                                // TODO: Maybe calculate normals if they are missing?
                                Vec4::from_direction(Vec3::new(0.0, 0.0, 1.0))
                            };

                            mesh_vertices.push(Vertex { position, normal });
                        }

                        // Handle indices
                        if let Some(indices_reader) = reader.read_indices() {
                            let indices = indices_reader.into_u32().map(|i| i + base_index);
                            mesh_indices.get_or_insert(Vec::new()).extend(indices);
                        } else if mesh_indices.is_some() {
                            // TODO: Currently all primitives that share a material are grouped together, but that's not right
                            // Meshes in different transform heirarchies can share a material, but they should be separate
                            return Err(
                                "Some primitives have indices while others don't".to_string()
                            );
                        }

                        base_index += vertex_count as u32;
                    } else {
                        return Err("No positions found".to_string());
                    }
                }

                let mesh_entity = world.create_entity();
                meshes.push(mesh_entity);

                let material = material_id.map(|id| materials[id]);
                let mesh = Mesh::new(mesh_vertices, mesh_indices, material);
                world.add_component(mesh_entity, mesh);
                world.add_component(mesh_entity, mesh_transform_component);
            }
        }

        Ok(meshes)
    }
}
