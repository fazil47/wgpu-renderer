pub struct Camera {
    eye: glam::Vec3,
    target: glam::Vec3,
    up: glam::Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    world_to_camera: glam::Mat4,
    camera_to_world: glam::Mat4,
    camera_projection: glam::Mat4,
    camera_inverse_projection: glam::Mat4,
    view_projection: glam::Mat4,
}

impl Camera {
    pub fn new(
        eye: glam::Vec3,
        target: glam::Vec3,
        up: glam::Vec3,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        let (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        ) = Self::calculate_matrices(eye, target, up, aspect, fovy, znear, zfar);

        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_matrices();
    }

    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.update_matrices();
    }

    pub fn set_znear(&mut self, znear: f32) {
        self.znear = znear;
        self.update_matrices();
    }

    pub fn set_zfar(&mut self, zfar: f32) {
        self.zfar = zfar;
        self.update_matrices();
    }

    pub fn set_eye(&mut self, eye: glam::Vec3) {
        self.eye = eye;
        self.update_matrices();
    }

    pub fn set_target(&mut self, target: glam::Vec3) {
        self.target = target;
        self.update_matrices();
    }

    pub fn set_up(&mut self, up: glam::Vec3) {
        self.up = up;
        self.update_matrices();
    }

    pub fn eye(&self) -> glam::Vec3 {
        self.eye
    }

    pub fn target(&self) -> glam::Vec3 {
        self.target
    }

    pub fn up(&self) -> glam::Vec3 {
        self.up
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn fovy(&self) -> f32 {
        self.fovy
    }

    pub fn znear(&self) -> f32 {
        self.znear
    }

    pub fn zfar(&self) -> f32 {
        self.zfar
    }

    pub fn world_to_camera(&self) -> glam::Mat4 {
        self.world_to_camera
    }

    pub fn camera_to_world(&self) -> glam::Mat4 {
        self.camera_to_world
    }

    pub fn camera_projection(&self) -> glam::Mat4 {
        self.camera_projection
    }

    pub fn camera_inverse_projection(&self) -> glam::Mat4 {
        self.camera_inverse_projection
    }

    /// Returns the view projection matrix of the camera.
    /// The view projection matrix is the product of the camera projection matrix and the world to camera matrix,
    /// and it's used to transform the vertices of the objects from world space to clip space.
    ///
    /// # Returns
    ///
    /// * `glam::Mat4` - The view projection matrix of the camera.
    /// ```
    pub fn view_projection(&self) -> glam::Mat4 {
        self.view_projection
    }

    fn update_matrices(&mut self) {
        let (
            world_to_camera,
            camera_to_world,
            camera_projection,
            camera_inverse_projection,
            view_projection,
        ) = Self::calculate_matrices(
            self.eye,
            self.target,
            self.up,
            self.aspect,
            self.fovy,
            self.znear,
            self.zfar,
        );

        self.world_to_camera = world_to_camera;
        self.camera_to_world = camera_to_world;
        self.camera_projection = camera_projection;
        self.camera_inverse_projection = camera_inverse_projection;
        self.view_projection = view_projection;
    }

    fn calculate_matrices(
        eye: glam::Vec3,
        target: glam::Vec3,
        up: glam::Vec3,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> (glam::Mat4, glam::Mat4, glam::Mat4, glam::Mat4, glam::Mat4) {
        let world_to_camera = glam::Mat4::look_at_rh(eye, target, up);
        let camera_to_world = world_to_camera.inverse();
        let camera_projection = glam::Mat4::perspective_rh(fovy, aspect, znear, zfar);
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
