use std::collections::{HashMap, HashSet};

use ecs::{Entity, World};
use maths::Mat4;

use crate::{
    core::events::{GlobalTransformChanged, TransformChanged},
    transform::{GlobalTransform, Transform},
};

pub fn calculate_global_transform_system(world: &mut World) {
    // Collect changed entities from events
    let changed_entities: HashSet<Entity> = match world.read_events::<TransformChanged>() {
        Some(events) if !events.is_empty() => events.iter().map(|e| e.0).collect(),
        _ => return,
    };

    // Build parent -> children map so we can find descendants
    let all_transforms = world.get_entities_with::<Transform>();
    let mut children_map: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for &entity in &all_transforms {
        if let Some(transform) = world.get_component::<Transform>(entity)
            && let Some(parent) = transform.parent
        {
            children_map.entry(parent).or_default().push(entity);
        }
    }

    // Collect dirty set: changed entities + all their descendants
    let mut dirty: HashSet<Entity> = HashSet::new();
    let mut stack: Vec<Entity> = changed_entities.into_iter().collect();
    while let Some(entity) = stack.pop() {
        if dirty.insert(entity)
            && let Some(children) = children_map.get(&entity)
        {
            stack.extend(children);
        }
    }

    // Recalculate only dirty entities, using stored GlobalTransform for clean ancestors
    let mut cache: HashMap<Entity, Mat4> = HashMap::new();
    let mut visiting: HashSet<Entity> = HashSet::new();
    for &entity in &dirty {
        let _ = calculate_global_transform(world, entity, &mut cache, &mut visiting, &dirty);
    }

    // Notify downstream systems which entities had their GlobalTransform recomputed
    for entity in dirty {
        world.send_event(GlobalTransformChanged(entity));
    }
}

fn calculate_global_transform(
    world: &World,
    entity: Entity,
    cache: &mut HashMap<Entity, Mat4>,
    visiting: &mut HashSet<Entity>,
    dirty: &HashSet<Entity>,
) -> Result<Mat4, String> {
    if let Some(matrix) = cache.get(&entity) {
        return Ok(*matrix);
    }

    // Non-dirty entities still have a valid GlobalTransform — use it directly
    if !dirty.contains(&entity)
        && let Some(global) = world.get_component::<GlobalTransform>(entity)
    {
        cache.insert(entity, global.matrix);
        return Ok(global.matrix);
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

        let parent_matrix = calculate_global_transform(world, parent, cache, visiting, dirty)?;
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
