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
