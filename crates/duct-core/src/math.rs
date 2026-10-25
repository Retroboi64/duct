//! Pure zero-dependency 3D mathematics module for Project32 Duct graphics subsystem.
//! Provides column-major 4x4 matrices, 2D/3D/4D vectors, and transformation utilities.

use std::ops::{Add, Mul, Sub};

/// 2-component vector.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self::new(self.x / len, self.y / len)
        } else {
            Self::new(0.0, 0.0)
        }
    }

    #[inline]
    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

/// 3-component vector.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// 4-component vector.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

/// 4x4 matrix stored in column-major order.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mat4 {
    pub cols: [[f32; 4]; 4],
}

impl Default for Mat4 {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl Mat4 {
    /// Identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a translation matrix.
    #[inline]
    pub const fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x,   y,   z,   1.0],
            ],
        }
    }

    /// Creates a uniform or non-uniform scaling matrix.
    #[inline]
    pub const fn scaling(x: f32, y: f32, z: f32) -> Self {
        Self {
            cols: [
                [x,   0.0, 0.0, 0.0],
                [0.0, y,   0.0, 0.0],
                [0.0, 0.0, z,   0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the X axis (pitch).
    #[inline]
    pub fn rotation_x(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            cols: [
                [1.0, 0.0,  0.0, 0.0],
                [0.0, c,    s,   0.0],
                [0.0, -s,   c,   0.0],
                [0.0, 0.0,  0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Y axis (yaw).
    #[inline]
    pub fn rotation_y(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            cols: [
                [c,   0.0, -s,  0.0],
                [0.0, 1.0, 0.0, 0.0],
                [s,   0.0, c,   0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a rotation matrix around the Z axis (roll).
    #[inline]
    pub fn rotation_z(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            cols: [
                [c,    s,   0.0, 0.0],
                [-s,   c,   0.0, 0.0],
                [0.0,  0.0, 1.0, 0.0],
                [0.0,  0.0, 0.0, 1.0],
            ],
        }
    }

    /// Creates a view matrix looking from `eye` towards `center` with up vector `up`.
    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let f = (center - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Self {
            cols: [
                [s.x, u.x, -f.x, 0.0],
                [s.y, u.y, -f.y, 0.0],
                [s.z, u.z, -f.z, 0.0],
                [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
            ],
        }
    }

    /// Creates a standard perspective projection matrix (compatible with OpenGL, DirectX, and Vulkan NDC).
    pub fn perspective(fov_y_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        let tan_half_fov = (fov_y_rad * 0.5).tan();
        let m00 = 1.0 / (aspect * tan_half_fov);
        let m11 = 1.0 / tan_half_fov;
        let m22 = -(z_far + z_near) / (z_far - z_near);
        let m23 = -1.0;
        let m32 = -(2.0 * z_far * z_near) / (z_far - z_near);

        Self {
            cols: [
                [m00, 0.0, 0.0, 0.0],
                [0.0, m11, 0.0, 0.0],
                [0.0, 0.0, m22, m23],
                [0.0, 0.0, m32, 0.0],
            ],
        }
    }

    /// Creates an orthographic projection matrix.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let r_l = 1.0 / (right - left);
        let t_b = 1.0 / (top - bottom);
        let f_n = 1.0 / (far - near);

        Self {
            cols: [
                [2.0 * r_l,           0.0,                 0.0,                0.0],
                [0.0,                 2.0 * t_b,           0.0,                0.0],
                [0.0,                 0.0,                -2.0 * f_n,          0.0],
                [-(right + left) * r_l, -(top + bottom) * t_b, -(far + near) * f_n, 1.0],
            ],
        }
    }

    /// Multiplies `self * rhs`.
    #[inline]
    pub fn mul_mat4(&self, rhs: &Self) -> Self {
        let mut out = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                out[c][r] = self.cols[0][r] * rhs.cols[c][0]
                    + self.cols[1][r] * rhs.cols[c][1]
                    + self.cols[2][r] * rhs.cols[c][2]
                    + self.cols[3][r] * rhs.cols[c][3];
            }
        }
        Self { cols: out }
    }

    /// Returns the matrix elements in column-major order as a flat 16-element array.
    #[inline]
    pub const fn to_cols_array(&self) -> [f32; 16] {
        [
            self.cols[0][0], self.cols[0][1], self.cols[0][2], self.cols[0][3],
            self.cols[1][0], self.cols[1][1], self.cols[1][2], self.cols[1][3],
            self.cols[2][0], self.cols[2][1], self.cols[2][2], self.cols[2][3],
            self.cols[3][0], self.cols[3][1], self.cols[3][2], self.cols[3][3],
        ]
    }
}

impl Mul for Mat4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat4(&rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_operations() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let cross = v1.cross(v2);
        assert!((cross.z - 1.0).abs() < 1e-5);
        assert_eq!(v1.dot(v2), 0.0);
        assert!((v1.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_mat4_identity_mul() {
        let id = Mat4::identity();
        let trans = Mat4::translation(1.0, 2.0, 3.0);
        let res = id * trans;
        assert_eq!(res.cols, trans.cols);
    }

    #[test]
    fn test_mat4_to_cols_array() {
        let id = Mat4::identity();
        let arr = id.to_cols_array();
        assert_eq!(arr[0], 1.0);
        assert_eq!(arr[5], 1.0);
        assert_eq!(arr[10], 1.0);
        assert_eq!(arr[15], 1.0);
        assert_eq!(arr[1], 0.0);
    }
}
