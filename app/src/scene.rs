use crate::{
    lights,
    mesh::{Mesh, StaticMesh},
};

pub struct Scene {
    pub mesh: Box<dyn Mesh>,
    pub sun_light: lights::DirectionalLight,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            mesh: Box::new(StaticMesh::cornell_box()),
            sun_light: lights::DirectionalLight::new(45.0, 45.0),
        }
    }
}
