use ecs::{Component, Entity};
use maths::{Mat4, Quat, Vec3};

/// Transform component for position, rotation, and scale
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub parent: Option<Entity>,
}

impl Component for Transform {}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: None,
        }
    }

    pub fn with_parent(position: Vec3, parent: Entity) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: Some(parent),
        }
    }

    pub fn get_matrix(&self) -> maths::Mat4 {
        let translation_matrix = maths::Mat4::from_translation(self.position);
        let rotation_matrix = maths::Mat4::from_rotation(self.rotation);
        let scale_matrix = maths::Mat4::from_scale(self.scale);
        translation_matrix * rotation_matrix * scale_matrix
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlobalTransform {
    pub matrix: Mat4,
}

impl GlobalTransform {
    pub fn identity() -> Self {
        Self {
            matrix: Mat4::IDENTITY,
        }
    }

    pub fn from_matrix(matrix: Mat4) -> Self {
        Self { matrix }
    }

    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            matrix: transform.get_matrix(),
        }
    }
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Component for GlobalTransform {}

#[derive(Debug, Clone, Default)]
pub struct Children {
    pub entities: Vec<Entity>,
}

impl Children {
    pub fn new(entities: Vec<Entity>) -> Self {
        Self { entities }
    }
}

impl Component for Children {}

#[derive(Debug, Clone)]
pub struct Name(pub String);

impl Name {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Component for Name {}

impl From<Transform> for transform_gizmo_egui::math::Transform {
    fn from(val: Transform) -> Self {
        transform_gizmo_egui::math::Transform {
            translation: val.position.into(),
            rotation: val.rotation.into(),
            scale: val.scale.into(),
        }
    }
}

impl From<transform_gizmo_egui::math::Transform> for Transform {
    fn from(value: transform_gizmo_egui::math::Transform) -> Self {
        Self {
            position: value.translation.into(),
            rotation: value.rotation.into(),
            scale: value.scale.into(),
            parent: None,
        }
    }
}

pub fn calculate_global_position_system(world: &mut ecs::World) {
    let is_dirty = world
        .get_resource::<crate::core::flags::DirtyFlags>()
        .map(|f| f.transforms)
        .unwrap_or(false);

    if !is_dirty {
        return;
    }

    let mut cache: std::collections::HashMap<Entity, Mat4> = std::collections::HashMap::new();
    let mut visiting: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    for entity in world.get_entities_with::<Transform>() {
        let _ = compute_global_transform(world, entity, &mut cache, &mut visiting);
    }
}

fn compute_global_transform(
    world: &ecs::World,
    entity: Entity,
    cache: &mut std::collections::HashMap<Entity, Mat4>,
    visiting: &mut std::collections::HashSet<Entity>,
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

        let parent_matrix = compute_global_transform(world, parent, cache, visiting)?;
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
