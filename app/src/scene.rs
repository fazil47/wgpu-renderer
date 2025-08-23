use crate::{
    mesh::Material as MeshMaterial, rendering::material::Material, rendering::wgpu_utils::WgpuExt,
};
use ecs::{EntityId, Resource};
/// CPU-side Scene for organizing Meshes and Materials
/// Separates asset management from ECS components and GPU resources
use std::collections::HashMap;

#[derive(Clone)]
pub struct Scene {
    /// All materials in the scene, indexed by MaterialId
    materials: HashMap<MaterialId, Material>,
    /// Next available material ID
    next_material_id: MaterialId,
    /// Mapping from Scene MaterialId to ECS material entity
    material_entities: HashMap<MaterialId, EntityId>,
    /// Reverse mapping from ECS material entity to Scene MaterialId
    entity_to_material_id: HashMap<EntityId, MaterialId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(pub u32);

impl Scene {
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            next_material_id: MaterialId(0),
            material_entities: HashMap::new(),
            entity_to_material_id: HashMap::new(),
        }
    }

    /// Add a material to the scene and return its ID
    pub fn add_material(&mut self, material: Material) -> MaterialId {
        let id = self.next_material_id;
        self.materials.insert(id, material);
        self.next_material_id = MaterialId(id.0 + 1);
        id
    }

    /// Get a material by ID
    pub fn get_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    /// Load materials from GLTF and create ECS entities with consistent indexing
    pub fn load_from_materials(
        &mut self,
        materials: Vec<MeshMaterial>,
        world: &mut ecs::World,
    ) -> Vec<MaterialId> {
        let mut material_ids = Vec::new();

        for material in materials {
            // Create material once per unique material
            let material_id = self.add_material(Material::new(material.color));
            material_ids.push(material_id);

            // Create material entity in ECS
            let material_entity = world.create_entity();
            world.add_component(material_entity, Material::new(material.color));

            // Register the bidirectional mapping between Scene MaterialId and ECS entity
            self.register_material_entity(material_id, material_entity);

            // Convert each mesh in the material to ECS entities
            for mesh in material.meshes {
                let mesh_entity = world.create_entity();
                world.add_component(
                    mesh_entity,
                    crate::rendering::Transform::new(maths::Vec3::ZERO),
                );
                world.add_component(mesh_entity, mesh); // Use the original mesh directly
                world.add_component(
                    mesh_entity,
                    crate::rendering::MaterialRef::new(material_entity),
                );

                // Add MaterialIndex component for stable indexing
                if let Some(material_index) = self.get_material_index_for_entity(material_entity) {
                    world.add_component(
                        mesh_entity,
                        crate::rendering::MaterialIndex::new(material_index),
                    );
                }

                world.add_component(mesh_entity, crate::rendering::Renderable);
            }
        }

        material_ids
    }

    /// Create material bind groups for rendering
    pub fn create_material_bind_groups(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Vec<wgpu::BindGroup> {
        let mut bind_groups = Vec::new();

        // Sort materials by ID to ensure consistent ordering
        let mut sorted_materials: Vec<_> = self.materials.iter().collect();
        sorted_materials.sort_by_key(|(id, _)| id.0);

        for (_, material) in sorted_materials {
            let color_array = material.color.to_array();
            let material_buffer = device
                .buffer()
                .label("Material Buffer")
                .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
                .uniform(&color_array);

            let bind_group = device
                .bind_group(layout)
                .label("Material Bind Group")
                .buffer(0, &material_buffer)
                .build();

            bind_groups.push(bind_group);
        }

        bind_groups
    }

    /// Get number of materials
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// Register a material entity with its Scene MaterialId for consistent indexing
    pub fn register_material_entity(&mut self, material_id: MaterialId, entity_id: EntityId) {
        self.material_entities.insert(material_id, entity_id);
        self.entity_to_material_id.insert(entity_id, material_id);
    }

    /// Get the stable Scene material index for a material entity
    /// This is the single source of truth for material indexing used by both renderers
    pub fn get_material_index_for_entity(&self, material_entity: EntityId) -> Option<usize> {
        let material_id = self.entity_to_material_id.get(&material_entity)?;
        let sorted_materials = self.sorted_materials();

        // Find the index of this material in the sorted list
        sorted_materials
            .iter()
            .position(|(id, _)| **id == *material_id)
    }

    /// Get the material entity for a given MaterialId
    pub fn get_material_entity(&self, material_id: MaterialId) -> Option<EntityId> {
        self.material_entities.get(&material_id).copied()
    }

    /// Get the MaterialId for a given material entity
    pub fn get_material_id_for_entity(&self, entity_id: EntityId) -> Option<MaterialId> {
        self.entity_to_material_id.get(&entity_id).copied()
    }

    /// Get materials in sorted order (consistent ordering for both renderers)
    pub fn sorted_materials(&self) -> Vec<(&MaterialId, &Material)> {
        let mut sorted_materials: Vec<_> = self.materials.iter().collect();
        sorted_materials.sort_by_key(|(id, _)| *id);
        sorted_materials
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Resource for Scene {}
