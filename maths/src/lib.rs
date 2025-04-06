use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn to_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn normalize(&self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if length == 0.0 {
            return Self::ZERO;
        }

        Self::new(self.x / length, self.y / length, self.z / length)
    }

    pub const fn dot(&self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub const fn cross(&self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            -(self.x * rhs.z - self.z * rhs.x),
            self.x * rhs.y - self.y * rhs.x,
        )
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl From<(f32, f32, f32)> for Vec3 {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self::new(x, y, z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub x_axis: Vec4,
    pub y_axis: Vec4,
    pub z_axis: Vec4,
    pub w_axis: Vec4,
}

impl Mat4 {
    pub const IDENTITY: Self = Self::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

    pub const fn a1(&self) -> f32 {
        self.x_axis.x
    }

    pub const fn a2(&self) -> f32 {
        self.x_axis.y
    }

    pub const fn a3(&self) -> f32 {
        self.x_axis.z
    }

    pub const fn a4(&self) -> f32 {
        self.x_axis.w
    }

    pub const fn b1(&self) -> f32 {
        self.y_axis.x
    }

    pub const fn b2(&self) -> f32 {
        self.y_axis.y
    }

    pub const fn b3(&self) -> f32 {
        self.y_axis.z
    }

    pub const fn b4(&self) -> f32 {
        self.y_axis.w
    }

    pub const fn c1(&self) -> f32 {
        self.z_axis.x
    }

    pub const fn c2(&self) -> f32 {
        self.z_axis.y
    }

    pub const fn c3(&self) -> f32 {
        self.z_axis.z
    }

    pub const fn c4(&self) -> f32 {
        self.z_axis.w
    }

    pub const fn d1(&self) -> f32 {
        self.w_axis.x
    }

    pub const fn d2(&self) -> f32 {
        self.w_axis.y
    }

    pub const fn d3(&self) -> f32 {
        self.w_axis.z
    }

    pub const fn d4(&self) -> f32 {
        self.w_axis.w
    }

    pub const fn from_cols(x_axis: Vec4, y_axis: Vec4, z_axis: Vec4, w_axis: Vec4) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
            w_axis,
        }
    }

    pub const fn to_cols_array_2d(&self) -> [[f32; 4]; 4] {
        [
            self.x_axis.to_array(),
            self.y_axis.to_array(),
            self.z_axis.to_array(),
            self.w_axis.to_array(),
        ]
    }

    pub const fn determinant(&self) -> f32 {
        let (a1, a2, a3, a4) = (self.a1(), self.a2(), self.a3(), self.a4());
        let (b1, b2, b3, b4) = (self.b1(), self.b2(), self.b3(), self.b4());
        let (c1, c2, c3, c4) = (self.c1(), self.c2(), self.c3(), self.c4());
        let (d1, d2, d3, d4) = (self.d1(), self.d2(), self.d3(), self.d4());

        // Compute determinant using signed cofactors
        a1 * Self::det3(b2, b3, b4, c2, c3, c4, d2, d3, d4)
            - a2 * Self::det3(b1, b3, b4, c1, c3, c4, d1, d3, d4)
            + a3 * Self::det3(b1, b2, b4, c1, c2, c4, d1, d2, d4)
            - a4 * Self::det3(b1, b2, b3, c1, c2, c3, d1, d2, d3)
    }

    pub const fn cofactor(&self) -> Self {
        let (a1, a2, a3, a4) = (self.a1(), self.a2(), self.a3(), self.a4());
        let (b1, b2, b3, b4) = (self.b1(), self.b2(), self.b3(), self.b4());
        let (c1, c2, c3, c4) = (self.c1(), self.c2(), self.c3(), self.c4());
        let (d1, d2, d3, d4) = (self.d1(), self.d2(), self.d3(), self.d4());

        // Compute all cofactors
        let co11 = Self::det3(b2, b3, b4, c2, c3, c4, d2, d3, d4);
        let co12 = -Self::det3(b1, b3, b4, c1, c3, c4, d1, d3, d4);
        let co13 = Self::det3(b1, b2, b4, c1, c2, c4, d1, d2, d4);
        let co14 = -Self::det3(b1, b2, b3, c1, c2, c3, d1, d2, d3);

        let co21 = -Self::det3(a2, a3, a4, c2, c3, c4, d2, d3, d4);
        let co22 = Self::det3(a1, a3, a4, c1, c3, c4, d1, d3, d4);
        let co23 = -Self::det3(a1, a2, a4, c1, c2, c4, d1, d2, d4);
        let co24 = Self::det3(a1, a2, a3, c1, c2, c3, d1, d2, d3);

        let co31 = Self::det3(a2, a3, a4, b2, b3, b4, d2, d3, d4);
        let co32 = -Self::det3(a1, a3, a4, b1, b3, b4, d1, d3, d4);
        let co33 = Self::det3(a1, a2, a4, b1, b2, b4, d1, d2, d4);
        let co34 = -Self::det3(a1, a2, a3, b1, b2, b3, d1, d2, d3);

        let co41 = -Self::det3(a2, a3, a4, b2, b3, b4, c2, c3, c4);
        let co42 = Self::det3(a1, a3, a4, b1, b3, b4, c1, c3, c4);
        let co43 = -Self::det3(a1, a2, a4, b1, b2, b4, c1, c2, c4);
        let co44 = Self::det3(a1, a2, a3, b1, b2, b3, c1, c2, c3);

        // Return the cofactor matrix
        Self::from_cols(
            Vec4::new(co11, co12, co13, co14),
            Vec4::new(co21, co22, co23, co24),
            Vec4::new(co31, co32, co33, co34),
            Vec4::new(co41, co42, co43, co44),
        )
    }

    pub const fn transpose(&self) -> Self {
        Self::from_cols(
            Vec4::new(self.a1(), self.b1(), self.c1(), self.d1()),
            Vec4::new(self.a2(), self.b2(), self.c2(), self.d2()),
            Vec4::new(self.a3(), self.b3(), self.c3(), self.d3()),
            Vec4::new(self.a4(), self.b4(), self.c4(), self.d4()),
        )
    }

    pub const fn adjugate(&self) -> Self {
        self.cofactor().transpose()
    }

    pub fn inverse(&self) -> Self {
        let det = self.determinant();
        if det == 0.0 {
            return Self::IDENTITY;
        }

        self.adjugate() * (1.0 / det)
    }

    const fn det3(
        a1: f32,
        a2: f32,
        a3: f32,
        b1: f32,
        b2: f32,
        b3: f32,
        c1: f32,
        c2: f32,
        c3: f32,
    ) -> f32 {
        a1 * (b2 * c3 - b3 * c2) - a2 * (b1 * c3 - b3 * c1) + a3 * (b1 * c2 - b2 * c1)
    }
}

impl Mul<f32> for Mat4 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::from_cols(
            self.x_axis * rhs,
            self.y_axis * rhs,
            self.z_axis * rhs,
            self.w_axis * rhs,
        )
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let x_axis = Vec4::new(
            self.a1() * rhs.a1()
                + self.b1() * rhs.a2()
                + self.c1() * rhs.a3()
                + self.d1() * rhs.a4(),
            self.a2() * rhs.a1()
                + self.b2() * rhs.a2()
                + self.c2() * rhs.a3()
                + self.d2() * rhs.a4(),
            self.a3() * rhs.a1()
                + self.b3() * rhs.a2()
                + self.c3() * rhs.a3()
                + self.d3() * rhs.a4(),
            self.a4() * rhs.a1()
                + self.b4() * rhs.a2()
                + self.c4() * rhs.a3()
                + self.d4() * rhs.a4(),
        );

        let y_axis = Vec4::new(
            self.a1() * rhs.b1()
                + self.b1() * rhs.b2()
                + self.c1() * rhs.b3()
                + self.d1() * rhs.b4(),
            self.a2() * rhs.b1()
                + self.b2() * rhs.b2()
                + self.c2() * rhs.b3()
                + self.d2() * rhs.b4(),
            self.a3() * rhs.b1()
                + self.b3() * rhs.b2()
                + self.c3() * rhs.b3()
                + self.d3() * rhs.b4(),
            self.a4() * rhs.b1()
                + self.b4() * rhs.b2()
                + self.c4() * rhs.b3()
                + self.d4() * rhs.b4(),
        );

        let z_axis = Vec4::new(
            self.a1() * rhs.c1()
                + self.b1() * rhs.c2()
                + self.c1() * rhs.c3()
                + self.d1() * rhs.c4(),
            self.a2() * rhs.c1()
                + self.b2() * rhs.c2()
                + self.c2() * rhs.c3()
                + self.d2() * rhs.c4(),
            self.a3() * rhs.c1()
                + self.b3() * rhs.c2()
                + self.c3() * rhs.c3()
                + self.d3() * rhs.c4(),
            self.a4() * rhs.c1()
                + self.b4() * rhs.c2()
                + self.c4() * rhs.c3()
                + self.d4() * rhs.c4(),
        );

        let w_axis = Vec4::new(
            self.a1() * rhs.d1()
                + self.b1() * rhs.d2()
                + self.c1() * rhs.d3()
                + self.d1() * rhs.d4(),
            self.a2() * rhs.d1()
                + self.b2() * rhs.d2()
                + self.c2() * rhs.d3()
                + self.d2() * rhs.d4(),
            self.a3() * rhs.d1()
                + self.b3() * rhs.d2()
                + self.c3() * rhs.d3()
                + self.d3() * rhs.d4(),
            self.a4() * rhs.d1()
                + self.b4() * rhs.d2()
                + self.c4() * rhs.d3()
                + self.d4() * rhs.d4(),
        );

        Self::from_cols(x_axis, y_axis, z_axis, w_axis)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn normalize(&self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if length == 0.0 {
            return Self::IDENTITY;
        }

        Self::new(
            self.x / length,
            self.y / length,
            self.z / length,
            self.w / length,
        )
    }

    pub fn from_rotation_y(angle: f32) -> Self {
        let half_angle = angle * 0.5;

        Self::new(0.0, half_angle.sin(), 0.0, half_angle.cos()).normalize()
    }

    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let sin_half_angle = half_angle.sin();
        let cos_half_angle = half_angle.cos();

        Self::new(
            axis.x * sin_half_angle,
            axis.y * sin_half_angle,
            axis.z * sin_half_angle,
            cos_half_angle,
        )
        .normalize()
    }

    pub const fn inverse(self) -> Self {
        // The inverse of a unit quaternion is its conjugate
        Self::new(-self.x, -self.y, -self.z, self.w)
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        // The result is q * v * q_inv
        let q_inv = self.inverse();

        let v = Quat::new(rhs.x, rhs.y, rhs.z, 0.0);

        let qv = Quat::new(
            self.w * v.x + self.x * v.w + self.y * v.z - self.z * v.y,
            self.w * v.y + self.y * v.w + self.z * v.x - self.x * v.z,
            self.w * v.z + self.z * v.w + self.x * v.y - self.y * v.x,
            self.w * v.w - self.x * v.x - self.y * v.y - self.z * v.z,
        );

        // This is q * v * q_inv, but without the w component since we don't need it and it's zero
        Vec3::new(
            qv.w * q_inv.x + qv.x * q_inv.w + qv.y * q_inv.z - qv.z * q_inv.y,
            qv.w * q_inv.y + qv.y * q_inv.w + qv.z * q_inv.x - qv.x * q_inv.z,
            qv.w * q_inv.z + qv.z * q_inv.w + qv.x * q_inv.y - qv.y * q_inv.x,
        )
    }
}
