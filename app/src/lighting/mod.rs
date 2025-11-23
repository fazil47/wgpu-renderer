pub mod directional_light;
pub mod probe_lighting;

pub use directional_light::DirectionalLight;

#[derive(Default)]
pub struct LightDirtyFlag(pub bool);
impl ecs::Resource for LightDirtyFlag {}

pub use probe_lighting::*;
