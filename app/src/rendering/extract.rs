use crate::{
    material::Material,
    mesh::Mesh,
    transform::{GlobalTransform, Transform},
};
use ecs::{Entity, World};
use wgpu::Device;

/// Trait for extracting renderable data from ECS World
pub trait Extract {
    type ExtractedData;

    fn extract(
        &self,
        device: &Device,
        world: &World,
    ) -> Result<Self::ExtractedData, ExtractionError>;
}

#[derive(Debug)]
pub enum ExtractionError {
    BorrowConflict(String),
    MissingComponent(Entity, String),
    InvalidMaterialReference(Entity),
    Misc(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionError::BorrowConflict(msg) => write!(f, "Borrow conflict: {msg}"),
            ExtractionError::MissingComponent(entity, component) => {
                write!(f, "Entity {entity:?} missing component: {component}")
            }
            ExtractionError::InvalidMaterialReference(entity) => {
                write!(f, "Entity {entity:?} has invalid material reference")
            }
            ExtractionError::Misc(msg) => write!(f, "Extraction error: {msg}"),
        }
    }
}

impl std::error::Error for ExtractionError {}

pub trait WorldExtractExt {
    fn get_materials(&self) -> Vec<Entity>;
    fn get_renderables(&self) -> Vec<Entity>;

    fn extract_material_component(&self, entity: Entity) -> Result<Material, ExtractionError>;
    fn extract_transform_component(&self, entity: Entity) -> Result<Transform, ExtractionError>;
    fn extract_global_transform_component(
        &self,
        entity: Entity,
    ) -> Result<GlobalTransform, ExtractionError>;
    fn extract_mesh_component(&self, entity: Entity) -> Result<Mesh, ExtractionError>;
    fn extract_mesh_material(&self, mesh: &Mesh) -> Result<Material, ExtractionError>;
}

macro_rules! extract_component {
    ($self:expr, $entity:expr, $type:ty, $name:expr) => {{
        $self
            .get_component::<$type>($entity)
            .ok_or_else(|| ExtractionError::MissingComponent($entity, $name.to_string()))
            .map(|c| c.clone())
    }};
}

impl WorldExtractExt for World {
    fn get_materials(&self) -> Vec<Entity> {
        self.get_entities_with::<Material>().into_iter().collect()
    }

    fn extract_material_component(&self, entity: Entity) -> Result<Material, ExtractionError> {
        extract_component!(self, entity, Material, "Material")
    }

    fn get_renderables(&self) -> Vec<Entity> {
        self.get_entities_with_2::<Transform, Mesh>()
            .into_iter()
            .collect()
    }

    fn extract_transform_component(&self, entity: Entity) -> Result<Transform, ExtractionError> {
        // Transform is Copy, so we can dereference it directly
        self.get_component::<Transform>(entity)
            .ok_or_else(|| ExtractionError::MissingComponent(entity, "Transform".to_string()))
            .map(|c| *c)
    }

    fn extract_global_transform_component(
        &self,
        entity: Entity,
    ) -> Result<GlobalTransform, ExtractionError> {
        // GlobalTransform is Copy
        self.get_component::<GlobalTransform>(entity)
            .ok_or_else(|| ExtractionError::MissingComponent(entity, "GlobalTransform".to_string()))
            .map(|c| *c)
    }

    fn extract_mesh_component(&self, entity: Entity) -> Result<Mesh, ExtractionError> {
        extract_component!(self, entity, Mesh, "Mesh")
    }

    fn extract_mesh_material(&self, mesh: &Mesh) -> Result<Material, ExtractionError> {
        let Some(material_entity) = mesh.material_entity else {
            return Ok(Material::default());
        };

        extract_component!(self, material_entity, Material, "Material")
    }
}
