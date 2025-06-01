use winit::dpi::PhysicalSize;

use crate::{
    camera::Camera,
    lights::DirectionalLight,
    mesh::{Material, static_mesh::StaticMeshExt},
};

pub struct Scene {
    pub materials: Vec<Material>,
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
            -camera_position.normalized(), // have the camera look at the origin
            window_size.width as f32 / window_size.height as f32,
            45.0,
            0.1,
            100.0,
        );
        let materials = Material::cornell_box();

        Self {
            materials,
            sun_light: DirectionalLight::new(45.0, 45.0),
            camera,
            is_light_dirty: false,
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.camera.set_aspect(aspect);
    }

    pub fn run_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.collapsing("Lighting", |ui| {
            let sun_azi_changed = ui
                .add(
                    egui::Slider::new(&mut self.sun_light.azimuth, 0.0..=360.0).text("Sun Azimuth"),
                )
                .changed();
            let sun_alt_changed = ui
                .add(
                    egui::Slider::new(&mut self.sun_light.altitude, 0.0..=90.0)
                        .text("Sun Altitude"),
                )
                .changed();
            if sun_azi_changed || sun_alt_changed {
                self.sun_light.recalculate();
                self.is_light_dirty = true;
                changed = true;
            }
        });

        changed
    }

    pub fn is_light_dirty(&self) -> bool {
        self.is_light_dirty
    }

    pub fn set_light_clean(&mut self) {
        self.is_light_dirty = false;
    }
}
