use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
};

use indexmap::IndexSet;

/// Marker trait for ECS components
pub trait Component: 'static + Any {}

impl<T> Component for Box<T> where T: Component {}

/// Marker trait for ECS resources
pub trait Resource: 'static + Any {}

impl<T> Resource for Box<T> where T: Resource {}

/// A unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

pub type EntityComponents = HashMap<TypeId, Box<RefCell<dyn Component>>>;

// ── World ───────────────────────────────────────────────────────────────────

/// The main ECS world that holds all entities and their components
pub struct World {
    entities: HashMap<Entity, EntityComponents>,
    resources: HashMap<TypeId, Box<RefCell<dyn Resource>>>,
    events: HashMap<TypeId, Box<dyn AnyEventStorage>>,
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
            events: HashMap::new(),
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
            entity.insert(TypeId::of::<T>(), Box::new(RefCell::new(component)));
        }
    }

    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        if let Some(components) = self.entities.get(&entity) {
            if let Some(component) = components.get(&TypeId::of::<T>()) {
                let downcasted = Ref::map(component.borrow(), |c| {
                    let as_any = c as &dyn Any;
                    as_any.downcast_ref::<T>().unwrap()
                });
                Some(downcasted)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_component_mut<T: Component>(&self, entity: Entity) -> Option<RefMut<'_, T>> {
        if let Some(components) = self.entities.get(&entity) {
            if let Some(component) = components.get(&TypeId::of::<T>()) {
                let downcasted = RefMut::map(component.borrow_mut(), |c| {
                    let as_any = c as &mut dyn Any;
                    as_any.downcast_mut::<T>().unwrap()
                });
                Some(downcasted)
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

    pub fn get_resource<T: 'static + Resource>(&self) -> Option<Ref<'_, T>> {
        let type_id = TypeId::of::<T>();
        let downcasted = Ref::map(self.resources.get(&type_id)?.borrow(), |r| {
            let as_any = r as &dyn Any;
            as_any.downcast_ref::<T>().unwrap()
        });

        Some(downcasted)
    }

    pub fn get_resource_mut<T: 'static + Resource>(&self) -> Option<RefMut<'_, T>> {
        let type_id = TypeId::of::<T>();
        let downcasted = RefMut::map(self.resources.get(&type_id)?.borrow_mut(), |r| {
            let as_any = r as &mut dyn Any;
            as_any.downcast_mut::<T>().unwrap()
        });

        Some(downcasted)
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        // TODO: Maybe disallow inserting a resource that already exists?
        self.resources
            .insert(resource.type_id(), Box::new(RefCell::new(resource)));
    }

    // ── Event methods ───────────────────────────────────────────────────

    /// Send an event. Deduplicated: sending the same event twice in a frame is a no-op.
    pub fn send_event<E: Event>(&mut self, event: E) {
        let storage = self
            .events
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(EventStorage::<E>::new()));

        let storage = storage
            .as_any_mut()
            .downcast_mut::<EventStorage<E>>()
            .unwrap();

        storage.events.insert(event);
    }

    /// Read all events of a given type this frame.
    pub fn read_events<E: Event>(&self) -> Option<&IndexSet<E>> {
        self.events
            .get(&TypeId::of::<E>())
            .and_then(|s| s.as_any().downcast_ref::<EventStorage<E>>())
            .map(|s| &s.events)
    }

    /// Check whether any events of a given type were sent this frame.
    pub fn has_events<E: Event>(&self) -> bool {
        self.events
            .get(&TypeId::of::<E>())
            .is_some_and(|s| s.len() > 0)
    }

    /// Clear all events of every type. Call at end of frame.
    pub fn clear_all_events(&mut self) {
        for storage in self.events.values_mut() {
            storage.clear();
        }
    }
}

pub type System = Box<dyn FnMut(&mut World)>;

pub struct Schedule {
    systems: Vec<System>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<S: FnMut(&mut World) + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system(world);
        }
    }
}

// ── Events ──────────────────────────────────────────────────────────────────

/// Marker trait for ECS events.
pub trait Event: 'static + Copy + Eq + Hash {}

/// An event that targets a specific entity.
pub trait EntityEvent: 'static + Copy + Eq + Hash {
    fn entity(&self) -> Entity;
}

impl<T: EntityEvent> Event for T {}

/// Type-erased storage so we can hold heterogeneous event queues in one map.
trait AnyEventStorage: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
    fn len(&self) -> usize;
}

/// Concrete, typed storage for a single event type.
struct EventStorage<E: Event> {
    events: IndexSet<E>,
}

impl<E: Event> EventStorage<E> {
    fn new() -> Self {
        Self {
            events: IndexSet::new(),
        }
    }
}

impl<E: Event> AnyEventStorage for EventStorage<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clear(&mut self) {
        self.events.clear();
    }

    fn len(&self) -> usize {
        self.events.len()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestComponent(u32);
    impl Component for TestComponent {}

    struct TestResource(u32);
    impl Resource for TestResource {}

    #[test]
    fn test_schedule_sanity() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(0));
        world.insert_resource(TestResource(0));

        let mut schedule = Schedule::new();

        // System that modifies a component
        schedule.add_system(move |world: &mut World| {
            if let Some(mut component) = world.get_component_mut::<TestComponent>(entity) {
                component.0 += 1;
            }
        });

        // System that modifies a resource
        schedule.add_system(|world: &mut World| {
            if let Some(mut resource) = world.get_resource_mut::<TestResource>() {
                resource.0 += 1;
            }
        });

        schedule.run(&mut world);

        let component = world.get_component::<TestComponent>(entity).unwrap();
        assert_eq!(component.0, 1);

        let resource = world.get_resource::<TestResource>().unwrap();
        assert_eq!(resource.0, 1);
    }

    #[test]
    fn test_schedule_mutable_ops() {
        let mut world = World::new();
        let mut schedule = Schedule::new();

        // System that creates an entity - this requires &mut World
        schedule.add_system(|world: &mut World| {
            let entity = world.create_entity();
            world.add_component(entity, TestComponent(10));
        });

        schedule.run(&mut world);

        assert_eq!(world.get_all_entities().len(), 1);
        let entities = world.get_entities_with::<TestComponent>();
        assert_eq!(entities.len(), 1);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct TestEvent(Entity);
    impl EntityEvent for TestEvent {
        fn entity(&self) -> Entity {
            self.0
        }
    }

    #[test]
    fn test_events_send_and_read() {
        let mut world = World::new();
        let e1 = world.create_entity();
        let e2 = world.create_entity();

        world.send_event(TestEvent(e1));
        world.send_event(TestEvent(e2));

        assert!(world.has_events::<TestEvent>());

        let events = world.read_events::<TestEvent>().unwrap();
        assert_eq!(events.len(), 2);

        assert!(events.contains(&TestEvent(e1)));
        assert!(events.contains(&TestEvent(e2)));
    }

    #[test]
    fn test_events_dedup() {
        let mut world = World::new();
        let e1 = world.create_entity();

        world.send_event(TestEvent(e1));
        world.send_event(TestEvent(e1));
        world.send_event(TestEvent(e1));

        let events = world.read_events::<TestEvent>().unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_events_clear() {
        let mut world = World::new();
        let e1 = world.create_entity();

        world.send_event(TestEvent(e1));
        assert!(world.has_events::<TestEvent>());

        world.clear_all_events();
        assert!(!world.has_events::<TestEvent>());
    }

    #[test]
    fn test_events_empty_by_default() {
        let world = World::new();
        assert!(!world.has_events::<TestEvent>());
        assert!(world.read_events::<TestEvent>().is_none());
    }

    #[test]
    fn test_events_insertion_order() {
        let mut world = World::new();
        let e1 = world.create_entity();
        let e2 = world.create_entity();
        let e3 = world.create_entity();

        world.send_event(TestEvent(e3));
        world.send_event(TestEvent(e1));
        world.send_event(TestEvent(e2));

        let events: Vec<TestEvent> = world
            .read_events::<TestEvent>()
            .unwrap()
            .iter()
            .copied()
            .collect();
        assert_eq!(events, vec![TestEvent(e3), TestEvent(e1), TestEvent(e2)]);
    }
}
