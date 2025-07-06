use winit::dpi::PhysicalSize;

use crate::{
    ecs::{
        CameraComponent, DirectionalLightComponent, EntityId, MaterialComponent, MeshComponent,
        Renderable, Transform, World, MaterialRef,
    },
    mesh::{Material, Mesh},
    rendering::wgpu::{RGBA, RaytracerMaterial},
    wgpu_utils::WgpuExt,
};

use maths::Vec3;

/// ECS-based scene that replaces the old monolithic Scene
pub struct EcsScene {
    pub world: World,
    pub camera_entity: EntityId,
    pub sun_light_entity: EntityId,
    is_light_dirty: bool,
}

impl EcsScene {
    pub fn new(window_size: &PhysicalSize<u32>) -> Self {
        let mut world = World::new();

        // Create camera entity
        let camera_entity = world.create_entity();
        let camera_position = Vec3::new(0.0, 0.0, 4.0);
        world.add_component(
            camera_entity,
            Transform::new(camera_position),
        );
        world.add_component(
            camera_entity,
            CameraComponent::new(
                camera_position,
                -camera_position.normalized(), // look at origin
                window_size.width as f32 / window_size.height as f32,
                45.0,
                0.1,
                100.0,
            ),
        );

        // Create sun light entity
        let sun_light_entity = world.create_entity();
        world.add_component(
            sun_light_entity,
            Transform::new(Vec3::ZERO),
        );
        world.add_component(
            sun_light_entity,
            DirectionalLightComponent::new(45.0, 45.0),
        );

        // Note: Mesh entities will be created later from GLTF loading or other sources

        Self {
            world,
            camera_entity,
            sun_light_entity,
            is_light_dirty: false,
        }
    }

    /// Convert old Material system to separate mesh and material entities
    pub fn create_mesh_entities_from_materials(world: &mut World, materials: Vec<Material>) {
        for material in materials {
            // First, create a material entity
            let material_entity = world.create_entity();
            world.add_component(material_entity, MaterialComponent::new(material.color));
            
            // Then create separate mesh entities for each mesh in the material
            for mesh in material.meshes {
                let (vertices, indices) = mesh.into_parts();
                let mesh_entity = world.create_entity();
                
                // Add transform component (each mesh can have its own transform)
                world.add_component(mesh_entity, Transform::new(Vec3::ZERO));
                
                // Add mesh geometry
                world.add_component(mesh_entity, MeshComponent::new(vertices, indices));
                
                // Reference the material entity instead of copying material data
                world.add_component(mesh_entity, MaterialRef::new(material_entity));
                
                // Mark as renderable
                world.add_component(mesh_entity, Renderable);
            }
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        if let Some(camera) = self.world.get_component::<CameraComponent>(self.camera_entity) {
            camera.borrow_mut().aspect = aspect;
        }
    }

    pub fn run_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.collapsing("Lighting", |ui| {
            if let Some(light) = self.world.get_component::<DirectionalLightComponent>(self.sun_light_entity) {
                let mut light_ref = light.borrow_mut();
                
                let sun_azi_changed = ui
                    .add(
                        egui::Slider::new(&mut light_ref.azimuth, 0.0..=360.0).text("Sun Azimuth"),
                    )
                    .changed();
                let sun_alt_changed = ui
                    .add(
                        egui::Slider::new(&mut light_ref.altitude, 0.0..=90.0)
                            .text("Sun Altitude"),
                    )
                    .changed();
                    
                if sun_azi_changed || sun_alt_changed {
                    light_ref.recalculate();
                    self.is_light_dirty = true;
                    changed = true;
                }
            }
        });

        changed
    }

    pub fn is_light_dirty(&self) -> bool {
        self.is_light_dirty
    }

    pub fn set_light_clean(&mut self) {
        self.is_light_dirty = false;
    }

    /// Get all renderable entities (entities with Transform, MeshComponent, MaterialRef, Renderable)
    pub fn get_renderable_entities(&self) -> Vec<EntityId> {
        self.world.get_entities_with_3::<Transform, MeshComponent, MaterialRef>()
            .into_iter()
            .filter(|&entity_id| self.world.has_component::<Renderable>(entity_id))
            .collect()
    }

    /// Create a new mesh entity with a new material
    pub fn create_mesh_entity(&mut self, vertices: Vec<crate::rendering::wgpu::Vertex>, indices: Vec<crate::rendering::wgpu::Index>, color: RGBA) -> EntityId {
        // Create material entity
        let material_entity = self.world.create_entity();
        self.world.add_component(material_entity, MaterialComponent::new(color));
        
        // Create mesh entity
        let mesh_entity = self.world.create_entity();
        self.world.add_component(mesh_entity, Transform::new(Vec3::ZERO));
        self.world.add_component(mesh_entity, MeshComponent::new(vertices, indices));
        self.world.add_component(mesh_entity, MaterialRef::new(material_entity));
        self.world.add_component(mesh_entity, Renderable);
        mesh_entity
    }

    /// Create a new mesh entity with an existing material
    pub fn create_mesh_entity_with_material(&mut self, vertices: Vec<crate::rendering::wgpu::Vertex>, indices: Vec<crate::rendering::wgpu::Index>, material_entity: EntityId) -> EntityId {
        let mesh_entity = self.world.create_entity();
        self.world.add_component(mesh_entity, Transform::new(Vec3::ZERO));
        self.world.add_component(mesh_entity, MeshComponent::new(vertices, indices));
        self.world.add_component(mesh_entity, MaterialRef::new(material_entity));
        self.world.add_component(mesh_entity, Renderable);
        mesh_entity
    }

    /// Create a new material entity
    pub fn create_material_entity(&mut self, color: RGBA) -> EntityId {
        let material_entity = self.world.create_entity();
        self.world.add_component(material_entity, MaterialComponent::new(color));
        material_entity
    }

    /// Set transform for an entity
    pub fn set_transform(&mut self, entity: EntityId, position: Vec3, rotation: maths::Quat, scale: Vec3) {
        if let Some(transform) = self.world.get_component::<Transform>(entity) {
            let mut transform_ref = transform.borrow_mut();
            transform_ref.position = position;
            transform_ref.rotation = rotation;
            transform_ref.scale = scale;
        }
    }

    // ECS-native camera access methods
    
    /// Get camera component
    pub fn get_camera_component(&self) -> Option<std::rc::Rc<std::cell::RefCell<CameraComponent>>> {
        self.world.get_component::<CameraComponent>(self.camera_entity)
    }

    /// Get sun light component  
    pub fn get_sun_light_component(&self) -> Option<std::rc::Rc<std::cell::RefCell<DirectionalLightComponent>>> {
        self.world.get_component::<DirectionalLightComponent>(self.sun_light_entity)
    }

    // ECS-native methods (not compatibility layer)

    /// Set material color
    pub fn set_material_color(&mut self, material_entity: EntityId, color: RGBA) {
        if let Some(material) = self.world.get_component::<MaterialComponent>(material_entity) {
            material.borrow_mut().color = color;
        }
    }

    /// Get all material entities
    pub fn get_material_entities(&self) -> Vec<EntityId> {
        self.world.get_entities_with::<MaterialComponent>()
    }

    /// Example method to demonstrate ECS flexibility - animate transforms
    pub fn animate_transforms(&mut self, time: f32) {
        let renderable_entities = self.get_renderable_entities();
        
        for (i, entity_id) in renderable_entities.iter().enumerate() {
            if let Some(transform) = self.world.get_component::<Transform>(*entity_id) {
                let mut transform_mut = transform.borrow_mut();
                
                // Example: slight rotation and scale animation
                let phase = time + i as f32 * 0.5;
                transform_mut.rotation = maths::Quat::from_rotation_y(phase * 0.1);
                
                // Small scale oscillation 
                let scale_factor = 1.0 + 0.05 * (phase * 2.0).sin();
                transform_mut.scale = Vec3::new(scale_factor, scale_factor, scale_factor);
            }
        }
    }

    /// Get transform component for an entity
    pub fn get_transform(&self, entity_id: EntityId) -> Option<std::rc::Rc<std::cell::RefCell<Transform>>> {
        self.world.get_component::<Transform>(entity_id)
    }

    /// Get mesh component for an entity  
    pub fn get_mesh(&self, entity_id: EntityId) -> Option<std::rc::Rc<std::cell::RefCell<MeshComponent>>> {
        self.world.get_component::<MeshComponent>(entity_id)
    }

    // Direct ECS-to-GPU buffer creation methods

    /// Create materials buffer directly from ECS material entities
    pub fn create_materials_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut materials = Vec::new();
        
        // Collect all material entities with stable ordering
        let mut material_entities = self.get_material_entities();
        material_entities.sort_by_key(|entity| entity.0); // Sort for stable ordering
        
        for material_entity in material_entities {
            if let Some(material_comp) = self.world.get_component::<MaterialComponent>(material_entity) {
                let material = material_comp.borrow();
                materials.push(RaytracerMaterial {
                    color: material.color.to_array(),
                });
            }
        }

        device
            .buffer()
            .label("ECS Materials Buffer")
            .storage(&materials)
    }

    /// Create vertices buffer directly from ECS mesh entities
    pub fn create_vertices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut all_vertices = Vec::new();
        let renderable_entities = self.get_renderable_entities();
        
        // Create a mapping from material entity to material index
        let mut material_entities = self.get_material_entities();
        material_entities.sort_by_key(|entity| entity.0); // Sort for stable ordering
        let material_entity_to_index: std::collections::HashMap<EntityId, usize> = 
            material_entities.iter().enumerate().map(|(i, &entity)| (entity, i)).collect();
        
        for entity_id in renderable_entities {
            if let (Some(mesh_comp), Some(material_ref)) = (
                self.world.get_component::<MeshComponent>(entity_id),
                self.world.get_component::<MaterialRef>(entity_id),
            ) {
                let mesh = mesh_comp.borrow();
                let mat_ref = material_ref.borrow();
                
                // Get the correct material ID from the mapping
                let material_id = material_entity_to_index.get(&mat_ref.material_entity)
                    .copied().unwrap_or(0);
                
                // Convert vertices and add material_id
                for vertex in &mesh.vertices {
                    all_vertices.push(crate::rendering::wgpu::RaytracerVertex::from_vertex(vertex, material_id));
                }
            }
        }

        device
            .buffer()
            .label("ECS Vertices Buffer")
            .storage(&all_vertices)
    }

    /// Create indices buffer directly from ECS mesh entities
    pub fn create_indices_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut all_indices = Vec::new();
        let mut vertex_offset = 0u32;
        let renderable_entities = self.get_renderable_entities();
        
        for entity_id in renderable_entities {
            if let Some(mesh_comp) = self.world.get_component::<MeshComponent>(entity_id) {
                let mesh = mesh_comp.borrow();
                // Add indices with vertex offset
                all_indices.extend(mesh.indices.iter().map(|&idx| idx + vertex_offset));
                vertex_offset += mesh.vertices.len() as u32;
            }
        }

        device
            .buffer()
            .label("ECS Indices Buffer") 
            .storage(&all_indices)
    }

    /// Create material bind groups directly from ECS material entities
    pub fn create_material_bind_groups(&self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Vec<wgpu::BindGroup> {
        let mut bind_groups = Vec::new();
        
        // Use same stable ordering as other methods
        let mut material_entities = self.get_material_entities();
        material_entities.sort_by_key(|entity| entity.0);
        
        for material_entity in material_entities {
            if let Some(material_comp) = self.world.get_component::<MaterialComponent>(material_entity) {
                let material = material_comp.borrow();
                let color_buffer = device
                    .buffer()
                    .label("Material Color Buffer")
                    .uniform(&material.color.to_array());
                
                let bind_group = device
                    .bind_group(layout)
                    .label("Material Bind Group")
                    .buffer(0, &color_buffer)
                    .build();
                
                bind_groups.push(bind_group);
            }
        }
        
        bind_groups
    }

    /// Get materials in Vec<Material> format for renderer compatibility
    pub fn get_materials_vec(&self) -> Vec<Material> {
        use std::collections::HashMap;
        
        let mut material_groups: HashMap<EntityId, Vec<Mesh>> = HashMap::new();
        let mut material_colors: HashMap<EntityId, RGBA> = HashMap::new();
        
        // Query all renderable entities
        let renderable_entities = self.get_renderable_entities();
        
        for entity_id in renderable_entities {
            if let (Some(mesh_component), Some(material_ref)) = (
                self.world.get_component::<MeshComponent>(entity_id),
                self.world.get_component::<MaterialRef>(entity_id),
            ) {
                let mesh = mesh_component.borrow();
                let mat_ref = material_ref.borrow();
                
                // Get the material component
                if let Some(material_component) = self.world.get_component::<MaterialComponent>(mat_ref.material_entity) {
                    let material_comp = material_component.borrow();
                    material_colors.insert(mat_ref.material_entity, material_comp.color);
                    
                    // Group meshes by material entity
                    let mesh_data = Mesh::new(mesh.vertices.clone(), mesh.indices.clone());
                    material_groups.entry(mat_ref.material_entity)
                        .or_insert_with(Vec::new)
                        .push(mesh_data);
                }
            }
        }
        
        // Convert grouped meshes back to Material format with stable ordering
        let mut material_entities: Vec<_> = material_groups.keys().copied().collect();
        material_entities.sort_by_key(|entity| entity.0); // Sort by entity ID for stable ordering
        
        material_entities.into_iter().map(|material_entity| {
            let meshes = material_groups.remove(&material_entity).unwrap();
            let color = material_colors.get(&material_entity).copied().unwrap_or(RGBA::new([1.0, 1.0, 1.0, 1.0]));
            let mut material = Material::new(color);
            for mesh in meshes {
                material.add_mesh(mesh);
            }
            material
        }).collect()
    }
}