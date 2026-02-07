use ecs::{Component, World};
use maths::{Mat4, Quat, Vec3, Vec4};

use crate::{camera::Camera, rendering::TlasBvh};

pub const CASCADED_SHADOW_FRUSTUM_SPLITS: [f32; 3] = [10.0, 50.0, 200.0]; // Frustum bounds are camera near plane, 10, 50, 200 and camera far plane. These are the z distances in camera space.
pub const CASCADED_SHADOW_NUM_CASCADES: usize = CASCADED_SHADOW_FRUSTUM_SPLITS.len() + 1;

/*
TODO: CASCADED SHADOW MAPS

- Divide view frustum into 4 cascades
  - [x] get world space positions of the corners of the 4 cascade frustums
  - [x] rotate using the light's rotation matrix to align with the light direction
  - [x] Find the AABB of the corners
  - [x] Calculate the view matrices by using the center of the AABBs offset by half their extent along z and the rotation matrix
  - [x] Calculate the projection matrices by using the half extent of the AABBs along x and y and the extent along z
- [x] Generate a shadow map for each cascade
  - [x] Store the cascade transforms in a single buffer
  - [x] Render each cascade using separate render passes
- [x] Modify the fragment shader to sample from the cascade a mesh is in
- [] Artifact mitigations
 - Shadow casters are being clipped because the bounding boxes calculated for each cascade is too tight.
  - [x] Switch to a reverse z depth buffer to improve precision for far away meshes, this is important for the next step
  - [] For each of the bounding boxes (that are rotated to align with the directional light), instead of them tightly surrounding the scene camera frustum cascade, extend the near face of the boxes back by a huge amount (enough to capture the whole scene, use scene_radius).
*/

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

    /// Gets the light's rotation matrix
    pub fn get_rotation_matrix(&self) -> maths::Mat4 {
        let from = Vec3::FORWARD;
        let to = self.direction.normalized();
        let quat = Quat::from_rotation_arc(from, to);

        // Basis vectors
        let forward = to;
        let right = quat * Vec3::RIGHT;
        let up = quat * Vec3::UP;

        // The z-column is made using the inverse of forward since +z direction should be towards the camera (so the inverse of forward)
        // with +x to the right and +y to the top according to the right hand rule
        Mat4::from_cols(
            Vec4::new(right.x, right.y, right.z, 0.0),
            Vec4::new(up.x, up.y, up.z, 0.0),
            Vec4::new(-forward.x, -forward.y, -forward.z, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
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

        // Orthographic projection for WebGPU with reverse depth buffer (depth range [1, 0])
        maths::Mat4::from_cols(
            maths::Vec4::new(2.0 / size, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, 2.0 / size, 0.0, 0.0),
            maths::Vec4::new(0.0, 0.0, 1.0 / depth, 0.0),
            maths::Vec4::new(0.0, 0.0, far / depth, 1.0),
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

    pub fn get_cascaded_light_matrices(
        &self,
        world: &World,
    ) -> [maths::Mat4; CASCADED_SHADOW_NUM_CASCADES] {
        let mut matrices = [Mat4::IDENTITY; CASCADED_SHADOW_NUM_CASCADES];

        let camera_entities = world.get_entities_with::<Camera>();
        let camera_entity = camera_entities.first().unwrap();
        let camera = world.get_component::<Camera>(*camera_entity).unwrap();

        let rotation_matrix = self.get_rotation_matrix();
        let inverse_rotation_matrix = rotation_matrix.inverse();

        let cascades = [
            (camera.near, CASCADED_SHADOW_FRUSTUM_SPLITS[0]),
            (
                CASCADED_SHADOW_FRUSTUM_SPLITS[0],
                CASCADED_SHADOW_FRUSTUM_SPLITS[1],
            ),
            (
                CASCADED_SHADOW_FRUSTUM_SPLITS[1],
                CASCADED_SHADOW_FRUSTUM_SPLITS[2],
            ),
            (CASCADED_SHADOW_FRUSTUM_SPLITS[2], camera.far),
        ];

        for (i, (start, end)) in cascades.iter().enumerate() {
            // Transform frustum corners to light space to compute tight AABB
            let corners: [Vec4; 8] = Self::get_frustum_corners(&camera, *start, *end)
                .map(|p| inverse_rotation_matrix * p.extend(1.0));

            // Compute AABB in light space
            let (min, max) = {
                let (mut min, mut max) = (Vec4::MAX, Vec4::MIN);

                for corner in corners {
                    min = corner.min(min);
                    max = corner.max(max);
                }

                (min, max)
            };

            let center_light = (max + min) / 2.0;
            let half_extent_light = (max - min) / 2.0;
            let radius = half_extent_light.z;

            // Transform center back to world space
            let center_world = (rotation_matrix * center_light).xyz();

            // Translate from center_world then offset towards light direction by radius
            let translation = center_world - (self.direction.normalized() * radius);
            let translation_matrix = Mat4::from_cols(
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 1.0, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 1.0, 0.0),
                Vec4::new(translation.x, translation.y, translation.z, 1.0),
            );
            let local_to_world = translation_matrix * rotation_matrix;
            let view_matrix = local_to_world.inverse(); // view matrix takes a point from world space to the camera's local space

            let depth = 2.0 * half_extent_light.z;
            let projection_matrix = Mat4::from_cols(
                Vec4::new(1.0 / half_extent_light.x, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 1.0 / half_extent_light.y, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 1.0 / depth, 0.0),
                Vec4::new(0.0, 0.0, 1.0, 1.0),
            );

            matrices[i] = projection_matrix * view_matrix;
        }

        matrices
    }

    // Get the cascade frustum corners in world space
    fn get_frustum_corners(camera: &Camera, cascade_start: f32, cascade_end: f32) -> [Vec3; 8] {
        let fov = camera.fov.to_radians() / 2.0; // Use half the vertical angle
        let fov_tan = fov.tan();
        let aspect = camera.aspect;

        let nhh = cascade_start * fov_tan; // Near half height
        let nhw = aspect * nhh; // Near half width

        let fhh = cascade_end * fov_tan; // Far half height
        let fhw = aspect * fhh; // Far half width

        let ntl = Vec3::new(-nhw, nhh, cascade_start); // near top left
        let nbl = Vec3::new(-nhw, -nhh, cascade_start); // near bottom left
        let nbr = Vec3::new(nhw, -nhh, cascade_start); // near bottom right
        let ntr = Vec3::new(nhw, nhh, cascade_start); // near top right

        let ftl = Vec3::new(-fhw, fhh, cascade_end); // far top left
        let fbl = Vec3::new(-fhw, -fhh, cascade_end); // far bottom left
        let fbr = Vec3::new(fhw, -fhh, cascade_end); // far bottom right
        let ftr = Vec3::new(fhw, fhh, cascade_end); // far top right

        let forward = camera.forward;
        let up = camera.up;
        let right = forward.cross(up).normalized();

        [ntl, nbl, nbr, ntr, ftl, fbl, fbr, ftr]
            .map(|corner| camera.eye + forward * corner.z + right * corner.x + up * corner.y)
    }
}

impl Component for DirectionalLight {}
