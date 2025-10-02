use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Marker trait for ECS components
pub trait Component: 'static + Any {}

impl<T> Component for Rc<T> where T: Component {}

/// Marker trait for ECS resources
pub trait Resource: 'static + Any {}

impl<T> Resource for Rc<T> where T: Resource {}

/// A unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u32);

impl Deref for Entity {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Entity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub type EntityComponents = HashMap<TypeId, Rc<RefCell<dyn Component>>>;

/// The main ECS world that holds all entities and their components
pub struct World {
    entities: HashMap<Entity, EntityComponents>,
    resources: HashMap<TypeId, Rc<RefCell<dyn Resource>>>,
    next_id: u32,
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

    pub fn create_entity(&mut self) -> Entity {
        let id = Entity(self.next_id);
        self.next_id += 1;

        let components = HashMap::new();

        self.entities.insert(id, components);
        id
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        if let Some(entity) = self.entities.get_mut(&entity) {
            entity.insert(TypeId::of::<T>(), Rc::new(RefCell::new(component)));
        }
    }

    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<Rc<RefCell<T>>> {
        if let Some(components) = self.entities.get(&entity) {
            if let Some(component) = components.get(&TypeId::of::<T>()) {
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

    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        if let Some(entity) = self.entities.get(&entity) {
            entity.contains_key(&TypeId::of::<T>())
        } else {
            false
        }
    }

    pub fn get_entities_with<T: Component>(&self) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|(_, components)| components.contains_key(&TypeId::of::<T>()))
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_entities_with_2<T1: Component, T2: Component>(&self) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|(_, components)| {
                components.contains_key(&TypeId::of::<T1>())
                    && components.contains_key(&TypeId::of::<T2>())
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_entities_with_3<T1: Component, T2: Component, T3: Component>(&self) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|(_, components)| {
                components.contains_key(&TypeId::of::<T1>())
                    && components.contains_key(&TypeId::of::<T2>())
                    && components.contains_key(&TypeId::of::<T3>())
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.entities.remove(&entity);
    }

    pub fn get_all_entities(&self) -> Vec<Entity> {
        self.entities.keys().copied().collect()
    }

    pub fn get_resource<T: 'static + Resource>(&self) -> Option<Ref<T>> {
        let type_id = TypeId::of::<T>();
        let downcasted = Ref::map(self.resources.get(&type_id)?.borrow(), |r| {
            let as_any = r as &dyn Any;
            as_any.downcast_ref::<T>().unwrap()
        });

        Some(downcasted)
    }

    pub fn get_resource_mut<T: 'static + Resource>(&self) -> Option<RefMut<T>> {
        let type_id = TypeId::of::<T>();
        let downcasted = RefMut::map(self.resources.get(&type_id)?.borrow_mut(), |r| {
            let as_any = r as &mut dyn Any;
            as_any.downcast_mut::<T>().unwrap()
        });

        Some(downcasted)
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources
            .insert(resource.type_id(), Rc::new(RefCell::new(resource)));
    }
}
