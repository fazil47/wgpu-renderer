use ecs::{Entity, EntityEvent};

/// Fired when an entity's Transform component has been modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransformChanged(pub Entity);

impl EntityEvent for TransformChanged {
    fn entity(&self) -> Entity {
        self.0
    }
}
