//! `.blockymodel` — the model format.
//!
//! Mirrors `docs/blockymodel-format.md` §4. Every field is optional or
//! defaulted, because Hytale's own assets omit plenty of them.

use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    json::{de_id, JsonQuat, JsonVec2, JsonVec3},
    math::{max3, min3, Mat4, Quaternion, Vec2, Vec3},
};

/// The six axis-aligned faces of a block, using Hytale's own naming.
///
/// Note that `Front` is **+Z** — Hytale's `front` is Blockbench's `south`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face6 {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

impl Face6 {
    /// All six faces, in the order the reference exporter emits them.
    pub const ALL: [Face6; 6] = [
        Face6::Right,
        Face6::Left,
        Face6::Top,
        Face6::Bottom,
        Face6::Front,
        Face6::Back,
    ];

    /// The `textureLayout` key for this face.
    pub fn name(self) -> &'static str {
        match self {
            Face6::Right => "right",
            Face6::Left => "left",
            Face6::Top => "top",
            Face6::Bottom => "bottom",
            Face6::Front => "front",
            Face6::Back => "back",
        }
    }

    /// The outward normal in model space (right-handed, +Y up, +Z forward).
    pub fn normal(self) -> Vec3<f32> {
        match self {
            Face6::Right => Vec3::new(1.0, 0.0, 0.0),
            Face6::Left => Vec3::new(-1.0, 0.0, 0.0),
            Face6::Top => Vec3::new(0.0, 1.0, 0.0),
            Face6::Bottom => Vec3::new(0.0, -1.0, 0.0),
            Face6::Front => Vec3::new(0.0, 0.0, 1.0),
            Face6::Back => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    /// Parses a `settings.normal` / `textureLayout` key.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "right" | "+X" => Face6::Right,
            "left" | "-X" => Face6::Left,
            "top" | "+Y" => Face6::Top,
            "bottom" | "-Y" => Face6::Bottom,
            "front" | "+Z" => Face6::Front,
            "back" | "-Z" => Face6::Back,
            _ => return None,
        })
    }
}

/// The root of a `.blockymodel` document.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockyModel {
    /// Root nodes. The format allows more than one.
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Only ever seen as `"auto"`; ignored by renderers.
    #[serde(default)]
    pub lod: Option<String>,
    /// `"character"` or `"prop"`; only meaningful to the editor.
    #[serde(default)]
    pub format: Option<String>,
}

/// One bone (and, usually, one shape) in the hierarchy.
#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    /// A string in the wire format — never deserialize this as an integer.
    #[serde(default, deserialize_with = "de_id")]
    pub id: String,
    /// Bone name; animations bind to this, and names are **not** unique.
    #[serde(default)]
    pub name: String,
    /// Translation relative to the parent bone.
    #[serde(default)]
    pub position: JsonVec3,
    /// Rotation, `(x, y, z, w)`, scalar last. Missing means identity.
    #[serde(default)]
    pub orientation: JsonQuat,
    #[serde(default)]
    pub shape: Option<Shape>,
    #[serde(default)]
    pub children: Vec<Node>,
}

/// A node's geometry: a box, a flat quad, or nothing at all.
#[derive(Debug, Clone, Deserialize)]
pub struct Shape {
    /// Offset of the shape centre from the bone origin. Also becomes the
    /// child bones' pivot offset — see §5 of the spec.
    #[serde(default)]
    pub offset: JsonVec3,
    /// **Scale multiplier**, not an absolute size. May be negative.
    #[serde(default = "JsonVec3::one")]
    pub stretch: JsonVec3,
    #[serde(rename = "type", default)]
    pub kind: ShapeType,
    #[serde(default)]
    pub settings: ShapeSettings,
    /// Per-face UV rectangles. A face is only drawn if it appears here.
    #[serde(rename = "textureLayout", default)]
    pub texture_layout: HashMap<String, TextureFace>,
    /// Only ever `"custom"`.
    #[serde(rename = "unwrapMode", default)]
    pub unwrap_mode: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(rename = "doubleSided", default)]
    pub double_sided: Option<bool>,
    #[serde(rename = "shadingMode", default)]
    pub shading_mode: Option<String>,
}

impl Shape {
    pub fn kind(&self) -> ShapeType {
        self.kind
    }

    /// `true` when this shape produces triangles.
    pub fn has_geometry(&self) -> bool {
        matches!(self.kind, ShapeType::Box | ShapeType::Quad)
    }

    pub fn stretch_vec(&self) -> Vec3<f32> {
        self.stretch.to_vec3()
    }

    pub fn offset_vec(&self) -> Vec3<f32> {
        self.offset.to_vec3()
    }

    /// Un-stretched size. Quad sizes only carry `x`/`y`; `z` stays 0.
    pub fn size(&self) -> Vec3<f32> {
        self.settings.size.unwrap_or(JsonVec3::ONE).to_vec3()
    }

    /// The `settings.normal` plane for a quad. Defaults to `+Z`.
    pub fn quad_normal(&self) -> Face6 {
        self.settings
            .normal
            .as_deref()
            .and_then(Face6::from_name)
            .unwrap_or(Face6::Front)
    }

    pub fn is_visible(&self) -> bool {
        self.visible.unwrap_or(true)
    }

    pub fn is_double_sided(&self) -> bool {
        self.double_sided.unwrap_or(false)
    }

    pub fn shading_mode(&self) -> ShadingMode {
        match self.shading_mode.as_deref() {
            Some("standard") => ShadingMode::Standard,
            Some("fullbright") => ShadingMode::Fullbright,
            Some("reflective") => ShadingMode::Reflective,
            _ => ShadingMode::Flat,
        }
    }

    /// Looks up a face's texture layout.
    ///
    /// Quads only ever author `front`, and the reference renderer falls back to
    /// it when the natural face has no layout, so callers should use
    /// [`Shape::quad_normal`] plus this rather than assuming an entry exists.
    pub fn layout(&self, face: Face6) -> Option<&TextureFace> {
        self.texture_layout.get(face.name())
    }
}

/// How a shape should be lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// Unlit: the texture is used as-is. Hytale's own assets are all `flat`.
    #[default]
    Flat,
    /// Directional diffuse lighting.
    Standard,
    /// Unlit, explicitly fully bright.
    Fullbright,
    /// Unlit; the game reflects an environment map here.
    Reflective,
}

impl ShadingMode {
    /// `1.0` means "do not light this fragment".
    pub fn unlit_factor(self) -> f32 {
        match self {
            ShadingMode::Standard => 0.0,
            _ => 1.0,
        }
    }
}

/// `shape.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeType {
    Box,
    Quad,
    /// Skeleton / attachment point with no geometry.
    #[serde(rename = "none")]
    Empty,
    #[default]
    #[serde(other)]
    Unknown,
}

/// `shape.settings`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ShapeSettings {
    /// Boxes carry `{x,y,z}`; quads only `{x,y}`.
    #[serde(default)]
    pub size: Option<JsonVec3>,
    /// Quads only: `"+X" | "-X" | "+Y" | "-Y" | "+Z" | "-Z"`.
    #[serde(default)]
    pub normal: Option<String>,
    /// Attachment marker: merge onto the main model's bone of the same name.
    #[serde(rename = "isPiece", default)]
    pub is_piece: bool,
    /// Editor-only hint about how the node tree was authored.
    #[serde(rename = "isStaticBox", default)]
    pub is_static_box: bool,
}

/// One face's UV rectangle inside the shared texture.
#[derive(Debug, Clone, Deserialize)]
pub struct TextureFace {
    /// Top-left corner in **texture pixels**, not normalized units.
    pub offset: JsonVec2,
    #[serde(default)]
    pub mirror: Mirror,
    /// `0 | 90 | 180 | 270`.
    #[serde(default)]
    pub angle: f32,
    /// Optional: composited instead of alpha-tested.
    #[serde(default)]
    pub transparent: bool,
    #[serde(rename = "lockUVs", default)]
    pub lock_uvs: bool,
}

impl TextureFace {
    pub fn offset_vec2(&self) -> Vec2<f32> {
        self.offset.to_vec2()
    }

    /// Normalized rotation in degrees, forced into `0..360`.
    pub fn angle_deg(&self) -> i32 {
        let a = self.angle as i32;
        ((a % 360) + 360) % 360
    }
}

/// Per-axis UV mirroring. `mirror.x = true` makes the rectangle grow *left*
/// from `offset`.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Mirror {
    #[serde(default)]
    pub x: bool,
    #[serde(default)]
    pub y: bool,
}

/// Axis-aligned bounds in model space.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: Vec3<f32>,
    pub max: Vec3<f32>,
}

impl Bounds {
    /// The inverted box that [`Bounds::expand`] grows from.
    pub const EMPTY: Bounds = Bounds {
        min: Vec3::broadcast(f32::INFINITY),
        max: Vec3::broadcast(f32::NEG_INFINITY),
    };

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    pub fn expand(&mut self, p: Vec3<f32>) {
        self.min = min3(self.min, p);
        self.max = max3(self.max, p);
    }

    pub fn center(&self) -> Vec3<f32> {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3<f32> {
        self.max - self.min
    }

    pub fn radius(&self) -> f32 {
        if self.is_empty() {
            1.0
        } else {
            self.size().magnitude() * 0.5
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Applies the bind-pose bone transform, exactly as described in §5 of the
/// spec:
///
/// ```text
/// bone_world(N) = bone_world(P) · T(P.shape.offset + N.position) · R(N.orientation)
/// ```
///
/// Note the order: the parent's shape offset **and** the node's own position
/// are translated first, then the node's rotation is applied. Swapping the two
/// throws every joint across the model.
pub fn compose_bone(
    parent_world: Mat4<f32>,
    parent_shape_offset: Vec3<f32>,
    position: Vec3<f32>,
    orientation: Quaternion<f32>,
) -> Mat4<f32> {
    parent_world * Mat4::translation_3d(parent_shape_offset + position) * Mat4::from(orientation)
}
