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
    pub indices: Vec<Index>,
    pub material_entity: Entity, // TODO: Make this optional
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<Index>, material_entity: Entity) -> Self {
        Self {
            vertices,
            indices,
            material_entity,
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[Index] {
        &self.indices
    }
}

impl Component for Mesh {}

pub mod gltf;
pub mod static_mesh;
