use winit::dpi::PhysicalSize;

use crate::{
    camera::Camera,
    lights::DirectionalLight,
    mesh::{Mesh, static_mesh::StaticMesh},
};

pub struct Scene {
    pub meshes: Vec<Box<dyn Mesh>>,
    pub sun_light: DirectionalLight,
    pub camera: Camera,
    is_light_dirty: bool,
}

impl Scene {
    pub fn new(window_size: &PhysicalSize<u32>) -> Self {
        // position the camera 4 units back
        // +z is out of the screen
        let camera_position: maths::Vec3 = (0.0, 0.0, 4.0).into();
        let camera = Camera::new(
            camera_position,
            -camera_position.normalize(), // have the camera look at the origin
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            100.0,
        );

        Self {
            meshes: vec![Box::new(StaticMesh::cornell_box())],
            sun_light: DirectionalLight::new(45.0, 45.0),
            camera,
            is_light_dirty: false,
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.camera.set_aspect(aspect);
    }

    pub fn run_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let sun_azi_changed = ui
            .add(egui::Slider::new(&mut self.sun_light.azimuth, 0.0..=360.0).text("Sun Azimuth"))
            .changed();

        let sun_alt_changed = ui
            .add(egui::Slider::new(&mut self.sun_light.altitude, 0.0..=90.0).text("Sun Altitude"))
            .changed();

        if sun_azi_changed || sun_alt_changed {
            self.sun_light.recalculate();
            self.is_light_dirty = true;
            return true;
        }

        false
    }

    pub fn is_light_dirty(&self) -> bool {
        self.is_light_dirty
    }

    pub fn set_light_clean(&mut self) {
        self.is_light_dirty = false;
    }
}
