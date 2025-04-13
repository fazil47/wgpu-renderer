use maths::Vec3;

pub struct DirectionalLight {
    pub azimuth: f32,
    pub altitude: f32,
    pub direction: Vec3,
}

impl DirectionalLight {
    pub fn new(azimuth: f32, altitude: f32) -> Self {
        Self {
            azimuth,
            altitude,
            direction: Self::calculate_direction(azimuth, altitude),
        }
    }

    pub fn recalculate(&mut self) {
        self.direction = Self::calculate_direction(self.azimuth, self.altitude);
    }

    fn calculate_direction(azimuth: f32, altitude: f32) -> Vec3 {
        let azi = azimuth.to_radians();
        let alt = altitude.to_radians();
        let x = azi.sin() * alt.cos();
        let y = azi.cos() * alt.cos();
        let z = alt.sin();

        Vec3::new(x, y, z)
    }
}
