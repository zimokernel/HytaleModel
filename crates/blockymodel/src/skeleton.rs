//! The flattened bone hierarchy and per-frame pose evaluation.

use std::collections::HashMap;

use crate::{
    anim::{BlockyAnim, ChannelDelta, FPS},
    math::{recip_or_zero, Mat4, Quaternion, Vec2, Vec3},
    model::{BlockyModel, Bounds, Face6, Node, Shape, ShapeType},
};

/// One bone, flattened out of the source tree.
#[derive(Debug, Clone)]
pub struct Bone {
    /// Source `id` string.
    pub id: String,
    /// Bone name. **Not unique** — several bones may share one.
    pub name: String,
    /// Bind translation relative to the parent bone.
    pub position: Vec3<f32>,
    /// Bind rotation.
    pub orientation: Quaternion<f32>,
    /// Offset of the shape centre from the bone origin. This is also the pivot
    /// offset handed down to child bones, and it is never animated.
    pub shape_offset: Vec3<f32>,
    /// Bind scale multiplier.
    pub shape_stretch: Vec3<f32>,
    /// The geometry, if this bone has any. `None`, or a shape with
    /// `kind == Empty`, means a pure bone.
    pub shape: Option<Shape>,
    /// Index into [`Skeleton::bones`]; always `< self index`.
    pub parent: Option<usize>,
    /// Depth from the root, for debugging.
    pub depth: u32,
}

impl Bone {
    /// The bone's mesh, if it draws anything.
    pub fn geometry(&self) -> Option<&Shape> {
        self.shape.as_ref().filter(|s| s.has_geometry())
    }
}

/// A model flattened into arrays, ready for per-frame evaluation.
#[derive(Debug, Clone)]
pub struct Skeleton {
    /// Bones in pre-order: a bone's parent always has a lower index.
    pub bones: Vec<Bone>,
    /// `name -> indices`. Always a list: names repeat in real assets.
    pub by_name: HashMap<String, Vec<usize>>,
    /// Bind-pose world bounds, including every shape's full extents.
    pub bounds: Bounds,
}

/// Deltas for every bone in a [`Skeleton`], relative to the bind pose.
#[derive(Debug, Clone, Default)]
pub struct Pose {
    pub deltas: Vec<ChannelDelta>,
}

impl Pose {
    /// Every bone back at its bind pose.
    pub fn reset(&mut self) {
        for d in &mut self.deltas {
            *d = ChannelDelta::default();
        }
    }
}

/// What a renderer needs for one bone.
///
/// Shaped to match `vrage_render::pipelines::figure::BoneData` — `bone_mat`
/// plus `normals_mat`, both column-major `[[f32; 4]; 4]` — with one addition:
/// the animated UV offset, which this renderer supports and the figure
/// pipeline does not.
#[derive(Debug, Clone, Copy)]
pub struct BoneMatrix {
    /// Maps a baked shape-local vertex into model space:
    /// `bone_world · T(shape.offset) · S(animated stretch)`.
    pub bone_mat: Mat4<f32>,
    /// `bone_mat`'s linear part, inverted and transposed, for normals.
    pub normals_mat: Mat4<f32>,
    /// Animated UV pixel offset for this bone's faces.
    pub uv_offset: Vec2<f32>,
}

impl Skeleton {
    /// Flattens a parsed model into arrays.
    pub fn from_model(model: &BlockyModel) -> Self {
        let mut bones = Vec::new();
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();

        fn walk(
            nodes: &[Node],
            parent: Option<usize>,
            depth: u32,
            bones: &mut Vec<Bone>,
            by_name: &mut HashMap<String, Vec<usize>>,
        ) {
            for node in nodes {
                let index = bones.len();
                let shape = node.shape.clone();
                let shape_offset = shape
                    .as_ref()
                    .map(Shape::offset_vec)
                    .unwrap_or(Vec3::zero());
                let shape_stretch = shape
                    .as_ref()
                    .map(Shape::stretch_vec)
                    .unwrap_or(Vec3::one());

                bones.push(Bone {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    position: node.position.to_vec3(),
                    orientation: node.orientation.to_quat(),
                    shape_offset,
                    shape_stretch,
                    shape,
                    parent,
                    depth,
                });
                by_name.entry(node.name.clone()).or_default().push(index);

                walk(&node.children, Some(index), depth + 1, bones, by_name);
            }
        }

        walk(&model.nodes, None, 0, &mut bones, &mut by_name);

        let mut skeleton = Self {
            bones,
            by_name,
            bounds: Bounds::EMPTY,
        };
        skeleton.bounds = skeleton.compute_bind_bounds();
        skeleton
    }

    /// Number of bones.
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    /// A pose with every bone at its bind value.
    pub fn bind_pose(&self) -> Pose {
        Pose {
            deltas: vec![ChannelDelta::default(); self.bones.len()],
        }
    }

    /// Lists the names an animation drives that this skeleton does not have.
    ///
    /// Real assets reference bones from other variants (the Dog's animations
    /// mention `Collar` and `TailDog`, which the Dog model lacks), so this is
    /// informational only — binding always ignores unknown names.
    pub fn unknown_animated_bones(&self, anim: &BlockyAnim) -> Vec<String> {
        let mut names: Vec<String> = anim
            .node_animations
            .keys()
            .filter(|name| !self.by_name.contains_key(*name))
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Absolute bind-pose transform of each bone, before its shape offset.
    pub fn bind_bone_world(&self) -> Vec<Mat4<f32>> {
        let mut out = vec![Mat4::identity(); self.bones.len()];
        for (i, bone) in self.bones.iter().enumerate() {
            let parent_world = bone.parent.map(|p| out[p]).unwrap_or(Mat4::identity());
            let parent_offset = self.parent_offset(bone);
            out[i] = parent_world
                * Mat4::translation_3d(parent_offset + bone.position)
                * Mat4::from(bone.orientation);
        }
        out
    }

    /// Bind-pose transform that maps a **baked** shape-local vertex into model
    /// space, per bone.
    ///
    /// Baked vertices already carry the bind stretch (see
    /// [`crate::MeshBuilder`]), so this deliberately stops at
    /// `bone_world · T(shape.offset)` — exactly what [`Skeleton::bone_matrices`]
    /// produces when no animation is applied.
    pub fn bind_shape_world(&self) -> Vec<Mat4<f32>> {
        let bone_world = self.bind_bone_world();
        self.bones
            .iter()
            .enumerate()
            .map(|(i, bone)| bone_world[i] * Mat4::translation_3d(bone.shape_offset))
            .collect()
    }

    fn parent_offset(&self, bone: &Bone) -> Vec3<f32> {
        bone.parent
            .map(|p| self.bones[p].shape_offset)
            .unwrap_or(Vec3::zero())
    }

    /// Samples `anim` at `time_seconds` into `pose`.
    ///
    /// Passing `None` resets `pose` to the bind pose. Bone names with no
    /// matching track are left alone, as are tracks with no matching bone.
    pub fn apply(&self, pose: &mut Pose, anim: Option<&BlockyAnim>, time_seconds: f32) {
        if pose.deltas.len() != self.bones.len() {
            pose.deltas
                .resize(self.bones.len(), ChannelDelta::default());
        }
        pose.reset();

        let Some(anim) = anim else { return };
        if anim.duration <= 0.0 {
            return;
        }

        let raw_frame = time_seconds * FPS;
        let frame = if anim.hold_last_keyframe {
            raw_frame.clamp(0.0, anim.duration)
        } else {
            raw_frame.rem_euclid(anim.duration)
        };

        for (i, bone) in self.bones.iter().enumerate() {
            if let Some(tracks) = anim.node_animations.get(&bone.name) {
                tracks.sample(frame, anim.duration, anim.wraps(), &mut pose.deltas[i]);
            }
        }
    }

    /// Turns a pose into per-bone GPU-ready matrices.
    ///
    /// Vertices are expected to be pre-baked with the bind stretch (see
    /// [`crate::MeshBuilder`]), so the runtime matrix only carries the
    /// *animated* stretch multiplier.
    ///
    /// The normal matrix is built analytically rather than by inverting the
    /// model matrix: stretch only ever scales a bone's own mesh, never its
    /// children, so the accumulated transform is rigid and the linear part is
    /// simply `R · S`. Its inverse-transpose is therefore `R · S⁻¹`.
    pub fn bone_matrices(&self, pose: &Pose) -> Vec<BoneMatrix> {
        let collapsed = collapsed_matrix();
        let mut out = vec![
            BoneMatrix {
                bone_mat: collapsed,
                normals_mat: Mat4::identity(),
                uv_offset: Vec2::zero(),
            };
            self.bones.len()
        ];

        let mut bone_world: Vec<Mat4<f32>> = vec![Mat4::identity(); self.bones.len()];
        let mut orientation_world: Vec<Quaternion<f32>> =
            vec![Quaternion::identity(); self.bones.len()];

        for (i, bone) in self.bones.iter().enumerate() {
            let delta = pose.deltas.get(i).copied().unwrap_or_default();

            let position = bone.position + delta.position;
            let orientation: Quaternion<f32> = bone.orientation * delta.rotation;
            let orientation = orientation.normalized();
            let stretch = bone.shape_stretch * delta.stretch;

            let visible = delta
                .visible
                .unwrap_or_else(|| bone.shape.as_ref().is_none_or(Shape::is_visible));

            let parent_world: Mat4<f32> = bone
                .parent
                .map(|p| bone_world[p])
                .unwrap_or(Mat4::identity());
            let parent_rotation: Quaternion<f32> = bone
                .parent
                .map(|p| orientation_world[p])
                .unwrap_or(Quaternion::identity());
            let parent_offset = self.parent_offset(bone);

            orientation_world[i] = (parent_rotation * orientation).normalized();
            bone_world[i] = parent_world
                * Mat4::translation_3d(parent_offset + position)
                * Mat4::from(orientation);

            if !visible {
                // Collapse the geometry onto the origin: degenerate triangles
                // are never rasterised, and no buffers need rebuilding. The
                // homogeneous row is kept at 1 so clip-space W stays valid.
                out[i] = BoneMatrix {
                    bone_mat: collapsed,
                    normals_mat: Mat4::identity(),
                    uv_offset: Vec2::zero(),
                };
                continue;
            }

            let bone_mat =
                bone_world[i] * Mat4::translation_3d(bone.shape_offset) * Mat4::scaling_3d(stretch);
            let normals_mat =
                Mat4::from(orientation_world[i]) * Mat4::scaling_3d(recip_or_zero(stretch));

            out[i] = BoneMatrix {
                bone_mat,
                normals_mat,
                uv_offset: delta.uv_offset,
            };
        }

        out
    }

    fn compute_bind_bounds(&self) -> Bounds {
        let shape_world = self.bind_shape_world();
        let mut bounds = Bounds::EMPTY;

        for (i, bone) in self.bones.iter().enumerate() {
            let Some(shape) = bone.geometry() else {
                continue;
            };
            let transform = shape_world[i];

            for corner in shape_corners(shape) {
                bounds.expand(transform.mul_point(corner));
            }
        }

        if bounds.is_empty() {
            bounds.min = Vec3::broadcast(-0.5);
            bounds.max = Vec3::broadcast(0.5);
        }
        bounds
    }
}

/// Every vertex lands on the origin, but clip-space W stays 1 so the
/// perspective divide never sees a zero.
fn collapsed_matrix() -> Mat4<f32> {
    Mat4::from_col_arrays([
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// The eight corners of a shape's baked, centred geometry (bind stretch
/// included). Quads collapse to a flat rectangle.
pub(crate) fn shape_corners(shape: &Shape) -> Vec<Vec3<f32>> {
    let stretch = shape.stretch_vec();
    let size = shape.size();

    let half = match shape.kind() {
        ShapeType::Quad => {
            let full = match shape.quad_normal() {
                Face6::Front | Face6::Back => Vec3::new(size.x, size.y, 0.0),
                Face6::Right | Face6::Left => Vec3::new(0.0, size.y, size.x),
                Face6::Top | Face6::Bottom => Vec3::new(size.x, 0.0, size.y),
            };
            full * stretch * 0.5
        }
        _ => size * stretch * 0.5,
    };

    let mut out = Vec::with_capacity(8);
    for x in [-half.x, half.x] {
        for y in [-half.y, half.y] {
            for z in [-half.z, half.z] {
                out.push(Vec3::new(x, y, z));
            }
        }
    }
    out
}
