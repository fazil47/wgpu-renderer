use ecs::{Component, World};
use maths::{Quat, Vec3};

use crate::rendering::TlasBvh;

/// Directional light component
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3, // from light to scene
    pub azimuth: f32,
    pub altitude: f32,
    pub near: f32,
}

impl DirectionalLight {
    pub fn new(azimuth: f32, altitude: f32) -> Self {
        let mut light = Self {
            direction: Vec3::ZERO,
            azimuth,
            altitude,
            near: 0.001,
        };
        light.recalculate();
        light
    }

    pub fn recalculate(&mut self) {
        let azi_rad = self.azimuth.to_radians();
        let alt_rad = self.altitude.to_radians();

        self.direction = -Vec3::new(
            azi_rad.sin() * alt_rad.cos(),
            alt_rad.sin(),
            azi_rad.cos() * alt_rad.cos(),
        )
        .normalized();
    }

    /// Gets the light's view matrix
    pub fn get_light_view_matrix(&self, scene_center: Vec3, scene_radius: f32) -> maths::Mat4 {
        // Position the light camera away from the scene at the direction of the light
        let light_position = scene_center - (self.direction.normalized() * scene_radius);

        // Use a quaternion to find the basis vectors
        let from = Vec3::FORWARD;
        let to = self.direction.normalized();
        let quat = Quat::from_rotation_arc(from, to);

        // Build basis vectors
        let forward = quat * Vec3::FORWARD;
        let right = quat * Vec3::RIGHT;
        let up = quat * Vec3::UP;

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
        let far = scene_radius * 2.0;
        let depth = far - self.near;

        // Orthographic projection for WebGPU (depth range [0, 1])
        maths::Mat4::from_cols(
            maths::Vec4::new(2.0 / size, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, 2.0 / size, 0.0, 0.0),
            maths::Vec4::new(0.0, 0.0, -1.0 / depth, 0.0),
            maths::Vec4::new(0.0, 0.0, -self.near / depth, 1.0),
        )
    }

    /// Gets the light's view-projection matrix
    pub fn get_light_matrix(&self, world: &World) -> maths::Mat4 {
        let tlas = world.get_resource::<TlasBvh>().unwrap();
        let [x_max, y_max, z_max, _] = tlas.bvh.nodes[0].bounds_max;
        let [x_min, y_min, z_min, _] = tlas.bvh.nodes[0].bounds_min;
        let scene_max = Vec3::new(x_max, y_max, z_max);
        let scene_min = Vec3::new(x_min, y_min, z_min);
        let scene_center = (scene_max + scene_min) / 2.0; // min + (max - min) / 2.0
        let scene_radius = (scene_max - scene_min).length() / 2.0;

        let view = self.get_light_view_matrix(scene_center, scene_radius);
        let projection = self.get_light_projection_matrix(scene_radius);
        projection * view
    }
}

impl Component for DirectionalLight {}
