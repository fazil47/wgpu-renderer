use ecs::Component;
use maths::Vec3;

/// Camera component for view and projection calculations
#[derive(Debug, Clone)]
pub struct Camera {
    pub eye: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(eye: Vec3, forward: Vec3, aspect: f32, fov: f32, near: f32, far: f32) -> Self {
        Self {
            eye,
            forward,
            up: Vec3::Y,
            fov,
            aspect,
            near,
            far,
        }
    }

    pub fn view_projection(&self) -> maths::Mat4 {
        let (_, _, _, _, view_projection) = self.calculate_matrices();
        view_projection
    }

    pub fn view_matrix(&self) -> maths::Mat4 {
        let (world_to_camera, _, _, _, _) = self.calculate_matrices();
        world_to_camera
    }

    pub fn projection_matrix(&self) -> maths::Mat4 {
        let (_, _, camera_projection, _, _) = self.calculate_matrices();
        camera_projection
    }

    pub fn camera_to_world(&self) -> maths::Mat4 {
        let (_, camera_to_world, _, _, _) = self.calculate_matrices();
        camera_to_world
    }

    pub fn camera_inverse_projection(&self) -> maths::Mat4 {
        let (_, _, _, camera_inverse_projection, _) = self.calculate_matrices();
        camera_inverse_projection
    }

    fn calculate_matrices(
        &self,
    ) -> (
        maths::Mat4,
        maths::Mat4,
        maths::Mat4,
        maths::Mat4,
        maths::Mat4,
    ) {
        let right = self.forward.cross(self.up).normalized();
        let up = right.cross(self.forward).normalized();

        let world_to_camera = maths::Mat4::from_cols(
            maths::Vec4::new(right.x, up.x, -self.forward.x, 0.0),
            maths::Vec4::new(right.y, up.y, -self.forward.y, 0.0),
            maths::Vec4::new(right.z, up.z, -self.forward.z, 0.0),
            maths::Vec4::new(
                -right.dot(self.eye),
                -up.dot(self.eye),
                self.forward.dot(self.eye),
                1.0,
            ),
        );
        let camera_to_world = world_to_camera.inverse();

        let top = self.near * (self.fov.to_radians() / 2.0).tan();
        let right_proj = top * self.aspect;

        let camera_projection = maths::Mat4::from_cols(
            maths::Vec4::new(self.near / right_proj, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, self.near / top, 0.0, 0.0),
            maths::Vec4::new(
                0.0,
                0.0,
                -(self.far + self.near) / (self.far - self.near),
                -1.0,
            ),
            maths::Vec4::new(
                0.0,
                0.0,
                -(2.0 * self.far * self.near) / (self.far - self.near),
                0.0,
            ),
        );
        let camera_inverse_projection = camera_projection.inverse();

        let view_projection = camera_projection * world_to_camera;

        (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        )
    }
}

impl Component for Camera {}
