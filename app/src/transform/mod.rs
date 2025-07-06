use ecs::{Component, Entity};
use maths::{Quat, Vec3};

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
