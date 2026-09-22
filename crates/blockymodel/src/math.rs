//! The slice of `vek` this crate uses.
//!
//! Deliberately re-exported from the `repr_c` modules, because those are the
//! exact types `vrage_anim::vek` hands to `vrage_render` in
//! `D:\work\ttc\Aether_terrain`. Using them means the skeleton and mesh data
//! produced here drop straight into that renderer with no conversion layer.

pub use vek::{
    mat::repr_c::column_major::{Mat3, Mat4},
    quaternion::repr_c::Quaternion,
    vec::repr_c::{Vec2, Vec3, Vec4},
};

/// Component-wise minimum.
pub fn min3(a: Vec3<f32>, b: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
}

/// Component-wise maximum.
pub fn max3(a: Vec3<f32>, b: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
}

/// `Vec3::normalized`, but yields the zero vector instead of `NaN`.
pub fn normalize_or_zero(v: Vec3<f32>) -> Vec3<f32> {
    let magnitude = v.magnitude();
    if magnitude > f32::EPSILON {
        v / magnitude
    } else {
        Vec3::zero()
    }
}

/// Component-wise reciprocal, leaving non-finite entries at zero.
///
/// Used to build the inverse-transpose of a diagonal scale, which is where
/// stretched axes turn into compressed normals.
pub fn recip_or_zero(v: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(recip(v.x), recip(v.y), recip(v.z))
}

fn recip(v: f32) -> f32 {
    if v.abs() > f32::EPSILON {
        1.0 / v
    } else {
        0.0
    }
}
