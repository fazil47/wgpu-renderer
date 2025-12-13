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

    /// Gets the light's view matrix
    pub fn get_light_view_matrix(&self, scene_center: Vec3, scene_radius: f32) -> maths::Mat4 {
        // Position the light camera behind the scene
        let light_position = scene_center - (self.direction.normalized() * scene_radius);

        // Build basis vectors
        let forward = self.direction.normalized();

        // Handle edge case where forward is parallel to world up
        let world_up = if forward.y.abs() > 0.99 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::Y
        };

        let right = forward.cross(world_up).normalized();
        let up = right.cross(forward).normalized();

        // Construct view matrix, which is the inverse of the transformation matrix
        // since view matrix undoes the camera transformation. Inverse is the transpose
        // for orthonormal basis vectors
        maths::Mat4::from_cols(
            maths::Vec4::new(right.x, up.x, -forward.x, 0.0),
            maths::Vec4::new(right.y, up.y, -forward.y, 0.0),
            maths::Vec4::new(right.z, up.z, -forward.z, 0.0),
            maths::Vec4::new(
                -right.dot(light_position),
                -up.dot(light_position),
                forward.dot(light_position),
                1.0,
            ),
        )
    }

    /// Gets the light's orthographic projection matrix
    pub fn get_light_projection_matrix(&self, scene_radius: f32) -> maths::Mat4 {
        let size = 2.0 * scene_radius;
        let near = 0.1;
        let far = scene_radius * 2.0;
        let depth = far - near;

        // Orthographic projection for WebGPU (depth range [0, 1])
        maths::Mat4::from_cols(
            maths::Vec4::new(2.0 / size, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, 2.0 / size, 0.0, 0.0),
            maths::Vec4::new(0.0, 0.0, -1.0 / depth, 0.0),
            maths::Vec4::new(0.0, 0.0, -near / depth, 1.0),
        )
    }

    /// Gets the light's view-projection matrix
    pub fn get_light_matrix(&self, scene_center: Vec3, scene_radius: f32) -> maths::Mat4 {
        let view = self.get_light_view_matrix(scene_center, scene_radius);
        let projection = self.get_light_projection_matrix(scene_radius);
        projection * view
    }
}

impl Component for DirectionalLight {}
