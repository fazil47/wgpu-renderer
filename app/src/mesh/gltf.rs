use super::{Mesh, Vertex};
use crate::{
    material::{Material, RGBA},
    transform::{Children, GlobalTransform, Transform},
};
use ecs::{Entity, World};
use gltf::material::AlphaMode;
use maths::{Quat, Vec3, Vec4};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub trait GltfMeshExt {
    fn from_gltf<P: AsRef<Path>>(world: &mut World, path: P) -> Result<Vec<Entity>, String>;
}

impl GltfMeshExt for Mesh {
    fn from_gltf<P: AsRef<Path>>(world: &mut World, path: P) -> Result<Vec<Entity>, String> {
        let (document, buffers, _) = gltf::import(path).map_err(|err| err.to_string())?;

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
            let material = Material::new(rgba, material.double_sided());
            let material_entity = world.create_entity();
            world.add_component(material_entity, material.clone());

            materials.push(material_entity);
        }

        let mut mesh_entities = Vec::new();
        let mut node_entities: HashMap<usize, Entity> = HashMap::new();
        let mut processed_nodes: HashSet<usize> = HashSet::new();

        if let Some(default_scene) = document.default_scene() {
            for node in default_scene.nodes() {
                process_node(
                    world,
                    node,
                    None,
                    &buffers,
                    &materials,
                    &mut mesh_entities,
                    &mut node_entities,
                    &mut processed_nodes,
                )?;
            }
        } else {
            for scene in document.scenes() {
                for node in scene.nodes() {
                    process_node(
                        world,
                        node,
                        None,
                        &buffers,
                        &materials,
                        &mut mesh_entities,
                        &mut node_entities,
                        &mut processed_nodes,
                    )?;
                }
            }
        }

        Ok(mesh_entities)
    }
}

fn process_node(
    world: &mut World,
    node: gltf::Node,
    parent: Option<Entity>,
    buffers: &[gltf::buffer::Data],
    materials: &[Entity],
    mesh_entities: &mut Vec<Entity>,
    node_entities: &mut HashMap<usize, Entity>,
    processed_nodes: &mut HashSet<usize>,
) -> Result<(), String> {
    let node_index = node.index();

    let entity = match node_entities.get(&node_index) {
        Some(&existing) => {
            ensure_parent(world, existing, parent)?;
            existing
        }
        None => {
            let (translation, rotation, scale) = node.transform().decomposed();
            let transform = Transform {
                position: Vec3::from_array(&translation),
                rotation: Quat::from_array(&rotation),
                scale: Vec3::from_array(&scale),
                parent,
            };

            let entity = world.create_entity();
            world.add_component(entity, transform);

            if parent.is_none() {
                world.add_component(entity, GlobalTransform::from_transform(&transform));
            } else {
                world.add_component(entity, GlobalTransform::identity());
            }

            node_entities.insert(node_index, entity);
            entity
        }
    };

    if let Some(parent_entity) = parent {
        append_child(world, parent_entity, entity)?;
    }

    if processed_nodes.insert(node_index) {
        if let Some(mesh) = node.mesh() {
            create_mesh_entities(world, entity, mesh, buffers, materials, mesh_entities)?;
        }

        for child in node.children() {
            process_node(
                world,
                child,
                Some(entity),
                buffers,
                materials,
                mesh_entities,
                node_entities,
                processed_nodes,
            )?;
        }
    }

    Ok(())
}

fn create_mesh_entities(
    world: &mut World,
    node_entity: Entity,
    mesh: gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    materials: &[Entity],
    mesh_entities: &mut Vec<Entity>,
) -> Result<(), String> {
    let mut primitives_by_material: HashMap<Option<usize>, Vec<gltf::Primitive>> = HashMap::new();

    for primitive in mesh.primitives() {
        let material_id = primitive.material().index();
        primitives_by_material
            .entry(material_id)
            .or_default()
            .push(primitive);
    }

    let groups: Vec<(Option<usize>, Vec<gltf::Primitive>)> =
        primitives_by_material.into_iter().collect();
    let use_node_entity = groups.len() == 1;

    for (material_id, primitives) in groups {
        let target_entity = if use_node_entity && !world.has_component::<Mesh>(node_entity) {
            node_entity
        } else {
            create_child_entity(world, node_entity)?
        };

        let (vertices, indices) = build_mesh(&primitives, buffers)?;

        if let Some(material_index) = material_id
            && material_index >= materials.len()
        {
            return Err(format!("Material index out of bounds: {material_index}"));
        }

        let material = material_id.map(|id| materials[id]);
        let mesh_component = Mesh::new(vertices, indices, material);
        world.add_component(target_entity, mesh_component);
        mesh_entities.push(target_entity);
    }

    Ok(())
}

fn build_mesh(
    primitives: &[gltf::Primitive],
    buffers: &[gltf::buffer::Data],
) -> Result<(Vec<Vertex>, Option<Vec<u32>>), String> {
    let mut vertices = Vec::new();
    let mut indices: Option<Vec<u32>> = None;
    let mut base_index = 0u32;

    for primitive in primitives {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
        let normals = reader.read_normals().map(|n| n.collect::<Vec<[f32; 3]>>());

        let positions = reader
            .read_positions()
            .ok_or_else(|| "No positions found".to_string())?;
        let positions_vec: Vec<[f32; 3]> = positions.collect();
        let vertex_count = positions_vec.len();

        for (i, position) in positions_vec.iter().enumerate() {
            let position = Vec4::from_point(Vec3::from_array(position));
            let normal_raw = normals.as_ref().and_then(|n| n.get(i)).copied();
            let normal = normal_raw.map_or_else(
                || Vec4::from_direction(Vec3::new(0.0, 0.0, 1.0)),
                |n| Vec4::from_direction(Vec3::from_array(&n)),
            );

            vertices.push(Vertex { position, normal });
        }

        if let Some(indices_reader) = reader.read_indices() {
            let adjusted = indices_reader.into_u32().map(|i| i + base_index);
            indices.get_or_insert_with(Vec::new).extend(adjusted);
        } else if indices.is_some() {
            return Err("Some primitives have indices while others don't".to_string());
        }

        base_index += vertex_count as u32;
    }

    Ok((vertices, indices))
}

fn create_child_entity(world: &mut World, parent: Entity) -> Result<Entity, String> {
    let child = world.create_entity();
    let transform = Transform {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        parent: Some(parent),
    };

    world.add_component(child, transform);
    world.add_component(child, GlobalTransform::identity());
    append_child(world, parent, child)?;

    Ok(child)
}

fn ensure_parent(world: &World, entity: Entity, parent: Option<Entity>) -> Result<(), String> {
    let transform_rc = world
        .get_component::<Transform>(entity)
        .ok_or_else(|| format!("Entity {entity:?} missing Transform component"))?;

    let mut transform = transform_rc
        .try_borrow_mut()
        .map_err(|_| "Failed to mutably borrow Transform".to_string())?;
    transform.parent = parent;

    Ok(())
}

fn append_child(world: &mut World, parent: Entity, child: Entity) -> Result<(), String> {
    if let Some(children_rc) = world.get_component::<Children>(parent) {
        let mut children = children_rc
            .try_borrow_mut()
            .map_err(|_| "Failed to mutably borrow Children".to_string())?;

        if !children.entities.contains(&child) {
            children.entities.push(child);
        }
    } else {
        world.add_component(parent, Children::new(vec![child]));
    }

    Ok(())
}
