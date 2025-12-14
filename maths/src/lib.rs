use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::ops::Div;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn from_array(arr: &[f32; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }

    pub const fn to_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        if length == 0.0 {
            return;
        }

        self.x /= length;
        self.y /= length;
        self.z /= length;
    }

    pub fn normalized(&self) -> Self {
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

    pub const fn min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
    }
    pub const fn max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
    }

    pub const fn extend(&self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
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

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
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

    pub const fn from_array(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }

    pub const fn from_point(point: Vec3) -> Self {
        Self::new(point.x, point.y, point.z, 1.0)
    }

    pub const fn from_direction(vector: Vec3) -> Self {
        Self::new(vector.x, vector.y, vector.z, 0.0)
    }

    pub const fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    pub const fn dot(&self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        if length == 0.0 {
            return;
        }

        self.x /= length;
        self.y /= length;
        self.z /= length;
        self.w /= length;
    }

    pub fn normalized(&self) -> Self {
        let length = self.length();
        if length == 0.0 {
            return Self::ZERO;
        }

        Self::new(
            self.x / length,
            self.y / length,
            self.z / length,
            self.w / length,
        )
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
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

    pub const fn from_cols_array_2d(columns: [[f32; 4]; 4]) -> Self {
        Self::from_cols(
            Vec4::from_array(columns[0]),
            Vec4::from_array(columns[1]),
            Vec4::from_array(columns[2]),
            Vec4::from_array(columns[3]),
        )
    }

    pub const fn from_rows_array_2d(rows: [[f32; 4]; 4]) -> Self {
        Self::from_cols(
            Vec4::new(rows[0][0], rows[1][0], rows[2][0], rows[3][0]),
            Vec4::new(rows[0][1], rows[1][1], rows[2][1], rows[3][1]),
            Vec4::new(rows[0][2], rows[1][2], rows[2][2], rows[3][2]),
            Vec4::new(rows[0][3], rows[1][3], rows[2][3], rows[3][3]),
        )
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
        let coa1 = Self::det3(b2, b3, b4, c2, c3, c4, d2, d3, d4);
        let coa2 = -Self::det3(b1, b3, b4, c1, c3, c4, d1, d3, d4);
        let coa3 = Self::det3(b1, b2, b4, c1, c2, c4, d1, d2, d4);
        let coa4 = -Self::det3(b1, b2, b3, c1, c2, c3, d1, d2, d3);

        let cob1 = -Self::det3(a2, a3, a4, c2, c3, c4, d2, d3, d4);
        let cob2 = Self::det3(a1, a3, a4, c1, c3, c4, d1, d3, d4);
        let cob3 = -Self::det3(a1, a2, a4, c1, c2, c4, d1, d2, d4);
        let cob4 = Self::det3(a1, a2, a3, c1, c2, c3, d1, d2, d3);

        let coc1 = Self::det3(a2, a3, a4, b2, b3, b4, d2, d3, d4);
        let coc2 = -Self::det3(a1, a3, a4, b1, b3, b4, d1, d3, d4);
        let coc3 = Self::det3(a1, a2, a4, b1, b2, b4, d1, d2, d4);
        let coc4 = -Self::det3(a1, a2, a3, b1, b2, b3, d1, d2, d3);

        let cod1 = -Self::det3(a2, a3, a4, b2, b3, b4, c2, c3, c4);
        let cod2 = Self::det3(a1, a3, a4, b1, b3, b4, c1, c3, c4);
        let cod3 = -Self::det3(a1, a2, a4, b1, b2, b4, c1, c2, c4);
        let cod4 = Self::det3(a1, a2, a3, b1, b2, b3, c1, c2, c3);

        // Return the cofactor matrix
        Self::from_cols(
            Vec4::new(coa1, coa2, coa3, coa4),
            Vec4::new(cob1, cob2, cob3, cob4),
            Vec4::new(coc1, coc2, coc3, coc4),
            Vec4::new(cod1, cod2, cod3, cod4),
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

    pub fn from_translation(translation: Vec3) -> Self {
        Self::from_cols(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(translation.x, translation.y, translation.z, 1.0),
        )
    }

    pub fn from_rotation(quat: Quat) -> Self {
        let (x, y, z, w) = (quat.x, quat.y, quat.z, quat.w);
        let (x2, y2, z2) = (x + x, y + y, z + z);

        let xx2 = x * x2;
        let yy2 = y * y2;
        let zz2 = z * z2;
        let xy2 = x * y2;
        let xz2 = x * z2;
        let yz2 = y * z2;
        let wx2 = w * x2;
        let wy2 = w * y2;
        let wz2 = w * z2;

        Self::from_cols(
            Vec4::new(1.0 - (yy2 + zz2), xy2 + wz2, xz2 - wy2, 0.0),
            Vec4::new(xy2 - wz2, 1.0 - (xx2 + zz2), yz2 + wx2, 0.0),
            Vec4::new(xz2 + wy2, yz2 - wx2, 1.0 - (xx2 + yy2), 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self::from_cols(
            Vec4::new(scale.x, 0.0, 0.0, 0.0),
            Vec4::new(0.0, scale.y, 0.0, 0.0),
            Vec4::new(0.0, 0.0, scale.z, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    pub fn extract_translation(&self) -> Vec3 {
        Vec3::new(self.w_axis.x, self.w_axis.y, self.w_axis.z)
    }

    pub fn extract_scale(&self) -> Vec3 {
        let scale_x = Vec3::new(self.x_axis.x, self.x_axis.y, self.x_axis.z).length();
        let scale_y = Vec3::new(self.y_axis.x, self.y_axis.y, self.y_axis.z).length();
        let scale_z = Vec3::new(self.z_axis.x, self.z_axis.y, self.z_axis.z).length();
        Vec3::new(scale_x, scale_y, scale_z)
    }

    pub fn extract_rotation(&self) -> Quat {
        let scale = self.extract_scale();

        let rotation_matrix = Self::from_cols(
            self.x_axis / scale.x,
            self.y_axis / scale.y,
            self.z_axis / scale.z,
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        Quat::from_rotation_matrix(&rotation_matrix)
    }

    #[allow(clippy::too_many_arguments)]
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

impl Mul<Vec3> for Mat4 {
    type Output = Vec4;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec4::new(
            self.a1() * rhs.x + self.b1() * rhs.y + self.c1() * rhs.z,
            self.a2() * rhs.x + self.b2() * rhs.y + self.c2() * rhs.z,
            self.a3() * rhs.x + self.b3() * rhs.y + self.c3() * rhs.z,
            1.0,
        )
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    fn mul(self, rhs: Vec4) -> Self::Output {
        Vec4::new(
            self.a1() * rhs.x + self.b1() * rhs.y + self.c1() * rhs.z + self.d1() * rhs.w,
            self.a2() * rhs.x + self.b2() * rhs.y + self.c2() * rhs.z + self.d2() * rhs.w,
            self.a3() * rhs.x + self.b3() * rhs.y + self.c3() * rhs.z + self.d3() * rhs.w,
            self.a4() * rhs.x + self.b4() * rhs.y + self.c4() * rhs.z + self.d4() * rhs.w,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
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

    pub const fn from_array(arr: &[f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
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

    pub fn from_rotation_matrix(mat: &Mat4) -> Self {
        let m11 = mat.x_axis.x;
        let m12 = mat.y_axis.x;
        let m13 = mat.z_axis.x;
        let m21 = mat.x_axis.y;
        let m22 = mat.y_axis.y;
        let m23 = mat.z_axis.y;
        let m31 = mat.x_axis.z;
        let m32 = mat.y_axis.z;
        let m33 = mat.z_axis.z;

        let trace = m11 + m22 + m33;

        let quat = if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            Self::new((m32 - m23) / s, (m13 - m31) / s, (m21 - m12) / s, 0.25 * s)
        } else if m11 > m22 && m11 > m33 {
            let s = (1.0 + m11 - m22 - m33).sqrt() * 2.0;
            Self::new(0.25 * s, (m12 + m21) / s, (m13 + m31) / s, (m32 - m23) / s)
        } else if m22 > m33 {
            let s = (1.0 + m22 - m11 - m33).sqrt() * 2.0;
            Self::new((m12 + m21) / s, 0.25 * s, (m23 + m32) / s, (m13 - m31) / s)
        } else {
            let s = (1.0 + m33 - m11 - m22).sqrt() * 2.0;
            Self::new((m13 + m31) / s, (m23 + m32) / s, 0.25 * s, (m21 - m12) / s)
        };

        quat.normalize()
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

/// Transform Gizmo integration
impl From<Vec3> for transform_gizmo_egui::mint::Vector3<f64> {
    fn from(val: Vec3) -> Self {
        transform_gizmo_egui::mint::Vector3 {
            x: val.x as f64,
            y: val.y as f64,
            z: val.z as f64,
        }
    }
}

impl From<transform_gizmo_egui::mint::Vector3<f64>> for Vec3 {
    fn from(value: transform_gizmo_egui::mint::Vector3<f64>) -> Self {
        Self::new(value.x as f32, value.y as f32, value.z as f32)
    }
}

impl From<Quat> for transform_gizmo_egui::mint::Quaternion<f64> {
    fn from(val: Quat) -> Self {
        transform_gizmo_egui::mint::Quaternion {
            v: transform_gizmo_egui::mint::Vector3 {
                x: val.x as f64,
                y: val.y as f64,
                z: val.z as f64,
            },
            s: val.w as f64,
        }
    }
}

impl From<transform_gizmo_egui::mint::Quaternion<f64>> for Quat {
    fn from(value: transform_gizmo_egui::mint::Quaternion<f64>) -> Self {
        Self::new(
            value.v.x as f32,
            value.v.y as f32,
            value.v.z as f32,
            value.s as f32,
        )
    }
}

impl From<Vec4> for transform_gizmo_egui::mint::Vector4<f64> {
    fn from(val: Vec4) -> Self {
        transform_gizmo_egui::mint::Vector4 {
            x: val.x as f64,
            y: val.y as f64,
            z: val.z as f64,
            w: val.w as f64,
        }
    }
}

impl From<Mat4> for transform_gizmo_egui::mint::RowMatrix4<f64> {
    fn from(val: Mat4) -> Self {
        let x = Vec4::new(val.a1(), val.b1(), val.c1(), val.d1());
        let y = Vec4::new(val.a2(), val.b2(), val.c2(), val.d2());
        let z = Vec4::new(val.a3(), val.b3(), val.c3(), val.d3());
        let w = Vec4::new(val.a4(), val.b4(), val.c4(), val.d4());

        transform_gizmo_egui::mint::RowMatrix4 {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            w: w.into(),
        }
    }
}
