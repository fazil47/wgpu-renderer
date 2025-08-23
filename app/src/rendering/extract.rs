use crate::{
    mesh::Mesh,
    rendering::{MaterialRef, Renderable, Transform},
};
use ecs::{EntityId, World};

/// Trait for extracting renderable data from ECS World
pub trait Extract {
    type ExtractedData;

    fn extract(&self, world: &World) -> Result<Self::ExtractedData, ExtractionError>;
}

#[derive(Debug)]
pub enum ExtractionError {
    BorrowConflict(String),
    MissingComponent(EntityId, String),
    InvalidMaterialReference(EntityId),
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

pub struct RenderableEntity {
    pub entity_id: EntityId,
    pub transform: Transform,
    pub mesh: Mesh,
    pub material_entity: EntityId,
    pub material_index: usize,
}

pub fn query_renderable_entities(world: &World) -> Vec<EntityId> {
    world
        .get_entities_with_3::<Transform, Mesh, MaterialRef>()
        .into_iter()
        .filter(|&entity_id| world.has_component::<Renderable>(entity_id))
        .collect()
}

pub fn extract_entity_components(
    world: &World,
    entity_id: EntityId,
) -> Result<(Transform, Mesh, EntityId), ExtractionError> {
    let transform_component = world
        .get_component::<Transform>(entity_id)
        .ok_or_else(|| ExtractionError::MissingComponent(entity_id, "Transform".to_string()))?;

    let mesh_component = world
        .get_component::<Mesh>(entity_id)
        .ok_or_else(|| ExtractionError::MissingComponent(entity_id, "RenderMesh".to_string()))?;

    let material_ref_component = world
        .get_component::<MaterialRef>(entity_id)
        .ok_or_else(|| ExtractionError::MissingComponent(entity_id, "MaterialRef".to_string()))?;

    let transform = *transform_component
        .try_borrow()
        .map_err(|_| ExtractionError::BorrowConflict("Transform".to_string()))?;

    let mesh = mesh_component
        .try_borrow()
        .map_err(|_| ExtractionError::BorrowConflict("RenderMesh".to_string()))?
        .clone();

    let material_entity = material_ref_component
        .try_borrow()
        .map_err(|_| ExtractionError::BorrowConflict("MaterialRef".to_string()))?
        .material_entity;

    Ok((transform, mesh, material_entity))
}
