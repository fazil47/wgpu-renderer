use crate::core::engine::StaticDataDirtyFlag;
use ecs::World;

pub fn reset_dirty_flags_system(world: &mut World) {
    // Reset static data dirty flag
    if let Some(mut flag) = world.get_resource_mut::<StaticDataDirtyFlag>() {
        flag.0 = false;
    }

    // Reset light dirty flag
    if let Some(mut flag) = world.get_resource_mut::<crate::core::engine::LightDirtyFlag>() {
        flag.0 = false;
    }

    // Reset raytracer flag
    if let Some(mut config) = world.get_resource_mut::<crate::core::engine::EngineConfiguration>() {
        config.reset_raytracer = false;
    }

    // Reset probe bake requested flag
    if let Some(mut ui_state) = world.get_resource_mut::<crate::ui::UiState>() {
        ui_state.bake_requested = false;
    }
}
