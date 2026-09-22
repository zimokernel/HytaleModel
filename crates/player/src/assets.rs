//! Asset discovery and loading.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use blockymodel::{BlockyAnim, BlockyModel, Mesh, MeshBuilder, Pose, Skeleton};

/// One loadable animation clip.
pub struct Clip {
    /// Display name — the file stem.
    pub name: String,
    /// Path relative to the animation root, for disambiguating duplicates.
    pub source: String,
    pub anim: BlockyAnim,
}

/// Everything the player needs to draw a model.
pub struct AssetSet {
    pub model_path: PathBuf,
    pub texture_path: PathBuf,
    pub texture_size: (u32, u32),
    pub texture_rgba: Vec<u8>,
    pub model: BlockyModel,
    pub skeleton: Skeleton,
    pub clips: Vec<Clip>,
}

/// A renderable model that shares the animation clip selected for the main
/// character, but keeps its own skeleton and texture.
pub struct CharacterPart {
    pub asset: AssetSet,
    pub mesh: Mesh,
    pub pose: Pose,
}

impl CharacterPart {
    pub fn load(model_path: &Path, texture_path: Option<&Path>) -> Result<Self> {
        let asset = AssetSet::load_impl(model_path, texture_path, None, false)?;
        let mesh = asset.mesh_builder().build(&asset.skeleton);
        let pose = asset.skeleton.bind_pose();
        Ok(Self { asset, mesh, pose })
    }
}

impl AssetSet {
    /// Loads a model, its texture and every animation next to it.
    pub fn load(
        model_path: &Path,
        texture_path: Option<&Path>,
        anim_root: Option<&Path>,
    ) -> Result<Self> {
        Self::load_impl(model_path, texture_path, anim_root, true)
    }

    fn load_impl(
        model_path: &Path,
        texture_path: Option<&Path>,
        anim_root: Option<&Path>,
        discover_anims: bool,
    ) -> Result<Self> {
        let model = BlockyModel::from_path(model_path)
            .with_context(|| format!("loading model {}", model_path.display()))?;

        if model.nodes.is_empty() {
            bail!("{} contains no nodes", model_path.display());
        }

        let model_dir = model_path.parent().unwrap_or(Path::new("."));
        let texture_path = match texture_path {
            Some(p) => p.to_path_buf(),
            None => discover_texture(model_dir, model_path)
                .with_context(|| format!("finding a texture for {}", model_path.display()))?,
        };

        let image = image::open(&texture_path)
            .with_context(|| format!("opening texture {}", texture_path.display()))?
            .to_rgba8();
        let texture_size = (image.width(), image.height());
        let texture_rgba = image.into_raw();

        let skeleton = Skeleton::from_model(&model);

        let anim_root = match anim_root {
            Some(p) => Some(p.to_path_buf()),
            None if discover_anims => discover_animations_root(model_dir),
            None => None,
        };

        let mut clips = match &anim_root {
            Some(root) => load_clips(root)?,
            None => Vec::new(),
        };
        clips.sort_by(|a, b| a.source.cmp(&b.source));

        Ok(Self {
            model_path: model_path.to_path_buf(),
            texture_path,
            texture_size,
            texture_rgba,
            model,
            skeleton,
            clips,
        })
    }

    /// A mesh builder configured for this asset's texture.
    pub fn mesh_builder(&self) -> MeshBuilder {
        MeshBuilder::new(self.texture_size.0 as f32, self.texture_size.1 as f32)
    }

    /// A short human-readable summary.
    pub fn describe(&self) -> String {
        use blockymodel::ShapeType;

        let mut boxes = 0;
        let mut quads = 0;
        let mut bones = 0;
        for bone in &self.skeleton.bones {
            match bone.shape.as_ref().map(|s| s.kind()) {
                Some(ShapeType::Box) => boxes += 1,
                Some(ShapeType::Quad) => quads += 1,
                _ => {}
            }
            if bone.shape.is_none() {
                bones += 1;
            }
        }

        let b = self.skeleton.bounds;
        let size = b.size();
        let mut out = String::new();
        out.push_str(&format!("model      : {}\n", self.model_path.display()));
        out.push_str(&format!(
            "texture    : {} ({}x{})\n",
            self.texture_path.display(),
            self.texture_size.0,
            self.texture_size.1
        ));
        out.push_str(&format!("roots      : {}\n", self.model.nodes.len()));
        out.push_str(&format!(
            "nodes      : {} ({} box, {} quad, {} empty)\n",
            self.skeleton.len(),
            boxes,
            quads,
            bones
        ));
        out.push_str(&format!(
            "bounds     : min ({:.2}, {:.2}, {:.2})  max ({:.2}, {:.2}, {:.2})\n",
            b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z
        ));
        out.push_str(&format!(
            "size       : {:.2} x {:.2} x {:.2} units  (≈ {:.2} x {:.2} x {:.2} blocks @64u)\n",
            size.x,
            size.y,
            size.z,
            size.x / 64.0,
            size.y / 64.0,
            size.z / 64.0
        ));

        let mesh = self.mesh_builder().build(&self.skeleton);
        out.push_str(&format!(
            "geometry   : {} vertices, {} triangles\n",
            mesh.vertices.len(),
            mesh.triangle_count()
        ));

        // Bone names repeat in real assets, so list the duplicates explicitly.
        let duplicates: Vec<String> = self
            .skeleton
            .by_name
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(k, _)| k.clone())
            .collect();
        if !duplicates.is_empty() {
            out.push_str(&format!(
                "duplicates : {} (names shared by several bones)\n",
                duplicates.join(", ")
            ));
        }

        out.push_str(&format!("animations : {}\n", self.clips.len()));
        for clip in &self.clips {
            out.push_str(&format!(
                "  {:<18} {:<22} {:>3} frames ({:.2}s)  {}\n",
                clip.name,
                clip.source,
                clip.anim.duration as i32,
                clip.anim.duration_seconds(),
                if clip.anim.hold_last_keyframe {
                    "hold"
                } else {
                    "loop"
                }
            ));
        }

        // Animations routinely drive bones the model does not have.
        for clip in &self.clips {
            let unknown = self.skeleton.unknown_animated_bones(&clip.anim);
            if !unknown.is_empty() {
                out.push_str(&format!(
                    "  [{}] drives absent bones: {}\n",
                    clip.name,
                    unknown.join(", ")
                ));
            }
        }

        out
    }
}

fn discover_texture(model_dir: &Path, model_path: &Path) -> Result<PathBuf> {
    let stem = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Model");

    let candidates = [
        model_dir.join("Texture.png"),
        model_dir.join(format!("{stem}_Texture.png")),
        model_dir.join(format!("{stem}.png")),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let texture_dir = model_dir.join(format!("{stem}_Textures"));
    if let Ok(entries) = std::fs::read_dir(texture_dir) {
        let mut pngs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            })
            .collect();
        pngs.sort();
        if let Some(first) = pngs.into_iter().next() {
            return Ok(first);
        }
    }

    // Fall back to the first PNG in the folder, then next to it.
    if let Ok(entries) = std::fs::read_dir(model_dir) {
        let mut pngs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            })
            .collect();
        pngs.sort();
        if let Some(first) = pngs.into_iter().next() {
            return Ok(first);
        }
    }

    bail!("no PNG found in {}", model_dir.display())
}

fn discover_animations_root(model_dir: &Path) -> Option<PathBuf> {
    // Standard Hytale layout: <asset>/Models/Model.blockymodel and
    // <asset>/Animations/<Clip>.blockyanim.
    let sibling = model_dir.parent()?.join("Animations");
    if sibling.is_dir() {
        return Some(sibling);
    }
    let local = model_dir.join("Animations");
    if local.is_dir() {
        return Some(local);
    }
    None
}

fn load_clips(root: &Path) -> Result<Vec<Clip>> {
    let mut paths = Vec::new();
    collect_animations(root, root, &mut paths)?;
    paths.sort();

    let mut clips = Vec::new();
    for path in paths {
        match BlockyAnim::from_path(&path) {
            Ok(mut anim) => {
                anim.prepare();
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("clip")
                    .to_string();
                let source = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                clips.push(Clip { name, source, anim });
            }
            Err(err) => log::warn!("skipping {}: {err}", path.display()),
        }
    }
    Ok(clips)
}

fn collect_animations(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_animations(root, &path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("blockyanim"))
        {
            out.push(path);
        }
    }
    let _ = root;
    Ok(())
}
