use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use maths::{Quat, Vec3};

use crate::rendering::{Index, Vertex, RGBA};

pub mod scene;

/// A unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// The main ECS world that holds all entities and their components
pub struct World {
    entities: HashMap<EntityId, Entity>,
    next_id: u32,
}

/// An entity is just a collection of components
pub struct Entity {
    pub id: EntityId,
    components: HashMap<TypeId, Rc<RefCell<dyn Any>>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 0,
        }
    }

    /// Create a new entity and return its ID
    pub fn create_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;

        let entity = Entity {
            id,
            components: HashMap::new(),
        };

        self.entities.insert(id, entity);
        id
    }

    /// Add a component to an entity
    pub fn add_component<T: 'static>(&mut self, entity_id: EntityId, component: T) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.components.insert(
                TypeId::of::<T>(),
                Rc::new(RefCell::new(component)),
            );
        }
    }

    /// Get a component from an entity
    pub fn get_component<T: 'static>(&self, entity_id: EntityId) -> Option<Rc<RefCell<T>>> {
        if let Some(entity) = self.entities.get(&entity_id) {
            if let Some(component) = entity.components.get(&TypeId::of::<T>()) {
                // This is safe because we only store T in the HashMap under TypeId::of::<T>()
                let any_rc = component.clone();
                let concrete_rc = unsafe {
                    Rc::from_raw(Rc::into_raw(any_rc) as *const RefCell<T>)
                };
                Some(concrete_rc)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if an entity has a specific component
    pub fn has_component<T: 'static>(&self, entity_id: EntityId) -> bool {
        if let Some(entity) = self.entities.get(&entity_id) {
            entity.components.contains_key(&TypeId::of::<T>())
        } else {
            false
        }
    }

    /// Get all entities that have the specified component
    pub fn get_entities_with<T: 'static>(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| entity.components.contains_key(&TypeId::of::<T>()))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all entities that have both specified components
    pub fn get_entities_with_2<T1: 'static, T2: 'static>(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| {
                entity.components.contains_key(&TypeId::of::<T1>())
                    && entity.components.contains_key(&TypeId::of::<T2>())
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all entities that have all three specified components
    pub fn get_entities_with_3<T1: 'static, T2: 'static, T3: 'static>(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| {
                entity.components.contains_key(&TypeId::of::<T1>())
                    && entity.components.contains_key(&TypeId::of::<T2>())
                    && entity.components.contains_key(&TypeId::of::<T3>())
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Remove an entity and all its components
    pub fn remove_entity(&mut self, entity_id: EntityId) {
        self.entities.remove(&entity_id);
    }

    /// Get all entity IDs
    pub fn get_all_entities(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }
}

// Component definitions

/// Transform component for position, rotation, and scale
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub parent: Option<EntityId>,
}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: None,
        }
    }

    pub fn with_parent(position: Vec3, parent: EntityId) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: Some(parent),
        }
    }
}

/// Mesh component containing geometry data
#[derive(Debug, Clone)]
pub struct MeshComponent {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
}

impl MeshComponent {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }
}

/// Material component for rendering properties
#[derive(Debug, Clone)]
pub struct MaterialComponent {
    pub color: RGBA,
}

impl MaterialComponent {
    pub fn new(color: RGBA) -> Self {
        Self { color }
    }
}

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

/// Camera component for view and projection
#[derive(Debug, Clone)]
pub struct CameraComponent {
    pub eye: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraComponent {
    pub fn new(eye: Vec3, forward: Vec3, aspect: f32, fov: f32, near: f32, far: f32) -> Self {
        Self {
            eye,
            forward,
            up: Vec3::Y,
            fov,
            aspect,
            near,
            far,
        }
    }

    pub fn view_projection(&self) -> maths::Mat4 {
        let (_, _, _, _, view_projection) = self.calculate_matrices();
        view_projection
    }

    pub fn view_matrix(&self) -> maths::Mat4 {
        let (world_to_camera, _, _, _, _) = self.calculate_matrices();
        world_to_camera
    }

    pub fn projection_matrix(&self) -> maths::Mat4 {
        let (_, _, camera_projection, _, _) = self.calculate_matrices();
        camera_projection
    }

    pub fn camera_to_world(&self) -> maths::Mat4 {
        let (_, camera_to_world, _, _, _) = self.calculate_matrices();
        camera_to_world
    }

    pub fn camera_inverse_projection(&self) -> maths::Mat4 {
        let (_, _, _, camera_inverse_projection, _) = self.calculate_matrices();
        camera_inverse_projection
    }

    fn calculate_matrices(&self) -> (maths::Mat4, maths::Mat4, maths::Mat4, maths::Mat4, maths::Mat4) {
        let right = self.forward.cross(self.up);

        let world_to_camera = maths::Mat4::from_cols(
            maths::Vec4::new(right.x, self.up.x, -self.forward.x, 0.0),
            maths::Vec4::new(right.y, self.up.y, -self.forward.y, 0.0),
            maths::Vec4::new(right.z, self.up.z, -self.forward.z, 0.0),
            maths::Vec4::new(-right.dot(self.eye), -self.up.dot(self.eye), self.forward.dot(self.eye), 1.0),
        );
        let camera_to_world = world_to_camera.inverse();

        let top = self.near * (self.fov / 2.0).tan();
        let right_proj = top * self.aspect;

        let camera_projection = maths::Mat4::from_cols(
            maths::Vec4::new(self.near / right_proj, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, self.near / top, 0.0, 0.0),
            maths::Vec4::new(0.0, 0.0, -(self.far + self.near) / (self.far - self.near), -1.0),
            maths::Vec4::new(0.0, 0.0, -(2.0 * self.far * self.near) / (self.far - self.near), 0.0),
        );
        let camera_inverse_projection = camera_projection.inverse();

        let view_projection = camera_projection * world_to_camera;

        (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        )
    }
}

/// Directional light component
#[derive(Debug, Clone)]
pub struct DirectionalLightComponent {
    pub direction: Vec3,
    pub azimuth: f32,
    pub altitude: f32,
}

impl DirectionalLightComponent {
    pub fn new(azimuth: f32, altitude: f32) -> Self {
        let mut light = Self {
            direction: Vec3::ZERO,
            azimuth,
            altitude,
        };
        light.recalculate();
        light
    }

    pub fn recalculate(&mut self) {
        let azi_rad = self.azimuth.to_radians();
        let alt_rad = self.altitude.to_radians();

        self.direction = Vec3::new(
            azi_rad.sin() * alt_rad.cos(),
            alt_rad.sin(),
            azi_rad.cos() * alt_rad.cos(),
        )
        .normalized();
    }
}

/// Tag component to mark entities as renderable
#[derive(Debug, Clone)]
pub struct Renderable;