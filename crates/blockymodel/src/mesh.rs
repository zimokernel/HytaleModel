//! Turns a [`Skeleton`]'s shapes into indexed triangles.
//!
//! Vertices are baked into a *shape-local* frame: centred on the shape origin
//! and already scaled by the **bind** stretch. The renderer supplies the rest
//! (`bone_world · T(shape.offset) · S(animated stretch)`) as a per-bone
//! matrix, which is what lets one static vertex buffer survive any animation.

use bytemuck::{Pod, Zeroable};

use crate::{
    math::{normalize_or_zero, Vec2, Vec3},
    model::{Bounds, Face6, Shape, ShapeType, TextureFace},
    skeleton::Skeleton,
};

/// A vertex in bind-shape space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    /// Already scaled by the bind stretch; the bone matrix applies the rest.
    pub position: [f32; 3],
    /// Unit normal, with negative stretch axes already folded in.
    pub normal: [f32; 3],
    /// Normalised texture coordinates, origin top-left.
    pub uv: [f32; 2],
    /// Index into the skeleton's bone array.
    pub bone: u32,
    /// `1.0` = unlit (the texture is used as-is), `0.0` = lit.
    pub unlit: f32,
}

/// One generated quad, before it becomes triangles. Handy for inspection and
/// for tests.
#[derive(Debug, Clone, Copy)]
pub struct Face {
    /// `[TL, TR, BL, BR]`, viewed from outside the shape.
    pub positions: [Vec3<f32>; 4],
    pub normal: Vec3<f32>,
    /// Matches `positions` one-to-one.
    pub uvs: [Vec2<f32>; 4],
    /// The bone that drives this face.
    pub bone: u32,
    /// `true` when the face must survive back-face culling.
    pub double_sided: bool,
    pub unlit: f32,
    /// Which `textureLayout` entry produced it. Quads always report `Front`.
    pub source_face: Face6,
    /// `true` when an odd number of stretch axes are negative, which mirrors
    /// the geometry and therefore reverses the triangle winding.
    pub winding_flipped: bool,
}

/// Indexed triangle data plus the index lists the two pipelines need.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    /// Triangles for opaque, single-sided shapes (`CullMode::Back`).
    pub indices_single: Vec<u32>,
    /// Triangles for `doubleSided` shapes (`CullMode::None`).
    pub indices_double: Vec<u32>,
    /// Bind-pose world bounds, copied from the skeleton.
    pub bounds: Bounds,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        (self.indices_single.len() + self.indices_double.len()) / 3
    }

    /// Both index lists concatenated, for consumers that want one buffer.
    pub fn all_indices(&self) -> Vec<u32> {
        let mut out = self.indices_single.clone();
        out.extend_from_slice(&self.indices_double);
        out
    }
}

/// Builds [`Mesh`]es from a [`Skeleton`].
#[derive(Debug, Clone, Copy)]
pub struct MeshBuilder {
    /// Texture width in pixels; `textureLayout.offset` is in these units.
    pub texture_width: f32,
    /// Texture height in pixels.
    pub texture_height: f32,
    /// UVs are pulled inwards by this many pixels so neighbouring atlas
    /// islands do not bleed under filtering. The reference exporter uses 1/8.
    pub uv_inset_px: f32,
}

impl MeshBuilder {
    /// A builder for a texture of the given pixel size.
    pub fn new(texture_width: f32, texture_height: f32) -> Self {
        Self {
            texture_width,
            texture_height,
            uv_inset_px: 0.125,
        }
    }

    /// Generates every face in the model, in bone order.
    pub fn build_faces(&self, skeleton: &Skeleton) -> Vec<Face> {
        let mut faces = Vec::new();
        for (index, bone) in skeleton.bones.iter().enumerate() {
            let Some(shape) = bone.geometry() else {
                continue;
            };
            self.shape_faces(shape, index as u32, &mut faces);
        }
        faces
    }

    /// Generates the renderable mesh.
    pub fn build(&self, skeleton: &Skeleton) -> Mesh {
        let faces = self.build_faces(skeleton);

        let mut mesh = Mesh {
            vertices: Vec::with_capacity(faces.len() * 4),
            indices_single: Vec::with_capacity(faces.len() * 6),
            indices_double: Vec::new(),
            bounds: skeleton.bounds,
        };

        for face in &faces {
            let target = if face.double_sided {
                &mut mesh.indices_double
            } else {
                &mut mesh.indices_single
            };

            let base = mesh.vertices.len() as u32;
            for corner in 0..4 {
                let p = face.positions[corner];
                let uv = face.uvs[corner];
                mesh.vertices.push(Vertex {
                    position: [p.x, p.y, p.z],
                    normal: [face.normal.x, face.normal.y, face.normal.z],
                    uv: [uv.x, uv.y],
                    bone: face.bone,
                    unlit: face.unlit,
                });
            }

            // `[TL, TR, BL, BR]`, wound counter-clockwise when viewed from
            // outside. Negative stretch mirrors the geometry and therefore
            // reverses the winding, so it has to be flipped back.
            let (a, b, c, d, e, f) = if face.winding_flipped {
                (base, base + 1, base + 2, base + 2, base + 1, base + 3)
            } else {
                (base, base + 2, base + 1, base + 2, base + 3, base + 1)
            };
            target.extend_from_slice(&[a, b, c, d, e, f]);
        }

        mesh
    }

    fn shape_faces(&self, shape: &Shape, bone: u32, out: &mut Vec<Face>) {
        let stretch = shape.stretch_vec();
        let sizes = shape_sizes(shape, stretch);
        let normal_sign = Vec3::new(sign_of(stretch.x), sign_of(stretch.y), sign_of(stretch.z));
        let winding_flipped = negative_axes(stretch) % 2 == 1;
        let unlit = shape.shading_mode().unlit_factor();

        match shape.kind() {
            ShapeType::Box => {
                for face in Face6::ALL {
                    let Some(layout) = shape.layout(face) else {
                        continue;
                    };
                    let half = sizes * 0.5;
                    let (w, h) = face_uv_dims(shape, face);
                    out.push(Face {
                        positions: face_corners(face, half),
                        normal: normalize_or_zero(face.normal() * normal_sign),
                        uvs: self.face_uvs(layout, w, h),
                        bone,
                        double_sided: shape.is_double_sided(),
                        unlit,
                        source_face: face,
                        winding_flipped,
                    });
                }
            }
            ShapeType::Quad => {
                let face = shape.quad_normal();
                // Quads author only `front`, and the reference renderer falls
                // back to it whenever the natural face has no layout.
                let layout = shape.layout(face).or_else(|| shape.layout(Face6::Front));
                let Some(layout) = layout else { return };

                let half = sizes * 0.5;
                let (w, h) = face_uv_dims(shape, face);
                out.push(Face {
                    positions: face_corners(face, half),
                    normal: normalize_or_zero(face.normal() * normal_sign),
                    uvs: self.face_uvs(layout, w, h),
                    bone,
                    double_sided: shape.is_double_sided(),
                    unlit,
                    source_face: face,
                    winding_flipped,
                });
            }
            ShapeType::Empty | ShapeType::Unknown => {}
        }
    }

    /// Maps a `textureLayout` entry onto the four face corners.
    ///
    /// This is a direct port of the UV maths in `docs/blockymodel-format.md`
    /// §7, which in turn comes from the official Blockbench plugin.
    fn face_uvs(&self, layout: &TextureFace, width: f32, height: f32) -> [Vec2<f32>; 4] {
        let offset = layout.offset_vec2();
        let (ox, oy) = (offset.x, offset.y);

        let mut w = width;
        let mut h = height;
        let mut mx = if layout.mirror.x { -1.0 } else { 1.0 };
        let mut my = if layout.mirror.y { -1.0 } else { 1.0 };

        let (mut u0, mut v0, mut u1, mut v1) = match layout.angle_deg() {
            90 => {
                std::mem::swap(&mut w, &mut h);
                std::mem::swap(&mut mx, &mut my);
                mx = -mx;
                (ox, oy + h * my, ox + w * mx, oy)
            }
            180 => {
                mx = -mx;
                my = -my;
                (ox + w * mx, oy + h * my, ox, oy)
            }
            270 => {
                std::mem::swap(&mut w, &mut h);
                std::mem::swap(&mut mx, &mut my);
                my = -my;
                (ox + w * mx, oy, ox, oy + h * my)
            }
            _ => (ox, oy, ox + w * mx, oy + h * my),
        };

        // Pull the rectangle in by a fraction of a texel. The direction
        // depends on which way the rectangle grew.
        let (inset_u, inset_v) = (self.uv_inset_px, self.uv_inset_px);
        if u0 < u1 {
            u0 += inset_u;
            u1 -= inset_u;
        } else {
            u0 -= inset_u;
            u1 += inset_u;
        }
        if v0 < v1 {
            v0 += inset_v;
            v1 -= inset_v;
        } else {
            v0 -= inset_v;
            v1 += inset_v;
        }

        // Corner order matches `face_corners`: TL, TR, BL, BR.
        let mut corners = [
            Vec2::new(u0, v0),
            Vec2::new(u1, v0),
            Vec2::new(u0, v1),
            Vec2::new(u1, v1),
        ];

        // Rotate the corner *assignments*, exactly as Blockbench's
        // `getUVArray` does.
        let mut steps = layout.angle_deg() / 90;
        while steps > 0 {
            let tmp = corners[0];
            corners[0] = corners[2];
            corners[2] = corners[3];
            corners[3] = corners[1];
            corners[1] = tmp;
            steps -= 1;
        }

        // wgpu's texture origin is top-left, same as the pixel offsets, so
        // there is no V flip to apply here.
        let inv_w = 1.0 / self.texture_width.max(1.0);
        let inv_h = 1.0 / self.texture_height.max(1.0);
        [
            Vec2::new(corners[0].x * inv_w, corners[0].y * inv_h),
            Vec2::new(corners[1].x * inv_w, corners[1].y * inv_h),
            Vec2::new(corners[2].x * inv_w, corners[2].y * inv_h),
            Vec2::new(corners[3].x * inv_w, corners[3].y * inv_h),
        ]
    }
}

/// Full extents per axis, already multiplied by the stretch multiplier.
fn shape_sizes(shape: &Shape, stretch: Vec3<f32>) -> Vec3<f32> {
    let size = shape.size();
    match shape.kind() {
        ShapeType::Quad => match shape.quad_normal() {
            // Width along Z, height along Y; the normal axis is flat.
            Face6::Right | Face6::Left => Vec3::new(0.0, size.y * stretch.y, size.x * stretch.z),
            // Width along X, height along Z.
            Face6::Top | Face6::Bottom => Vec3::new(size.x * stretch.x, 0.0, size.y * stretch.z),
            // Width along X, height along Y.
            Face6::Front | Face6::Back => Vec3::new(size.x * stretch.x, size.y * stretch.y, 0.0),
        },
        _ => size * stretch,
    }
}

/// The four corners of a face, ordered `[TL, TR, BL, BR]` as seen from
/// outside the shape.
///
/// This table is the verified one from the reference exporter; getting the
/// corner order wrong silently mirrors or rotates every texture in the model.
pub(crate) fn face_corners(face: Face6, h: Vec3<f32>) -> [Vec3<f32>; 4] {
    let (x, y, z) = (h.x, h.y, h.z);
    match face {
        Face6::Right => [
            Vec3::new(x, y, z),
            Vec3::new(x, y, -z),
            Vec3::new(x, -y, z),
            Vec3::new(x, -y, -z),
        ],
        Face6::Left => [
            Vec3::new(-x, y, -z),
            Vec3::new(-x, y, z),
            Vec3::new(-x, -y, -z),
            Vec3::new(-x, -y, z),
        ],
        Face6::Top => [
            Vec3::new(-x, y, -z),
            Vec3::new(x, y, -z),
            Vec3::new(-x, y, z),
            Vec3::new(x, y, z),
        ],
        Face6::Bottom => [
            Vec3::new(-x, -y, z),
            Vec3::new(x, -y, z),
            Vec3::new(-x, -y, -z),
            Vec3::new(x, -y, -z),
        ],
        Face6::Front => [
            Vec3::new(-x, y, z),
            Vec3::new(x, y, z),
            Vec3::new(-x, -y, z),
            Vec3::new(x, -y, z),
        ],
        Face6::Back => [
            Vec3::new(x, y, -z),
            Vec3::new(-x, y, -z),
            Vec3::new(x, -y, -z),
            Vec3::new(-x, -y, -z),
        ],
    }
}

/// UV rectangle dimensions, in **un-stretched** model units.
fn face_uv_dims(shape: &Shape, face: Face6) -> (f32, f32) {
    let s = shape.size();
    if shape.kind() == ShapeType::Quad {
        return (s.x, s.y);
    }
    match face {
        Face6::Front | Face6::Back => (s.x, s.y),
        Face6::Left | Face6::Right => (s.z, s.y),
        Face6::Top | Face6::Bottom => (s.x, s.z),
    }
}

fn sign_of(v: f32) -> f32 {
    if v < 0.0 {
        -1.0
    } else {
        1.0
    }
}

fn negative_axes(v: Vec3<f32>) -> usize {
    [v.x, v.y, v.z].iter().filter(|c| **c < 0.0).count()
}
