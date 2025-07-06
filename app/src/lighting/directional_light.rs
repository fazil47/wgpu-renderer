use ecs::Component;
use maths::Vec3;

/// Directional light component
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub azimuth: f32,
    pub altitude: f32,
}

impl DirectionalLight {
    pub fn new(azimuth: f32, altitude: f32) -> Self {
        let mut light = Self {
            direction: Vec3::ZERO,
            azimuth,
            altitude,
        };
        light.recalculate();
        light
    }

    pub fn recalculate(&mut self) {
        let azi_rad = self.azimuth.to_radians();
        let alt_rad = self.altitude.to_radians();

        self.direction = Vec3::new(
            azi_rad.sin() * alt_rad.cos(),
            alt_rad.sin(),
            azi_rad.cos() * alt_rad.cos(),
        )
        .normalized();
    }
}

impl Component for DirectionalLight {}
