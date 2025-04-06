use maths::Vec3;

pub struct DirectionalLight {
    pub direction: Vec3,
}

impl DirectionalLight {
    pub fn new(direction: Vec3) -> Self {
        Self { direction }
    }

    pub fn from_azi_alt(azi: f32, alt: f32) -> Self {
        let azi = azi.to_radians();
        let alt = alt.to_radians();
        let x = azi.sin() * alt.cos();
        let y = azi.cos() * alt.cos();
        let z = alt.sin();

        Self {
            direction: Vec3::new(x, y, z),
        }
    }
}
