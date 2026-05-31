use ecs::{Component, Entity, Relationship, RelationshipTarget};

/// Marks an entity as a child of another entity. The inverse [`Children`]
/// component is maintained automatically by the ECS relationship system.
pub struct ChildOf(pub Entity);
impl Component for ChildOf {}
impl Relationship for ChildOf {
    type Target = Children;
    fn target(&self) -> Entity {
        self.0
    }
}

/// Auto-maintained inverse of [`ChildOf`]. Lists all entities that have a
/// `ChildOf` pointing at this entity.
#[derive(Default)]
pub struct Children(pub Vec<Entity>);
impl Component for Children {}
impl RelationshipTarget for Children {
    fn entities(&self) -> &[Entity] {
        &self.0
    }
    fn add(&mut self, entity: Entity) {
        self.0.push(entity);
    }
    fn remove(&mut self, entity: Entity) {
        self.0.retain(|&e| e != entity);
    }
}
