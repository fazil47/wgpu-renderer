use super::{Material, Mesh};
use crate::rendering::wgpu::{RGBA, Vertex};
use gltf::material::AlphaMode;
use maths::{Mat4, Vec3};
use std::{collections::HashMap, path::Path};

pub trait GltfMeshExt {
    fn from_gltf<P: AsRef<Path>>(path: P) -> Result<Vec<Material>, String>;
}

impl GltfMeshExt for Material {
    fn from_gltf<P: AsRef<Path>>(path: P) -> Result<Vec<Material>, String> {
        let (document, buffers, _) = gltf::import(path).unwrap();
        let mut materials = Vec::new();

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
            materials.push(Material::new(rgba));
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

            // Group mesh primitives by material
            let mut primitives_by_material: HashMap<usize, Vec<gltf::Primitive>> = HashMap::new();

            for primitive in mesh.primitives() {
                let material_id = primitive.material().index();
                if let Some(material_id) = material_id {
                    primitives_by_material
                        .entry(material_id)
                        .or_default()
                        .push(primitive);
                }
            }

            for (material_id, primitives) in primitives_by_material {
                if material_id >= materials.len() {
                    return Err(format!("Material index out of bounds: {material_id}"));
                }

                let mut mesh_vertices = Vec::new();
                let mut mesh_indices = Vec::new();
                let mut base_index = 0;

                for primitive in primitives {
                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                    let normals = reader.read_normals().map(|n| n.collect::<Vec<[f32; 3]>>());

                    if let Some(positions) = reader.read_positions() {
                        let positions_vec: Vec<_> = positions.collect();
                        let vertex_count = positions_vec.len();

                        for i in 0..vertex_count {
                            let position = Vec3::from_array(positions_vec[i]);

                            // Transform position by mesh_transform
                            let transformed_pos = mesh_transform * position;

                            let normal_raw = normals.as_ref().and_then(|n| n.get(i)).copied();
                            let normal = if let Some(n) = normal_raw {
                                Vec3::from_array(n)
                            } else {
                                return Err(format!("Missing normal for vertex {i}"));
                            };

                            // Transform normal vector by mesh_transform
                            let mut transformed_normal = mesh_transform * normal;
                            transformed_normal.normalize();

                            mesh_vertices.push(Vertex {
                                position: transformed_pos.to_array(),
                                normal: transformed_normal.to_array(),
                            });
                        }

                        // Handle indices
                        if let Some(indices_reader) = reader.read_indices() {
                            mesh_indices.extend(indices_reader.into_u32().map(|i| i + base_index));
                        } else {
                            return Err("No indices found".to_string());
                        }

                        base_index += vertex_count as u32;
                    } else {
                        return Err("No positions found".to_string());
                    }
                }

                materials[material_id].add_mesh(Mesh::new(mesh_vertices, mesh_indices));
            }
        }

        Ok(materials)
    }
}
