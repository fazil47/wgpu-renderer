pub struct Time {
    pub delta_time: f32,
    pub elapsed_time: f32,
}

impl ecs::Resource for Time {}
