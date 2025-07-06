use ecs::Component;

/// Tag component to mark entities as renderable
#[derive(Debug, Clone)]
pub struct Renderable;

impl Component for Renderable {}
