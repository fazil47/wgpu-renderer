use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Marker trait for ECS components
pub trait Component: 'static {}

/// Marker trait for ECS resources
pub trait Resource: 'static {}

/// A unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

impl Deref for EntityId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The main ECS world that holds all entities and their components
pub struct World {
    entities: HashMap<EntityId, Entity>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Resource>>>,
    next_id: u32,
}

/// An entity is just a collection of components
pub struct Entity {
    pub id: EntityId,
    components: HashMap<TypeId, Rc<RefCell<dyn Component>>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            resources: HashMap::new(),
            next_id: 0,
        }
    }

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

    pub fn add_component<T: Component>(&mut self, entity_id: EntityId, component: T) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity
                .components
                .insert(TypeId::of::<T>(), Rc::new(RefCell::new(component)));
        }
    }

    pub fn get_component<T: Component>(&self, entity_id: EntityId) -> Option<Rc<RefCell<T>>> {
        if let Some(entity) = self.entities.get(&entity_id) {
            if let Some(component) = entity.components.get(&TypeId::of::<T>()) {
                let any_rc = component.clone();
                let concrete_rc =
                    unsafe { Rc::from_raw(Rc::into_raw(any_rc) as *const RefCell<T>) };
                Some(concrete_rc)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn has_component<T: Component>(&self, entity_id: EntityId) -> bool {
        if let Some(entity) = self.entities.get(&entity_id) {
            entity.components.contains_key(&TypeId::of::<T>())
        } else {
            false
        }
    }

    pub fn get_entities_with<T: Component>(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| entity.components.contains_key(&TypeId::of::<T>()))
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_entities_with_2<T1: Component, T2: Component>(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| {
                entity.components.contains_key(&TypeId::of::<T1>())
                    && entity.components.contains_key(&TypeId::of::<T2>())
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_entities_with_3<T1: Component, T2: Component, T3: Component>(
        &self,
    ) -> Vec<EntityId> {
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

    pub fn remove_entity(&mut self, entity_id: EntityId) {
        self.entities.remove(&entity_id);
    }

    pub fn get_all_entities(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }

    pub fn get_resource(&self, type_id: TypeId) -> Option<Rc<RefCell<dyn Resource>>> {
        self.resources.get(&type_id).cloned()
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources
            .insert(resource.type_id(), Rc::new(RefCell::new(resource)));
    }
}
