use ecs::{Component, Entity};
use maths::Vec4;

use crate::rendering::wgpu::Index;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
}

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<Index>>,
    pub material_entity: Option<Entity>,
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vertex>,
        indices: Option<Vec<Index>>,
        material_entity: Option<Entity>,
    ) -> Self {
        Self {
            vertices,
            indices,
            material_entity,
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> Option<&[Index]> {
        self.indices.as_deref()
    }
}

impl Component for Mesh {}

pub mod gltf;
pub mod static_mesh;
