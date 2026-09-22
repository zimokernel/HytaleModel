//! `player` — an interactive wgpu player for Hytale `.blockymodel` /
//! `.blockyanim` assets.
//!
//! ```text
//! player --model Pets/Dog/Models/Model.blockymodel
//! player --info
//! player --list
//! ```

mod app;
mod assets;
mod camera;
mod renderer;

use std::path::PathBuf;

use anyhow::{bail, Result};

const DEFAULT_MODEL: &str = "Pets/Dog/Models/Model.blockymodel";

#[derive(Debug, Default)]
struct Options {
    model: Option<PathBuf>,
    texture: Option<PathBuf>,
    animations: Option<PathBuf>,
    parts: Vec<PathBuf>,
    clip: Option<String>,
    list: bool,
    info: bool,
    help: bool,
}

fn parse_args() -> Result<Options> {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--model" | "-m" => {
                options.model = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--model needs a path"))?,
                ));
            }
            "--texture" | "-t" => {
                options.texture = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--texture needs a path"))?,
                ));
            }
            "--anims" | "--animations" | "-a" => {
                options.animations = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--anims needs a path"))?,
                ));
            }
            "--anim" | "-c" => {
                options.clip = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--anim needs a name"))?,
                );
            }
            "--part" | "-p" => options
                .parts
                .push(PathBuf::from(args.next().ok_or_else(|| {
                    anyhow::anyhow!("--part needs a .blockymodel path")
                })?)),
            "--list" | "-l" => options.list = true,
            "--info" | "-i" => options.info = true,
            "--help" | "-h" => options.help = true,
            other => bail!("unknown argument `{other}` (try --help)"),
        }
    }

    Ok(options)
}

fn print_help() {
    println!(
        "\
blockyanim player — play Hytale .blockymodel / .blockyanim assets with wgpu

USAGE:
    player [OPTIONS]

OPTIONS:
    -m, --model <PATH>       .blockymodel to load        [default: {DEFAULT_MODEL}]
    -t, --texture <PATH>     texture PNG                  [default: auto-discovered]
    -a, --anims <DIR>        animation root directory     [default: <asset>/Animations]
    -c, --anim <NAME>        clip to start on             [default: first alphabetically]
    -p, --part <PATH>        extra Character attachment model (repeatable)
    -l, --list               list the discovered clips and exit
    -i, --info               print model details and exit
    -h, --help               show this help

CONTROLS:
    left drag      orbit the camera
    mouse wheel    zoom
    space          play / pause
    left / right   previous / next clip
    1 - 9          jump to clip
    up / down      playback speed
    , / .          step one frame back / forward
    r              restart the clip
    esc            quit"
    );
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = parse_args()?;
    if options.help {
        print_help();
        return Ok(());
    }

    let model_path = options
        .model
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MODEL));

    if !model_path.is_file() {
        bail!(
            "model not found: {}\nrun with --help, or pass --model <path.blockymodel>",
            model_path.display()
        );
    }

    let assets = assets::AssetSet::load(
        &model_path,
        options.texture.as_deref(),
        options.animations.as_deref(),
    )?;

    if options.info || options.list {
        print!("{}", assets.describe());
        return Ok(());
    }

    let parts = options
        .parts
        .iter()
        .map(|path| assets::CharacterPart::load(path, None))
        .collect::<Result<Vec<_>>>()?;

    if assets.clips.is_empty() {
        log::warn!("no .blockyanim files found; showing the bind pose only");
    }

    let mut app = app::App::new(assets, parts, options.clip.as_deref());
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}
