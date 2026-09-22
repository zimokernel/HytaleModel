//! The winit application: playback state, input handling and the frame loop.

use std::{sync::Arc, time::Instant};

use anyhow::Result;
use blockymodel::{Mesh, Pose};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{assets::AssetSet, camera::OrbitCamera, renderer::Renderer};

const BACKGROUND: wgpu::Color = wgpu::Color {
    r: 0.055,
    g: 0.060,
    b: 0.070,
    a: 1.0,
};

pub struct App {
    assets: AssetSet,
    mesh: Mesh,
    pose: Pose,
    camera: OrbitCamera,

    clip_index: usize,
    time: f32,
    speed: f32,
    playing: bool,

    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    last_frame: Instant,
    dragging: bool,
    panning: bool,
    last_cursor: Option<(f64, f64)>,
    title_dirty: bool,
}

impl App {
    pub fn new(assets: AssetSet, initial_clip: Option<&str>) -> Self {
        let mesh = assets.mesh_builder().build(&assets.skeleton);
        let pose = assets.skeleton.bind_pose();

        let bounds = assets.skeleton.bounds;
        let camera = OrbitCamera::framing(bounds.center(), bounds.radius());

        let clip_index = initial_clip
            .and_then(|name| {
                assets.clips.iter().position(|c| {
                    c.name.eq_ignore_ascii_case(name) || c.source.eq_ignore_ascii_case(name)
                })
            })
            .unwrap_or(0);

        Self {
            assets,
            mesh,
            pose,
            camera,
            clip_index,
            time: 0.0,
            speed: 1.0,
            playing: true,
            window: None,
            renderer: None,
            last_frame: Instant::now(),
            dragging: false,
            panning: false,
            last_cursor: None,
            title_dirty: true,
        }
    }

    fn select_clip(&mut self, index: usize) {
        if self.assets.clips.is_empty() {
            return;
        }
        self.clip_index = index % self.assets.clips.len();
        self.time = 0.0;
        self.title_dirty = true;
        if let Some(name) = self
            .assets
            .clips
            .get(self.clip_index)
            .map(|c| c.name.clone())
        {
            log::info!("animation: {name}");
        }
    }

    fn step_clip(&mut self, delta: isize) {
        if self.assets.clips.is_empty() {
            return;
        }
        let len = self.assets.clips.len() as isize;
        let next = (self.clip_index as isize + delta).rem_euclid(len);
        self.select_clip(next as usize);
    }

    fn advance(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let Some(clip) = self.assets.clips.get(self.clip_index) else {
            return;
        };
        let duration = clip.anim.duration_seconds();

        if self.playing && duration > 0.0 {
            self.time += dt * self.speed;
            if clip.anim.hold_last_keyframe {
                self.time = self.time.min(duration);
            } else {
                self.time = self.time.rem_euclid(duration);
            }
        }

        let before = self.time;
        self.assets
            .skeleton
            .apply(&mut self.pose, Some(&clip.anim), self.time);
        let _ = before;
        self.title_dirty = true;
    }

    fn update_title(&mut self) {
        if !self.title_dirty {
            return;
        }
        self.title_dirty = false;

        let Some(window) = &self.window else { return };
        let Some(renderer) = &self.renderer else {
            return;
        };

        let stats = renderer.stats;
        let (clip, timing) = match self.assets.clips.get(self.clip_index) {
            Some(clip) => (
                clip.name.clone(),
                format!(
                    "{:.2}/{:.2}s{}{}",
                    self.time,
                    clip.anim.duration_seconds(),
                    if clip.anim.hold_last_keyframe {
                        " hold"
                    } else {
                        " loop"
                    },
                    if self.playing { "" } else { " [paused]" },
                ),
            ),
            None => ("<no animations>".to_string(), "-".to_string()),
        };

        window.set_title(&format!(
            "{clip}  {timing}  x{:.2}  |  {} verts  {} tris  {} bones  |  drag=orbit  right-drag=pan  wheel=zoom  space=play  <-/->=clip  up/down=speed  r=restart  esc=quit",
            self.speed, stats.vertices, stats.triangles, stats.bones
        ));
    }

    fn restart(&mut self) {
        self.time = 0.0;
        self.title_dirty = true;
    }

    fn step_frame(&mut self, frames: f32) {
        self.playing = false;
        self.time += frames / blockymodel::FPS;
        if let Some(clip) = self.assets.clips.get(self.clip_index) {
            let duration = clip.anim.duration_seconds();
            if duration > 0.0 {
                self.time = self.time.rem_euclid(duration);
            }
        }
        self.title_dirty = true;
    }

    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.title_dirty = true;
    }

    fn change_speed(&mut self, delta: f32) {
        self.speed = (self.speed + delta).clamp(0.05, 8.0);
        self.title_dirty = true;
    }

    fn render(&mut self) -> Result<()> {
        let (Some(renderer), Some(clip)) =
            (&mut self.renderer, self.assets.clips.get(self.clip_index))
        else {
            // No clips at all: still draw the bind pose.
            let Some(renderer) = &mut self.renderer else {
                return Ok(());
            };
            let bones = self.assets.skeleton.bone_matrices(&self.pose);
            let aspect = renderer.size().0 as f32 / renderer.size().1.max(1) as f32;
            renderer.update(self.camera.view_proj(aspect), &bones);
            renderer.render(BACKGROUND)?;
            return Ok(());
        };

        let _ = clip;
        let bones = self.assets.skeleton.bone_matrices(&self.pose);
        let aspect = renderer.size().0 as f32 / renderer.size().1.max(1) as f32;
        renderer.update(self.camera.view_proj(aspect), &bones);
        renderer.render(BACKGROUND)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("blockyanim player")
            .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 800.0));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("failed to create a window: {err}");
                event_loop.exit();
                return;
            }
        };

        let renderer = match Renderer::new(
            window.clone(),
            &self.mesh,
            &self.assets.texture_rgba,
            self.assets.texture_size,
            self.assets.skeleton.len(),
        ) {
            Ok(renderer) => renderer,
            Err(err) => {
                log::error!("failed to initialise the renderer: {err:#}");
                event_loop.exit();
                return;
            }
        };

        log::info!(
            "renderer ready: {} ({}x{}, {}x MSAA)",
            format!("{:?}", renderer.surface_format()),
            renderer.size().0,
            renderer.size().1,
            renderer.sample_count()
        );
        log::debug!(
            "window: inner_size={:?} scale_factor={}",
            window.inner_size(),
            window.scale_factor()
        );
        log::debug!(
            "camera: target={:?} eye={:?} distance={:.1}",
            self.camera.target,
            self.camera.eye(),
            self.camera.distance
        );

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.last_frame = Instant::now();
        self.title_dirty = true;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                log::debug!("resized to {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.title_dirty = true;
            }

            WindowEvent::RedrawRequested => {
                if let Err(err) = self.render() {
                    log::error!("render failed: {err:#}");
                    event_loop.exit();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };

                match code {
                    KeyCode::Escape => event_loop.exit(),
                    KeyCode::Space => self.toggle_play(),
                    KeyCode::ArrowRight | KeyCode::BracketRight => self.step_clip(1),
                    KeyCode::ArrowLeft | KeyCode::BracketLeft => self.step_clip(-1),
                    KeyCode::ArrowUp => self.change_speed(0.1),
                    KeyCode::ArrowDown => self.change_speed(-0.1),
                    KeyCode::KeyR => self.restart(),
                    KeyCode::Period => self.step_frame(1.0),
                    KeyCode::Comma => self.step_frame(-1.0),
                    KeyCode::Digit1
                    | KeyCode::Digit2
                    | KeyCode::Digit3
                    | KeyCode::Digit4
                    | KeyCode::Digit5
                    | KeyCode::Digit6
                    | KeyCode::Digit7
                    | KeyCode::Digit8
                    | KeyCode::Digit9 => {
                        let n = match code {
                            KeyCode::Digit1 => 0,
                            KeyCode::Digit2 => 1,
                            KeyCode::Digit3 => 2,
                            KeyCode::Digit4 => 3,
                            KeyCode::Digit5 => 4,
                            KeyCode::Digit6 => 5,
                            KeyCode::Digit7 => 6,
                            KeyCode::Digit8 => 7,
                            _ => 8,
                        };
                        self.select_clip(n);
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        self.dragging = pressed;
                        self.panning = false;
                    }
                    MouseButton::Right => {
                        self.panning = pressed;
                        self.dragging = false;
                    }
                    _ => {}
                }
                if !pressed {
                    self.last_cursor = None;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let current = (position.x, position.y);
                if let Some(previous) = self.last_cursor {
                    let dx = (current.0 - previous.0) as f32;
                    let dy = (current.1 - previous.1) as f32;
                    if self.dragging {
                        self.camera.orbit(dx, dy);
                    } else if self.panning {
                        self.camera.pan(dx, dy);
                    }
                }
                self.last_cursor = Some(current);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 50.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.camera.zoom(amount);
            }

            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.advance();
        self.update_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
