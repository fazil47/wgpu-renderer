use ecs::Component;
use maths::Vec3;

// TODO: The transform is encoded in the Camera component. Should this be simplified by using a Transform component instead of the eye, forward and up fields?

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

        /*
         * View matrix intuition:
         *   1. Rotate world points into the camera basis [right, up, -forward].
         *   2. Translate so the eye ends up at the origin of camera space.
         *
         *   Camera-aligned basis (ASCII):
         *       world_up (Y)
         *          ^
         *          |   forward
         *          |  /
         *          | /
         *          o ----> right
         *         eye
         *
         *   After rotation, the dot products with right/up/-forward give the coordinates of any point
         *   relative to the eye. Subtracting those dot products in the last column shifts the scene so
         *   the camera sits at (0,0,0) looking down negative Z.
         */
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

        /*
         * Perspective matrix intuition:
         *   1) self.fov is the vertical field-of-view in degrees. The half-angle theta = fov/2 forms a right
         *      triangle with the near plane, so the near-plane half-height is
         *          top = near * tan(theta).
         *      Multiply by aspect to obtain the half-width
         *          right_proj = top * aspect.
         *   2) The first two columns scale x/y by near/right_proj and near/top so this near-plane rectangle
         *      lands inside clip-space [-1, 1].
         *   3) The rasterizer expects the homogeneous w after transformation to be -z (right-handed convention),
         *      so the fourth column is chosen to produce w_c = -z for any input point. The -1 sitting in the
         *      third-row/fourth-column slot is what multiplies the incoming w=1 to give w_c = -z. After the GPU
         *      divides by w_c, depth becomes z_ndc = z_c / w_c.
         *   4) We require z_ndc = 1 at z = -near and z_ndc = 0 at z = -far (we're using reverse depth buffer) so the clip depth range matches the API.
         *      Let z_c = A*z + B. Solving the two equations you get using the z_ndc formula.
         *          (A * -near + B) / near = 1
         *          (A * -far  + B) / far  = 0
         *      yields
         *          A = near / (far - near)
         *          B = far * near / (far - near).
         *      Placing A in the third-row, third-column slot and B in the third-row, fourth-column slot causes the
         *      perspective divide (division by -z) to generate the expected right-handed depth curve.
         */
        let camera_projection = maths::Mat4::from_cols(
            maths::Vec4::new(self.near / right_proj, 0.0, 0.0, 0.0),
            maths::Vec4::new(0.0, self.near / top, 0.0, 0.0),
            maths::Vec4::new(0.0, 0.0, self.near / (self.far - self.near), -1.0),
            maths::Vec4::new(
                0.0,
                0.0,
                (self.far * self.near) / (self.far - self.near),
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
