use std::collections::HashMap;

use ecs::{Component, Entity, Resource};

#[derive(Debug, Clone, PartialEq)]
pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RGBA {
    pub fn new(rgba: [f32; 4]) -> Self {
        Self {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    pub color: RGBA,
    pub emissive: RGBA,
    pub double_sided: bool,
}

impl Material {
    pub fn new(color: RGBA, emissive: RGBA, double_sided: bool) -> Self {
        Self {
            color,
            emissive,
            double_sided,
        }
    }
}

impl Component for Material {}

impl Default for RGBA {
    fn default() -> Self {
        Self {
            r: 0.8,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        }
    }
}

/// The fallback material used when a mesh has no material assigned.
/// Stored as a resource rather than on an entity so it doesn't
/// participate in material indexing or component hooks.
pub struct DefaultMaterial(pub Material);

impl Resource for DefaultMaterial {}

/// Maps material entities to dense indices used by the GPU.
///
/// Maintained automatically by `Material` component hooks (on_add / on_remove).
/// New materials are appended with the next available index; removed materials
/// leave a gap (indices are never compacted) so existing vertex data stays valid.
pub struct MaterialIndex {
    entity_to_index: HashMap<Entity, usize>,
    next_index: usize,
}

impl Resource for MaterialIndex {}

impl Default for MaterialIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialIndex {
    pub fn new() -> Self {
        Self {
            entity_to_index: HashMap::new(),
            next_index: 0,
        }
    }

    /// Register a new material entity and return its index.
    pub fn add(&mut self, entity: Entity) -> usize {
        let index = self.next_index;
        self.entity_to_index.insert(entity, index);
        self.next_index += 1;
        index
    }

    /// Remove a material entity. Its index is not reused.
    pub fn remove(&mut self, entity: Entity) {
        self.entity_to_index.remove(&entity);
    }

    /// Look up the GPU index for a material entity.
    pub fn get(&self, entity: Entity) -> Option<usize> {
        self.entity_to_index.get(&entity).copied()
    }

    /// Return the full entity-to-index map (for iteration during extraction).
    pub fn entity_to_index(&self) -> &HashMap<Entity, usize> {
        &self.entity_to_index
    }

    /// The number of index slots allocated (including gaps from removals).
    pub fn slot_count(&self) -> usize {
        self.next_index
    }
}
