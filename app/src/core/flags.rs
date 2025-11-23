#[derive(Default)]
pub struct DirtyFlags {
    pub static_data: bool,
    pub lights: bool,
    pub probe_bake_requested: bool,
    pub raytracer_reset: bool,
}

impl ecs::Resource for DirtyFlags {}
