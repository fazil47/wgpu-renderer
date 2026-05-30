use ecs::{Entity, EntityEvent, Event};

/// Fired when an entity's Transform component has been modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransformChanged(pub Entity);

impl EntityEvent for TransformChanged {
    fn entity(&self) -> Entity {
        self.0
    }
}

/// Fired when an entity's GlobalTransform has been recomputed
/// (either because its own Transform changed or an ancestor's did).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalTransformChanged(pub Entity);

impl EntityEvent for GlobalTransformChanged {
    fn entity(&self) -> Entity {
        self.0
    }
}

/// Fired when a light property (direction, color, etc.) has changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightsChanged;
impl Event for LightsChanged {}

/// Fired when the user requests a probe bake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProbeBakeRequested;
impl Event for ProbeBakeRequested {}

/// Fired when the raytracer accumulation should be reset (camera moved, resize, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RaytracerReset;
impl Event for RaytracerReset {}
