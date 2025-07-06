use ecs::{Component, EntityId};
use maths::{Quat, Vec3};

/// Transform component for position, rotation, and scale
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub parent: Option<EntityId>,
}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: None,
        }
    }

    pub fn with_parent(position: Vec3, parent: EntityId) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: Some(parent),
        }
    }
}

impl Component for Transform {}
