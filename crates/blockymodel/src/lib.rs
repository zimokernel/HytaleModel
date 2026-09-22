//! # blockymodel
//!
//! A parser, skeleton, mesh builder and animation sampler for Hytale's
//! Blockbench asset formats: `.blockymodel` (model) and `.blockyanim`
//! (animation).
//!
//! The layout follows the format specification in `docs/blockymodel-format.md`.
//! Everything here is renderer-agnostic — no GPU types leak out of this crate,
//! and the math types are the same `vek` `repr_c` types the Aether_terrain
//! renderer uses, so the output drops straight into that pipeline.
//!
//! ```no_run
//! use blockymodel::{BlockyAnim, BlockyModel, MeshBuilder, Skeleton};
//!
//! let model = BlockyModel::from_path("Pets/Dog/Models/Model.blockymodel")?;
//! let anim = BlockyAnim::from_path("Pets/Dog/Animations/Idle.blockyanim")?;
//!
//! let skeleton = Skeleton::from_model(&model);
//! let mesh = MeshBuilder::new(256.0, 128.0).build(&skeleton);
//!
//! // Per frame:
//! let mut pose = skeleton.bind_pose();
//! skeleton.apply(&mut pose, Some(&anim), 0.5 /* seconds */);
//! let bone_matrices = skeleton.bone_matrices(&pose);
//! # Ok::<(), blockymodel::Error>(())
//! ```

mod anim;
mod json;
mod math;
mod mesh;
mod model;
mod skeleton;

pub use anim::{
    weighted_cubic_bezier, BlockyAnim, ChannelDelta, ChannelValue, Interpolation, KeyTimes,
    KeyTrack, NodeTracks, OrientationKey, PositionKey, StretchKey, UvOffsetKey, VisibleKey, FPS,
};
pub use json::{JsonQuat, JsonVec2, JsonVec3, ToVek};
pub use math::{
    max3, min3, normalize_or_zero, recip_or_zero, Mat3, Mat4, Quaternion, Vec2, Vec3, Vec4,
};
pub use mesh::{Face, Mesh, MeshBuilder, Vertex};
pub use model::{
    compose_bone, BlockyModel, Bounds, Face6, Mirror, Node, ShadingMode, Shape, ShapeSettings,
    ShapeType, TextureFace,
};
pub use skeleton::{Bone, BoneMatrix, Pose, Skeleton};

/// Errors produced while loading or interpreting Hytale assets.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

impl BlockyModel {
    /// Reads and parses a `.blockymodel` file from disk.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_json(&text).map_err(|source| Error::Json {
            path: path.display().to_string(),
            source,
        })
    }

    /// Parses a `.blockymodel` document from a JSON string.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

impl BlockyAnim {
    /// Reads and parses a `.blockyanim` file from disk.
    ///
    /// The keyframe tracks are sorted on the way in, which sampling relies on.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
        let mut anim: Self = Self::from_json(&text).map_err(|source| Error::Json {
            path: path.display().to_string(),
            source,
        })?;
        anim.prepare();
        Ok(anim)
    }

    /// Parses a `.blockyanim` document from a JSON string.
    ///
    /// Remember to call [`BlockyAnim::prepare`] before sampling, or sort the
    /// tracks yourself.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}
