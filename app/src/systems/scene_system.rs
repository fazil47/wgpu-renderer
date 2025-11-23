use ecs::World;

pub fn reset_dirty_flags_system(world: &mut World) {
    if let Some(mut flags) = world.get_resource_mut::<crate::core::flags::DirtyFlags>() {
        flags.static_data = false;
        flags.lights = false;
        flags.probe_bake_requested = false;
        flags.raytracer_reset = false;
    }
}
