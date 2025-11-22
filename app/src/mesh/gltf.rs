use super::{Mesh, Vertex};
use crate::{
    material::{Material, RGBA},
    transform::{Children, GlobalTransform, Name, Transform},
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

struct GltfContext<'a> {
    buffers: &'a [gltf::buffer::Data],
    materials: &'a [Entity],
    mesh_entities: &'a mut Vec<Entity>,
    node_entities: &'a mut HashMap<usize, Entity>,
    processed_nodes: &'a mut HashSet<usize>,
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
            let material_entity = world.create_entity();
            world.add_component(
                material_entity,
                Material::new(rgba, material.double_sided()),
            );

            if let Some(name) = derive_material_name(&material) {
                world.add_component(material_entity, Name::new(name));
            }

            materials.push(material_entity);
        }

        let mut mesh_entities = Vec::new();
        let mut node_entities: HashMap<usize, Entity> = HashMap::new();
        let mut processed_nodes: HashSet<usize> = HashSet::new();

        let mut context = GltfContext {
            buffers: &buffers,
            materials: &materials,
            mesh_entities: &mut mesh_entities,
            node_entities: &mut node_entities,
            processed_nodes: &mut processed_nodes,
        };

        if let Some(default_scene) = document.default_scene() {
            for node in default_scene.nodes() {
                process_node(world, node, None, &mut context)?;
            }
        } else {
            for scene in document.scenes() {
                for node in scene.nodes() {
                    process_node(world, node, None, &mut context)?;
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
    context: &mut GltfContext,
) -> Result<(), String> {
    let node_index = node.index();

    let entity = match context.node_entities.get(&node_index) {
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

            if let Some(name) = derive_node_name(&node) {
                world.add_component(entity, Name::new(name));
            }

            if parent.is_none() {
                world.add_component(entity, GlobalTransform::from_transform(&transform));
            } else {
                world.add_component(entity, GlobalTransform::identity());
            }

            context.node_entities.insert(node_index, entity);
            entity
        }
    };

    if let Some(parent_entity) = parent {
        append_child(world, parent_entity, entity)?;
    }

    if context.processed_nodes.insert(node_index) {
        if let Some(mesh) = node.mesh() {
            create_mesh_entities(world, entity, mesh, context)?;
        }

        for child in node.children() {
            process_node(world, child, Some(entity), context)?;
        }
    }

    Ok(())
}

fn create_mesh_entities(
    world: &mut World,
    node_entity: Entity,
    mesh: gltf::Mesh,
    context: &mut GltfContext,
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
    let mut parent_name_cache: Option<Option<String>> = None;

    for (material_id, primitives) in groups.into_iter() {
        if let Some(material_index) = material_id
            && material_index >= context.materials.len()
        {
            return Err(format!("Material index out of bounds: {material_index}"));
        }

        let target_entity = if use_node_entity && !world.has_component::<Mesh>(node_entity) {
            node_entity
        } else {
            let parent_label = parent_name_cache
                .get_or_insert_with(|| entity_name(&*world, node_entity))
                .clone();

            let material_label = material_id.and_then(|material_index| {
                context
                    .materials
                    .get(material_index)
                    .and_then(|&entity| entity_name(&*world, entity))
            });

            let child_name = match (parent_label.clone(), material_label) {
                (Some(parent), Some(material)) => Some(format!("{parent} - {material}")),
                (Some(parent), None) => Some(parent),
                (None, Some(material)) => Some(material),
                (None, None) => None,
            };

            create_child_entity(world, node_entity, child_name)?
        };

        let (vertices, indices) = build_mesh(&primitives, context.buffers)?;

        let material = material_id.map(|id| context.materials[id]);
        let mesh_component = Mesh::new(vertices, indices, material);
        world.add_component(target_entity, mesh_component);
        context.mesh_entities.push(target_entity);
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

fn create_child_entity(
    world: &mut World,
    parent: Entity,
    name: Option<String>,
) -> Result<Entity, String> {
    let child = world.create_entity();
    let transform = Transform {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        parent: Some(parent),
    };

    world.add_component(child, transform);
    world.add_component(child, GlobalTransform::identity());
    if let Some(name) = name {
        world.add_component(child, Name::new(name));
    }
    append_child(world, parent, child)?;

    Ok(child)
}

fn ensure_parent(world: &World, entity: Entity, parent: Option<Entity>) -> Result<(), String> {
    let mut transform = world
        .get_component_mut::<Transform>(entity)
        .ok_or_else(|| format!("Entity {entity:?} missing Transform component"))?;

    transform.parent = parent;

    Ok(())
}

fn append_child(world: &mut World, parent: Entity, child: Entity) -> Result<(), String> {
    if let Some(mut children) = world.get_component_mut::<Children>(parent) {
        if !children.entities.contains(&child) {
            children.entities.push(child);
        }
    } else {
        world.add_component(parent, Children::new(vec![child]));
    }

    Ok(())
}

fn entity_name(world: &World, entity: Entity) -> Option<String> {
    world
        .get_component::<Name>(entity)
        .map(|name| name.0.clone())
}

fn derive_node_name(node: &gltf::Node) -> Option<String> {
    if let Some(name) = node.name()
        && !name.is_empty()
    {
        return Some(name.to_string());
    }

    if let Some(mesh) = node.mesh()
        && let Some(mesh_name) = mesh.name()
        && !mesh_name.is_empty()
    {
        return Some(mesh_name.to_string());
    }

    None
}

fn derive_material_name(material: &gltf::Material) -> Option<String> {
    if let Some(name) = material.name()
        && !name.is_empty()
    {
        return Some(name.to_string());
    }

    None
}
