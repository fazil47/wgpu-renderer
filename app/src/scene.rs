use crate::lights;

pub struct Scene {
    pub sun_light: lights::DirectionalLight,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            sun_light: lights::DirectionalLight::new(45.0, 45.0),
        }
    }
}
