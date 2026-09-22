//! Vertex and fragment shaders.
//!
//! Vertices arrive in *bind-shape space* (centred, with the bind stretch baked
//! in). `bones[in.bone]` supplies everything else, which is what lets the
//! static vertex buffer survive any animation.
//!
//! `Bone` mirrors `vrage_render::pipelines::figure::BoneData` — a bone matrix
//! and a normal matrix — plus the per-shape UV offset this renderer supports.

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Bone {
    // bone_world * T(shape.offset) * S(animated stretch)
    bone_mat: mat4x4<f32>,
    // inverse-transpose of `bone_mat`'s linear part
    normals_mat: mat4x4<f32>,
    // Per-shape UV offset in .xy, already normalized by the host. A vec4 (not
    // a vec2) because the uniform address space pads struct members onto
    // 16-byte boundaries, which would otherwise change the array stride.
    uv_offset: vec4<f32>,
};

// The format caps a model at 255 nodes, and a uniform array (unlike a storage
// one) has to be sized at compile time.
const MAX_BONES: u32 = 255u;

@group(1) @binding(0)
var<uniform> bones: array<Bone, MAX_BONES>;

@group(1) @binding(1)
var model_texture: texture_2d<f32>;

@group(1) @binding(2)
var model_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) bone: u32,
    @location(4) unlit: f32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) unlit: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let b = bones[in.bone];
    let world = b.bone_mat * vec4<f32>(in.position, 1.0);

    var out: VsOut;
    out.clip = camera.view_proj * world;
    out.world = world.xyz;
    out.normal = (b.normals_mat * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv = in.uv + b.uv_offset.xy;
    out.unlit = in.unlit;
    return out;
}

const LIGHT_DIR = vec3<f32>(0.35, 0.85, 0.4);

@fragment
fn fs_main(in: VsOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let texel = textureSample(model_texture, model_sampler, in.uv);

    // Hytale textures are pixel art with hard alpha cutouts; alpha-testing
    // keeps the pass opaque and therefore order-independent.
    if (texel.a < 0.5) {
        discard;
    }

    var n = normalize(in.normal);
    if (!front_facing) {
        n = -n;
    }

    let lambert = max(dot(n, normalize(LIGHT_DIR)), 0.0);
    let lit = 0.55 + 0.45 * lambert;
    let shade = mix(lit, 1.0, in.unlit);

    return vec4<f32>(texel.rgb * shade, 1.0);
}
