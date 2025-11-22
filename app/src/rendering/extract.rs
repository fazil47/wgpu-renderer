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

impl WorldExtractExt for World {
    fn get_materials(&self) -> Vec<Entity> {
        self.get_entities_with::<Material>().into_iter().collect()
    }

    fn extract_material_component(&self, entity: Entity) -> Result<Material, ExtractionError> {
        let material = self
            .get_component::<Material>(entity)
            .ok_or_else(|| ExtractionError::MissingComponent(entity, "Material".to_string()))?;
        let material: Material = material.clone();

        Ok(material)
    }

    fn get_renderables(&self) -> Vec<Entity> {
        self.get_entities_with_2::<Transform, Mesh>()
            .into_iter()
            .collect()
    }

    fn extract_transform_component(&self, entity: Entity) -> Result<Transform, ExtractionError> {
        let transform = self
            .get_component::<Transform>(entity)
            .ok_or_else(|| ExtractionError::MissingComponent(entity, "Transform".to_string()))?;
        let transform: Transform = *transform;

        Ok(transform)
    }

    fn extract_global_transform_component(
        &self,
        entity: Entity,
    ) -> Result<GlobalTransform, ExtractionError> {
        let global_transform = self
            .get_component::<GlobalTransform>(entity)
            .ok_or_else(|| {
                ExtractionError::MissingComponent(entity, "GlobalTransform".to_string())
            })?;
        let global_transform: GlobalTransform = *global_transform;

        Ok(global_transform)
    }

    fn extract_mesh_component(&self, entity: Entity) -> Result<Mesh, ExtractionError> {
        let mesh = self
            .get_component::<Mesh>(entity)
            .ok_or_else(|| ExtractionError::MissingComponent(entity, "Mesh".to_string()))?;
        let mesh: Mesh = mesh.clone();

        Ok(mesh)
    }

    fn extract_mesh_material(&self, mesh: &Mesh) -> Result<Material, ExtractionError> {
        let material_entity = mesh.material_entity;
        if material_entity.is_none() {
            return Ok(Material::default());
        }

        let material_entity = material_entity.unwrap();
        let material = self
            .get_component::<Material>(material_entity)
            .ok_or_else(|| {
                ExtractionError::MissingComponent(material_entity, "Material".to_string())
            })?;
        let material: Material = material.clone();

        Ok(material)
    }
}
