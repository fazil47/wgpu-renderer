use glam::Vec3A;

pub struct DirectionalLight {
    pub direction: Vec3A,
}

impl DirectionalLight {
    pub fn new(direction: Vec3A) -> Self {
        Self { direction }
    }

    pub fn from_azi_alt(azi: f32, alt: f32) -> Self {
        let azi = azi.to_radians();
        let alt = alt.to_radians();
        let x = azi.sin() * alt.cos();
        let y = azi.cos() * alt.cos();
        let z = alt.sin();

        Self {
            direction: Vec3A::new(x, y, z),
        }
    }
}
