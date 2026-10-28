//! 3D mathematics module for Project32 Duct graphics subsystem powered by `numix`.
//!
//! Provides column-major 4x4 matrices, 2D/3D/4D vectors, quaternions, and transformation
//! utilities for camera, perspective projections, rotations, and shader uniform uploads.

pub use numix::*;

/// 2-component vector (single-precision).
pub type Vec2 = numix::types::Vec2<f32>;

/// 3-component vector (single-precision).
pub type Vec3 = numix::types::Vec3<f32>;

/// 4-component vector (single-precision).
pub type Vec4 = numix::types::Vec4<f32>;

/// 4x4 matrix (single-precision column-major compatible).
pub type Mat4 = numix::types::Mat4x4<f32>;

/// Quaternion (single-precision).
pub type Quat = numix::quat::Quat<f32>;

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
        assert_eq!(Vec3::UP, Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_mat4_identity_mul() {
        let id = Mat4::identity();
        let trans = Mat4::translation(1.0, 2.0, 3.0);
        let res = id * trans;
        assert_eq!(res.mat, trans.mat);
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

    #[test]
    fn test_perspective_projections() {
        let gl = Mat4::perspective_gl(1.0, 1.0, 0.1, 100.0);
        let d3d = Mat4::perspective_d3d_vk(1.0, 1.0, 0.1, 100.0);
        let rev = Mat4::perspective_reverse_z(1.0, 1.0, 0.1, 100.0);

        // Near plane test: point at (0, 0, -0.1, 1.0)
        let pt_near = Vec4::new(0.0, 0.0, -0.1, 1.0);
        let gl_p = gl * pt_near;
        assert!((gl_p.z / gl_p.w - (-1.0)).abs() < 1e-4);

        let d3d_p = d3d * pt_near;
        assert!((d3d_p.z / d3d_p.w - 0.0).abs() < 1e-4);

        let rev_p = rev * pt_near;
        assert!((rev_p.z / rev_p.w - 1.0).abs() < 1e-4);

        // Far plane test: point at (0, 0, -100.0, 1.0)
        let pt_far = Vec4::new(0.0, 0.0, -100.0, 1.0);
        let d3d_f = d3d * pt_far;
        assert!((d3d_f.z / d3d_f.w - 1.0).abs() < 1e-4);

        let rev_f = rev * pt_far;
        assert!((rev_f.z / rev_f.w - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_look_at() {
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::UP;
        let view = Mat4::look_at(eye, center, up);

        let pt = Vec4::new(0.0, 0.0, 0.0, 1.0);
        let viewed = view * pt;
        assert!((viewed.x).abs() < 1e-5);
        assert!((viewed.y).abs() < 1e-5);
        assert!((viewed.z - (-5.0)).abs() < 1e-5);
    }
}
