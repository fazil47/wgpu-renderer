use ecs::World;

pub fn clear_events_system(world: &mut World) {
    world.clear_all_events();
}
