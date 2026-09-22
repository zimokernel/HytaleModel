//! JSON-shaped helper types.
//!
//! `vek` serializes vectors as sequences, but Hytale assets use
//! `{ "x": .., "y": .., "z": .. }` objects. These small mirrors keep the wire
//! format explicit and let each field carry its own default, which matters
//! because the format omits fields liberally.

use serde::Deserialize;

use crate::math::{Quaternion, Vec2, Vec3};

/// `{ "x": .., "y": .., "z": .. }`, all members optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
pub struct JsonVec3 {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub z: f32,
}

impl JsonVec3 {
    /// The multiplicative identity, `(1, 1, 1)`.
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    /// Shape `stretch` defaults to `(1, 1, 1)`, never `(0, 0, 0)`.
    pub fn one() -> Self {
        Self::ONE
    }

    pub fn to_vec3(self) -> Vec3<f32> {
        Vec3::new(self.x, self.y, self.z)
    }
}

/// `{ "x": .., "y": .. }`, all members optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
pub struct JsonVec2 {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

impl JsonVec2 {
    pub fn to_vec2(self) -> Vec2<f32> {
        Vec2::new(self.x, self.y)
    }
}

/// `{ "x": .., "y": .., "z": .., "w": .. }`; a missing `w` means identity.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct JsonQuat {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub z: f32,
    #[serde(default = "one")]
    pub w: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for JsonQuat {
    /// The identity rotation. Used when a node omits `orientation` entirely.
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

impl JsonQuat {
    pub fn to_quat(self) -> Quaternion<f32> {
        Quaternion::from_xyzw(self.x, self.y, self.z, self.w).normalized()
    }
}

/// Conversion into the `vek` type of the same arity.
///
/// Lets the animation sampler be generic over keyframe tracks without
/// allocating or hand-writing one sampler per channel.
pub trait ToVek {
    type Out;
    fn to_vek(self) -> Self::Out;
}

impl ToVek for JsonVec3 {
    type Out = Vec3<f32>;
    fn to_vek(self) -> Vec3<f32> {
        self.to_vec3()
    }
}

impl ToVek for JsonVec2 {
    type Out = Vec2<f32>;
    fn to_vek(self) -> Vec2<f32> {
        self.to_vec2()
    }
}

/// Node ids are strings in every asset seen so far, but accept bare numbers
/// too so a slightly-off exporter does not break the whole load.
pub(crate) fn de_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Str(String),
        Num(f64),
    }

    Ok(match Id::deserialize(deserializer)? {
        Id::Str(s) => s,
        Id::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", n as i64)
            } else {
                n.to_string()
            }
        }
    })
}
