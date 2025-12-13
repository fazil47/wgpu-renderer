use std::collections::{HashMap, HashSet};

use ecs::{Entity, World};
use maths::Mat4;

use crate::{
    core::flags::DirtyFlags,
    transform::{GlobalTransform, Transform},
};

pub fn calculate_global_position_system(world: &mut World) {
    let is_dirty = world
        .get_resource::<DirtyFlags>()
        .map(|f| f.transforms)
        .unwrap_or(false);

    if !is_dirty {
        return;
    }

    let mut cache: HashMap<Entity, Mat4> = HashMap::new();
    let mut visiting: HashSet<Entity> = HashSet::new();

    for entity in world.get_entities_with::<Transform>() {
        let _ = calculate_global_position(world, entity, &mut cache, &mut visiting);
    }
}

fn calculate_global_position(
    world: &World,
    entity: Entity,
    cache: &mut HashMap<Entity, Mat4>,
    visiting: &mut HashSet<Entity>,
) -> Result<Mat4, String> {
    if let Some(matrix) = cache.get(&entity) {
        return Ok(*matrix);
    }

    if !visiting.insert(entity) {
        return Err(format!(
            "Transform hierarchy cycle detected at entity {entity:?}"
        ));
    }

    let transform = *world
        .get_component::<Transform>(entity)
        .ok_or_else(|| format!("Entity {entity:?} missing component: Transform"))?;

    let local_matrix = transform.get_matrix();

    let global_matrix = if let Some(parent) = transform.parent {
        if !world.has_component::<Transform>(parent) {
            return Err(format!("Entity {parent:?} missing component: Transform"));
        }

        let parent_matrix = calculate_global_position(world, parent, cache, visiting)?;
        parent_matrix * local_matrix
    } else {
        local_matrix
    };

    visiting.remove(&entity);

    cache.insert(entity, global_matrix);

    if let Some(mut global_transform) = world.get_component_mut::<GlobalTransform>(entity) {
        *global_transform = GlobalTransform::from_matrix(global_matrix);
    }

    Ok(global_matrix)
}
