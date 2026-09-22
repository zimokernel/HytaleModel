//! End-to-end checks against the real `Pets/Dog` asset.
//!
//! These pin the behaviour that is easy to get subtly wrong: the bind-pose
//! transform chain, UV rectangles, negative-stretch winding and the animation
//! sampling rules.

use std::path::{Path, PathBuf};

use blockymodel::{
    normalize_or_zero, BlockyAnim, BlockyModel, Face6, MeshBuilder, Quaternion, ShapeType,
    Skeleton, Vec2, Vec3, FPS,
};

fn asset(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn dog_model() -> BlockyModel {
    BlockyModel::from_path(asset("Pets/Dog/Models/Model.blockymodel")).expect("parse model")
}

fn anim_names() -> Vec<&'static str> {
    vec![
        "Alerted",
        "Death",
        "Fall",
        "Hurt",
        "Idle",
        "Jump",
        "Jump_Far",
        "Run",
        "Sleep",
        "Walk",
        "Walk_Backward",
    ]
}

#[test]
fn parses_the_dog_skeleton() {
    let model = dog_model();
    assert_eq!(model.nodes.len(), 1, "one root node");
    assert_eq!(model.nodes[0].name, "Pelvis");
    assert_eq!(model.nodes[0].id, "12", "ids are strings");

    let skeleton = Skeleton::from_model(&model);
    assert_eq!(skeleton.len(), 31);

    let mut boxes = 0;
    let mut quads = 0;
    for bone in &skeleton.bones {
        match bone.shape.as_ref().map(|s| s.kind()) {
            Some(ShapeType::Box) => boxes += 1,
            Some(ShapeType::Quad) => quads += 1,
            _ => {}
        }
    }
    assert_eq!((boxes, quads), (26, 5));

    // Parents always precede their children, which is what makes the single
    // forward pass in `bone_matrices` valid.
    for (i, bone) in skeleton.bones.iter().enumerate() {
        if let Some(parent) = bone.parent {
            assert!(parent < i, "bone {i} has a later parent {parent}");
        }
    }
}

#[test]
fn bone_names_are_not_unique() {
    let skeleton = Skeleton::from_model(&dog_model());
    // The author of the asset accidentally named R-Ear's child "L-Ear2", so
    // name lookup has to be one-to-many.
    let duplicates = skeleton.by_name.get("L-Ear2").expect("L-Ear2 exists");
    assert_eq!(duplicates.len(), 2);
}

#[test]
fn bind_pose_bounds_match_the_reference_measurement() {
    let skeleton = Skeleton::from_model(&dog_model());
    let b = skeleton.bounds;
    let size = b.size();

    // Independently verified by a separate implementation of the §5 transform
    // chain; if the parent-shape-offset rule regresses, these move visibly.
    assert!((size.x - 43.90).abs() < 0.5, "size.x = {}", size.x);
    assert!((size.y - 88.54).abs() < 0.5, "size.y = {}", size.y);
    assert!((size.z - 94.91).abs() < 0.5, "size.z = {}", size.z);

    // Feet near the bottom, ears near the top.
    assert!(b.min.y > -5.0 && b.min.y < 1.0, "min.y = {}", b.min.y);
    assert!(b.max.y > 80.0 && b.max.y < 96.0, "max.y = {}", b.max.y);
}

#[test]
fn the_parent_shape_offset_shifts_child_bones() {
    let skeleton = Skeleton::from_model(&dog_model());

    // Pelvis sits at y = 35 with no shape offset, so its world origin is
    // exactly its position.
    let pelvis = skeleton.by_name["Pelvis"][0];
    let world = skeleton.bind_bone_world();
    let origin = world[pelvis].mul_point(Vec3::zero());
    assert!(
        (origin - Vec3::new(0.0, 35.0, -20.0)).magnitude() < 1e-3,
        "Pelvis landed at {origin:?}"
    );

    // Feet must be far below the pelvis: a regression that drops the parent
    // offset collapses the whole skeleton onto the root.
    let foot = skeleton.by_name["L-Foot"][0];
    let foot_origin = world[foot].mul_point(Vec3::zero());
    assert!(
        foot_origin.y < 15.0,
        "L-Foot landed at {foot_origin:?}, which is not below the body"
    );
}

#[test]
fn uv_rectangles_are_inside_the_texture() {
    let skeleton = Skeleton::from_model(&dog_model());
    let builder = MeshBuilder::new(256.0, 128.0);
    let faces = builder.build_faces(&skeleton);

    assert_eq!(faces.len(), 148, "13 faces have no textureLayout");

    for face in &faces {
        for uv in face.uvs {
            assert!(
                (-0.01..=1.01).contains(&uv.x) && (-0.01..=1.01).contains(&uv.y),
                "UV {uv:?} escaped the texture on bone {}",
                face.bone
            );
        }
        // Quads report the plane they live on; boxes report a real cube face.
        assert!(matches!(
            face.source_face,
            Face6::Front | Face6::Back | Face6::Left | Face6::Right | Face6::Top | Face6::Bottom
        ));
    }
}

/// `R-Eye` is a quad with `size = (11, 10)`, offset `(11, 71)`, no mirror and
/// no rotation. Its UVs must land exactly on that 11x10 pixel rectangle.
#[test]
fn quad_uvs_land_on_the_authored_rectangle() {
    let skeleton = Skeleton::from_model(&dog_model());
    let builder = MeshBuilder::new(256.0, 128.0);
    let faces = builder.build_faces(&skeleton);

    let eye = skeleton.by_name["R-Eye"][0] as u32;
    let face = faces
        .iter()
        .find(|f| f.bone == eye)
        .expect("R-Eye generates a face");

    // The UV inset pulls the rectangle in by 1/8 px on every side.
    let u0 = (11.0 + 0.125) / 256.0;
    let u1 = (22.0 - 0.125) / 256.0;
    let v0 = (71.0 + 0.125) / 128.0;
    let v1 = (81.0 - 0.125) / 128.0;

    let expected = [
        Vec2::new(u0, v0), // TL
        Vec2::new(u1, v0), // TR
        Vec2::new(u0, v1), // BL
        Vec2::new(u1, v1), // BR
    ];
    for (got, want) in face.uvs.iter().zip(expected.iter()) {
        assert!(
            (got.x - want.x).abs() < 1e-5 && (got.y - want.y).abs() < 1e-5,
            "got {got:?}, want {want:?}"
        );
    }
}

#[test]
fn mesh_winding_matches_the_declared_normals() {
    let skeleton = Skeleton::from_model(&dog_model());
    let mesh = MeshBuilder::new(256.0, 128.0).build(&skeleton);

    assert_eq!(mesh.vertices.len(), 592);
    assert_eq!(mesh.triangle_count(), 296);

    // Every quad in the Dog is double-sided; every box is not.
    assert!(!mesh.indices_double.is_empty());
    assert!(!mesh.indices_single.is_empty());

    let mut checked = 0;
    for face in MeshBuilder::new(256.0, 128.0).build_faces(&skeleton) {
        // Cross product of the emitted winding must point along the normal.
        let [tl, tr, bl, _br] = face.positions;
        let (a, b, c) = if face.winding_flipped {
            (tl, tr, bl)
        } else {
            (tl, bl, tr)
        };
        let geometric = normalize_or_zero((b - a).cross(c - a));
        let dot = geometric.dot(face.normal);
        assert!(
            dot > 0.9,
            "bone {} face {:?}: winding normal {geometric:?} vs declared {:?}",
            face.bone,
            face.source_face,
            face.normal
        );
        checked += 1;
    }
    assert_eq!(checked, 148);
}

#[test]
fn loads_every_animation_clip() {
    let dir = asset("Pets/Dog/Animations");
    let mut found = Vec::new();
    collect(&dir, &mut found);
    assert_eq!(found.len(), 15, "15 .blockyanim files");

    for name in anim_names() {
        let path = dir.join(format!("{name}.blockyanim"));
        let anim = BlockyAnim::from_path(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(anim.format_version, 1, "{name}");
        assert!(anim.duration > 0.0, "{name}");
    }
}

#[test]
fn death_holds_and_the_rest_loop() {
    let dir = asset("Pets/Dog/Animations");
    let death = BlockyAnim::from_path(dir.join("Death.blockyanim")).unwrap();
    assert!(death.hold_last_keyframe);
    assert!(!death.wraps());

    let idle = BlockyAnim::from_path(dir.join("Idle.blockyanim")).unwrap();
    assert!(!idle.hold_last_keyframe);
    assert!(idle.wraps());
    assert_eq!(idle.duration, 80.0);
    assert!((idle.duration_seconds() - 80.0 / FPS).abs() < 1e-6);
}

#[test]
fn animations_reference_bones_the_model_lacks() {
    let model = dog_model();
    let skeleton = Skeleton::from_model(&model);
    let dir = asset("Pets/Dog/Animations");

    let idle = BlockyAnim::from_path(dir.join("Idle.blockyanim")).unwrap();
    let unknown = skeleton.unknown_animated_bones(&idle);
    assert!(
        unknown.contains(&"Collar".to_string()),
        "expected Collar to be unmatched, got {unknown:?}"
    );

    // Sampling must not panic or fail just because a name is unmatched.
    let mut pose = skeleton.bind_pose();
    skeleton.apply(&mut pose, Some(&idle), 0.4);
}

#[test]
fn sampling_moves_the_pose_away_from_bind() {
    let skeleton = Skeleton::from_model(&dog_model());
    let idle = BlockyAnim::from_path(asset("Pets/Dog/Animations/Idle.blockyanim")).unwrap();

    let bind = skeleton.bind_pose();
    let bind_matrices = skeleton.bone_matrices(&bind);

    let mut pose = skeleton.bind_pose();
    skeleton.apply(&mut pose, Some(&idle), 20.0 / FPS);
    let posed = skeleton.bone_matrices(&pose);

    let moved = bind_matrices
        .iter()
        .zip(posed.iter())
        .filter(|(a, b)| {
            (a.bone_mat.mul_point(Vec3::zero()) - b.bone_mat.mul_point(Vec3::zero())).magnitude()
                > 1e-4
        })
        .count();
    assert!(moved > 0, "the Idle clip moved nothing");

    // A looping clip must be seamless: t == 0 and t == duration agree.
    let mut at_zero = skeleton.bind_pose();
    skeleton.apply(&mut at_zero, Some(&idle), 0.0);
    let mut at_end = skeleton.bind_pose();
    skeleton.apply(&mut at_end, Some(&idle), idle.duration_seconds());

    let head = skeleton.by_name["Head"][0];
    let a = skeleton.bone_matrices(&at_zero)[head].bone_mat;
    let b = skeleton.bone_matrices(&at_end)[head].bone_mat;
    let delta = (a.mul_point(Vec3::zero()) - b.mul_point(Vec3::zero())).magnitude();
    assert!(
        delta < 1e-3,
        "loop seam: Head moved {delta} between 0 and duration"
    );
}

#[test]
fn every_clip_samples_without_panicking() {
    let skeleton = Skeleton::from_model(&dog_model());
    let dir = asset("Pets/Dog/Animations");
    let mut paths = Vec::new();
    collect(&dir, &mut paths);

    for path in paths {
        let anim = BlockyAnim::from_path(&path).expect("parse clip");
        let mut pose = skeleton.bind_pose();

        // Sample across the clip plus a little past the end.
        for step in 0..=20 {
            let t = anim.duration_seconds() * (step as f32 / 20.0) * 1.2;
            skeleton.apply(&mut pose, Some(&anim), t);
            for m in skeleton.bone_matrices(&pose) {
                for column in m.bone_mat.into_col_arrays() {
                    for v in column {
                        assert!(v.is_finite(), "non-finite matrix at {t}s in {path:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn pose_state_is_reset_between_clips() {
    let skeleton = Skeleton::from_model(&dog_model());
    let dir = asset("Pets/Dog/Animations");

    let run = BlockyAnim::from_path(dir.join("Run.blockyanim")).unwrap();

    let mut pose = skeleton.bind_pose();
    skeleton.apply(&mut pose, Some(&run), 0.15);
    assert!(
        pose.deltas
            .iter()
            .any(|d| d.rotation != Quaternion::identity()),
        "Run should rotate at least one bone"
    );

    // Applying `None` must clear everything, or a clip switch leaves residue.
    skeleton.apply(&mut pose, None, 0.0);
    for delta in &pose.deltas {
        assert_eq!(delta.rotation, Quaternion::identity());
        assert_eq!(delta.position, Vec3::zero());
        assert_eq!(delta.stretch, Vec3::one());
        assert_eq!(delta.visible, None);
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .expect("read animations dir")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "blockyanim") {
            out.push(path);
        }
    }
}
