use std::{
    any::{Any, TypeId},
    cell::{Cell, Ref, RefCell, RefMut},
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

/// Mutable reference to a component that tracks changes via ticks.
///
/// On [`DerefMut`], the component's change tick is stamped with the current
/// world tick. Read-only access through [`Deref`] does not mark the
/// component as changed.
pub struct Mut<'a, T: ?Sized> {
    value: RefMut<'a, T>,
    changed_tick: &'a Cell<u32>,
    world_tick: u32,
}

impl<T: ?Sized> Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.changed_tick.set(self.world_tick);
        &mut self.value
    }
}

struct ComponentEntry {
    data: Box<RefCell<dyn Component>>,
    changed_tick: Cell<u32>,
}

type EntityComponents = HashMap<TypeId, ComponentEntry>;

// ── World ───────────────────────────────────────────────────────────────────

/// The main ECS world that holds all entities and their components
pub struct World {
    entities: HashMap<Entity, EntityComponents>,
    resources: HashMap<TypeId, Box<RefCell<dyn Resource>>>,
    events: HashMap<TypeId, Box<dyn AnyEventStorage>>,
    hooks: HashMap<TypeId, ComponentHooks>,
    current_tick: u32,
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
            hooks: HashMap::new(),
            current_tick: 0,
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

    /// Register lifecycle hooks for a component type. Returns a mutable
    /// reference to `ComponentHooks` for builder-style chaining.
    ///
    /// ```ignore
    /// world.register_hooks::<MyComponent>()
    ///     .on_add(|world, entity| { /* ... */ })
    ///     .on_remove(|world, entity| { /* ... */ });
    /// ```
    pub fn register_hooks<T: Component>(&mut self) -> &mut ComponentHooks {
        self.hooks
            .entry(TypeId::of::<T>())
            .or_insert(ComponentHooks {
                on_add: None,
                on_remove: None,
            })
    }

    /// Register a one-to-many relationship. Wires up `on_add` and `on_remove`
    /// hooks on `R` so that the inverse [`RelationshipTarget`] component is
    /// maintained automatically on the target entity.
    pub fn register_relationship<R: Relationship>(&mut self) {
        self.register_hooks::<R>()
            .on_add(relationship_on_add::<R>)
            .on_remove(relationship_on_remove::<R>);
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();

        if !self.entities.contains_key(&entity) {
            return;
        }

        // If replacing an existing component, fire on_remove for the old value
        let replacing = self.entities[&entity].contains_key(&type_id);
        if replacing {
            let hook = self.hooks.get(&type_id).and_then(|h| h.on_remove);
            if let Some(hook) = hook {
                hook(self, entity);
            }
        }

        // Insert the new component
        if let Some(components) = self.entities.get_mut(&entity) {
            components.insert(
                type_id,
                ComponentEntry {
                    data: Box::new(RefCell::new(component)),
                    changed_tick: Cell::new(self.current_tick),
                },
            );
        }

        // Fire on_add for the new component
        let hook = self.hooks.get(&type_id).and_then(|h| h.on_add);
        if let Some(hook) = hook {
            hook(self, entity);
        }
    }

    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        if let Some(components) = self.entities.get(&entity) {
            if let Some(entry) = components.get(&TypeId::of::<T>()) {
                let downcasted = Ref::map(entry.data.borrow(), |c| {
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

    pub fn get_component_mut<T: Component>(&self, entity: Entity) -> Option<Mut<'_, T>> {
        if let Some(components) = self.entities.get(&entity) {
            if let Some(entry) = components.get(&TypeId::of::<T>()) {
                let downcasted = RefMut::map(entry.data.borrow_mut(), |c| {
                    let as_any = c as &mut dyn Any;
                    as_any.downcast_mut::<T>().unwrap()
                });
                Some(Mut {
                    value: downcasted,
                    changed_tick: &entry.changed_tick,
                    world_tick: self.current_tick,
                })
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

    /// Remove a single component from an entity. Fires the `on_remove` hook
    /// (if registered) while the component is still readable.
    pub fn remove_component<T: Component>(&mut self, entity: Entity) {
        let type_id = TypeId::of::<T>();

        let has = self
            .entities
            .get(&entity)
            .is_some_and(|c| c.contains_key(&type_id));

        if !has {
            return;
        }

        // Fire on_remove while the component is still present
        let hook = self.hooks.get(&type_id).and_then(|h| h.on_remove);
        if let Some(hook) = hook {
            hook(self, entity);
        }

        if let Some(components) = self.entities.get_mut(&entity) {
            components.remove(&type_id);
        }
    }

    /// Remove an entity and all its components. Fires `on_remove` hooks for
    /// each component while the entity is still intact.
    pub fn remove_entity(&mut self, entity: Entity) {
        // Collect type IDs so we can fire hooks before removal
        let type_ids: Vec<TypeId> = match self.entities.get(&entity) {
            Some(c) => c.keys().copied().collect(),
            None => return,
        };

        for type_id in type_ids {
            let still_exists = self
                .entities
                .get(&entity)
                .is_some_and(|c| c.contains_key(&type_id));

            if still_exists {
                let hook = self.hooks.get(&type_id).and_then(|h| h.on_remove);
                if let Some(hook) = hook {
                    hook(self, entity);
                }
            }
        }

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

    // ── Change detection ────────────────────────────────────────────────

    /// Returns the current world tick.
    pub fn tick(&self) -> u32 {
        self.current_tick
    }

    /// Advance the world tick by one. Call once per frame so that
    /// modifications in the new frame get a fresh tick value.
    pub fn increment_tick(&mut self) {
        self.current_tick += 1;
    }

    /// Returns `true` if the component was added or mutably accessed at a
    /// tick strictly greater than `since`.
    pub fn is_changed<T: Component>(&self, entity: Entity, since: u32) -> bool {
        self.entities
            .get(&entity)
            .and_then(|c| c.get(&TypeId::of::<T>()))
            .is_some_and(|entry| entry.changed_tick.get() > since)
    }
}

pub type System = fn(&mut World);

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

    pub fn add_system(&mut self, system: System) {
        self.systems.push(system);
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

// ── Component Hooks ─────────────────────────────────────────────────────────

/// A hook that fires in response to a component lifecycle event.
///
/// Receives `&mut World` and the target `Entity`. The component is still
/// readable on the entity when the hook fires (for `on_remove`, it has not
/// been removed yet; for `on_add`, it has already been inserted).
pub type ComponentHook = fn(&mut World, Entity);

/// Per-component lifecycle hooks. At most one hook per event per component type.
pub struct ComponentHooks {
    on_add: Option<ComponentHook>,
    on_remove: Option<ComponentHook>,
}

impl ComponentHooks {
    /// Register a hook that fires after a component is added to an entity.
    ///
    /// Also fires when a component is replaced (after the old value's
    /// `on_remove` has run and the new value has been inserted).
    pub fn on_add(&mut self, hook: ComponentHook) -> &mut Self {
        self.on_add = Some(hook);
        self
    }

    /// Register a hook that fires before a component is removed from an entity.
    ///
    /// Fires during `remove_entity` (for each component), `remove_component`,
    /// and when a component is replaced via `add_component` (before the old
    /// value is overwritten).
    pub fn on_remove(&mut self, hook: ComponentHook) -> &mut Self {
        self.on_remove = Some(hook);
        self
    }
}

// ── Relationships ───────────────────────────────────────────────────────────

/// A component that references a single target entity, forming one half of a
/// one-to-many relationship. The inverse is maintained automatically via
/// [`RelationshipTarget`].
///
/// Example: `ChildOf(Entity)` — "this entity is a child of that entity".
pub trait Relationship: Component {
    /// The inverse component that collects all sources pointing at a target.
    type Target: RelationshipTarget;

    /// The entity this relationship points to.
    fn target(&self) -> Entity;
}

/// The inverse side of a [`Relationship`]. Automatically maintained — do not
/// modify directly. Holds the list of entities whose `Relationship` component
/// points at this entity.
///
/// Example: `Children(Vec<Entity>)` — "these entities are children of me".
pub trait RelationshipTarget: Component + Default {
    fn entities(&self) -> &[Entity];
    fn add(&mut self, entity: Entity);
    fn remove(&mut self, entity: Entity);
}

fn relationship_on_add<R: Relationship>(world: &mut World, entity: Entity) {
    let target = world.get_component::<R>(entity).unwrap().target();

    if let Some(mut target) = world.get_component_mut::<R::Target>(target) {
        target.add(entity);
    } else {
        let mut new_target = R::Target::default();
        new_target.add(entity);
        world.add_component(target, new_target);
    }
}

fn relationship_on_remove<R: Relationship>(world: &mut World, entity: Entity) {
    let target = world.get_component::<R>(entity).unwrap().target();

    let should_remove_target =
        if let Some(mut rel_target) = world.get_component_mut::<R::Target>(target) {
            rel_target.remove(entity);
            rel_target.entities().is_empty()
        } else {
            false
        };

    if should_remove_target {
        world.remove_component::<R::Target>(target);
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

        fn increment_component(world: &mut World) {
            for entity in world.get_entities_with::<TestComponent>() {
                if let Some(mut component) = world.get_component_mut::<TestComponent>(entity) {
                    component.0 += 1;
                }
            }
        }

        fn increment_resource(world: &mut World) {
            if let Some(mut resource) = world.get_resource_mut::<TestResource>() {
                resource.0 += 1;
            }
        }

        let mut schedule = Schedule::new();
        schedule.add_system(increment_component);
        schedule.add_system(increment_resource);

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

    // ── Hook tests ────────────────────────────────────────────────────

    struct HookTracker(Vec<&'static str>);
    impl Resource for HookTracker {}

    #[test]
    fn test_on_add_fires() {
        let mut world = World::new();
        world.insert_resource(HookTracker(Vec::new()));
        world
            .register_hooks::<TestComponent>()
            .on_add(|world, _entity| {
                world
                    .get_resource_mut::<HookTracker>()
                    .unwrap()
                    .0
                    .push("add");
            });

        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));

        let tracker = world.get_resource::<HookTracker>().unwrap();
        assert_eq!(tracker.0, vec!["add"]);
    }

    #[test]
    fn test_on_remove_fires_on_entity_removal() {
        let mut world = World::new();
        world.insert_resource(HookTracker(Vec::new()));
        world
            .register_hooks::<TestComponent>()
            .on_remove(|world, _entity| {
                world
                    .get_resource_mut::<HookTracker>()
                    .unwrap()
                    .0
                    .push("remove");
            });

        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));
        world.remove_entity(entity);

        let tracker = world.get_resource::<HookTracker>().unwrap();
        assert_eq!(tracker.0, vec!["remove"]);
    }

    #[test]
    fn test_on_remove_fires_on_component_removal() {
        let mut world = World::new();
        world.insert_resource(HookTracker(Vec::new()));
        world
            .register_hooks::<TestComponent>()
            .on_remove(|world, _entity| {
                world
                    .get_resource_mut::<HookTracker>()
                    .unwrap()
                    .0
                    .push("remove");
            });

        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));
        world.remove_component::<TestComponent>(entity);

        let tracker = world.get_resource::<HookTracker>().unwrap();
        assert_eq!(tracker.0, vec!["remove"]);
        assert!(!world.has_component::<TestComponent>(entity));
    }

    #[test]
    fn test_replacement_fires_remove_then_add() {
        let mut world = World::new();
        world.insert_resource(HookTracker(Vec::new()));
        world
            .register_hooks::<TestComponent>()
            .on_add(|world, _entity| {
                world
                    .get_resource_mut::<HookTracker>()
                    .unwrap()
                    .0
                    .push("add");
            })
            .on_remove(|world, _entity| {
                world
                    .get_resource_mut::<HookTracker>()
                    .unwrap()
                    .0
                    .push("remove");
            });

        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1)); // add
        world.add_component(entity, TestComponent(2)); // remove old, add new

        let tracker = world.get_resource::<HookTracker>().unwrap();
        assert_eq!(tracker.0, vec!["add", "remove", "add"]);

        let val = world.get_component::<TestComponent>(entity).unwrap();
        assert_eq!(val.0, 2);
    }

    #[test]
    fn test_on_remove_can_read_component() {
        let mut world = World::new();
        world.insert_resource(HookTracker(Vec::new()));
        world
            .register_hooks::<TestComponent>()
            .on_remove(|world, entity| {
                // The component should still be readable during on_remove
                let val = world.get_component::<TestComponent>(entity).unwrap();
                if val.0 == 42 {
                    world
                        .get_resource_mut::<HookTracker>()
                        .unwrap()
                        .0
                        .push("saw 42");
                }
            });

        let entity = world.create_entity();
        world.add_component(entity, TestComponent(42));
        world.remove_entity(entity);

        let tracker = world.get_resource::<HookTracker>().unwrap();
        assert_eq!(tracker.0, vec!["saw 42"]);
    }

    // ── Relationship tests ─────────────────────────────────────────────

    struct ChildOf(Entity);
    impl Component for ChildOf {}
    impl Relationship for ChildOf {
        type Target = Children;
        fn target(&self) -> Entity {
            self.0
        }
    }

    #[derive(Default)]
    struct Children(Vec<Entity>);
    impl Component for Children {}
    impl RelationshipTarget for Children {
        fn entities(&self) -> &[Entity] {
            &self.0
        }
        fn add(&mut self, entity: Entity) {
            self.0.push(entity);
        }
        fn remove(&mut self, entity: Entity) {
            self.0.retain(|&e| e != entity);
        }
    }

    #[test]
    fn test_relationship_on_add_creates_target() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent = world.create_entity();
        let child = world.create_entity();
        world.add_component(child, ChildOf(parent));

        let children = world.get_component::<Children>(parent).unwrap();
        assert_eq!(children.0, vec![child]);
    }

    #[test]
    fn test_relationship_multiple_children() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent = world.create_entity();
        let c1 = world.create_entity();
        let c2 = world.create_entity();
        let c3 = world.create_entity();
        world.add_component(c1, ChildOf(parent));
        world.add_component(c2, ChildOf(parent));
        world.add_component(c3, ChildOf(parent));

        let children = world.get_component::<Children>(parent).unwrap();
        assert_eq!(children.0, vec![c1, c2, c3]);
    }

    #[test]
    fn test_relationship_remove_child_entity() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent = world.create_entity();
        let c1 = world.create_entity();
        let c2 = world.create_entity();
        world.add_component(c1, ChildOf(parent));
        world.add_component(c2, ChildOf(parent));

        world.remove_entity(c1);

        let children = world.get_component::<Children>(parent).unwrap();
        assert_eq!(children.0, vec![c2]);
    }

    #[test]
    fn test_relationship_remove_last_child_removes_target() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent = world.create_entity();
        let child = world.create_entity();
        world.add_component(child, ChildOf(parent));
        world.remove_entity(child);

        assert!(!world.has_component::<Children>(parent));
    }

    #[test]
    fn test_relationship_reparent() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent_a = world.create_entity();
        let parent_b = world.create_entity();
        let child = world.create_entity();

        world.add_component(child, ChildOf(parent_a));
        assert_eq!(
            world.get_component::<Children>(parent_a).unwrap().0,
            vec![child]
        );

        // Re-parent: replacement fires on_remove(old) then on_add(new)
        world.add_component(child, ChildOf(parent_b));

        assert!(!world.has_component::<Children>(parent_a));
        assert_eq!(
            world.get_component::<Children>(parent_b).unwrap().0,
            vec![child]
        );
    }

    #[test]
    fn test_relationship_remove_component() {
        let mut world = World::new();
        world.register_relationship::<ChildOf>();

        let parent = world.create_entity();
        let child = world.create_entity();
        world.add_component(child, ChildOf(parent));
        world.remove_component::<ChildOf>(child);

        assert!(!world.has_component::<Children>(parent));
        assert!(!world.has_component::<ChildOf>(child));
    }

    // ── Change detection tests ──────────────────────────────────────────

    #[test]
    fn test_newly_added_component_is_changed() {
        let mut world = World::new();
        let since = world.tick();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));

        // Added at tick 0, since tick 0 → 0 > 0 → false (same tick)
        assert!(!world.is_changed::<TestComponent>(entity, since));

        // After incrementing, add at tick 1 is > since tick 0
        world.increment_tick();
        let entity2 = world.create_entity();
        world.add_component(entity2, TestComponent(2));
        assert!(world.is_changed::<TestComponent>(entity2, since));
    }

    #[test]
    fn test_mutation_marks_changed() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));

        let since = world.tick();
        world.increment_tick();

        // Mutably access the component (DerefMut stamps the tick)
        {
            let mut comp = world.get_component_mut::<TestComponent>(entity).unwrap();
            comp.0 = 42;
        }

        assert!(world.is_changed::<TestComponent>(entity, since));
    }

    #[test]
    fn test_read_only_access_does_not_mark_changed() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));

        world.increment_tick();
        let since = world.tick();

        // Read-only access should not mark as changed
        {
            let comp = world.get_component::<TestComponent>(entity).unwrap();
            assert_eq!(comp.0, 1);
        }

        assert!(!world.is_changed::<TestComponent>(entity, since));
    }

    #[test]
    fn test_unchanged_component_is_not_changed() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(1));

        world.increment_tick();
        let since = world.tick();
        world.increment_tick();

        // No access at all
        assert!(!world.is_changed::<TestComponent>(entity, since));
    }

    #[test]
    fn test_change_detection_across_frames() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(0));

        // Frame 1: no mutation
        world.increment_tick();
        let frame1_tick = world.tick();

        // Frame 2: mutate
        world.increment_tick();
        {
            let mut comp = world.get_component_mut::<TestComponent>(entity).unwrap();
            comp.0 = 10;
        }

        // Changed since frame 1
        assert!(world.is_changed::<TestComponent>(entity, frame1_tick));

        let frame2_tick = world.tick();

        // Frame 3: no mutation
        world.increment_tick();
        assert!(!world.is_changed::<TestComponent>(entity, frame2_tick));
    }

    #[test]
    fn test_deref_without_deref_mut_does_not_mark_changed() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, TestComponent(5));

        world.increment_tick();
        let since = world.tick();
        world.increment_tick();

        // get_component_mut but only read through Deref (not DerefMut)
        {
            let comp = world.get_component_mut::<TestComponent>(entity).unwrap();
            let _val = comp.0; // Deref, not DerefMut
        }

        assert!(!world.is_changed::<TestComponent>(entity, since));
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
