use crate::rendering::wgpu_utils::RGBA;
use ecs::{Component, EntityId};

/// Material component for rendering properties
#[derive(Debug, Clone)]
pub struct Material {
    pub color: RGBA,
}

impl Material {
    pub fn new(color: RGBA) -> Self {
        Self { color }
    }
}

impl Component for Material {}

/// Component that references a material entity
#[derive(Debug, Clone, Copy)]
pub struct MaterialRef {
    pub material_entity: EntityId,
}

impl MaterialRef {
    pub fn new(material_entity: EntityId) -> Self {
        Self { material_entity }
    }
}

impl Component for MaterialRef {}

/// Component that stores the stable Scene material index for consistent rendering
#[derive(Debug, Clone, Copy)]
pub struct MaterialIndex(pub usize);

impl MaterialIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

impl Component for MaterialIndex {}
