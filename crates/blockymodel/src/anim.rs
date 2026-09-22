//! `.blockyanim` — the animation format, and the sampling rules.
//!
//! Mirrors `docs/blockymodel-format.md` §8–§10. The interpolation behaviour is
//! a faithful port of Blockbench's animator (`js/animations/timeline_animators.js`)
//! as configured by the official Hytale plugin, which sets:
//!
//! * `quaternion_interpolation: true` → rotations always slerp, never spline.
//! * `animation_loop_wrapping: true` → looping clips interpolate back into
//!   their first keyframe during the tail.

use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    json::{JsonQuat, JsonVec2, JsonVec3, ToVek},
    math::{Quaternion, Vec2, Vec3},
};

/// Animation times are stored in frames, always at this rate.
pub const FPS: f32 = 60.0;

/// Time epsilon used by Blockbench when snapping onto a keyframe,
/// `1/1200` seconds — a twentieth of a frame.
const EPSILON_FRAMES: f32 = FPS / 1200.0;

/// A parsed `.blockyanim` document.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockyAnim {
    /// Always `1` in the wild. Kept for forward compatibility.
    #[serde(rename = "formatVersion", default = "default_format_version")]
    pub format_version: u32,
    /// Clip length in **frames** at [`FPS`], not seconds.
    pub duration: f32,
    /// `true` → hold the final pose; `false` → loop back to the start.
    #[serde(rename = "holdLastKeyframe", default)]
    pub hold_last_keyframe: bool,
    /// Tracks keyed by **bone name**. Names may have no matching node, and a
    /// single name may match several nodes; both are normal.
    #[serde(rename = "nodeAnimations", default)]
    pub node_animations: HashMap<String, NodeTracks>,
}

fn default_format_version() -> u32 {
    1
}

impl BlockyAnim {
    /// Clip length in seconds.
    pub fn duration_seconds(&self) -> f32 {
        self.duration / FPS
    }

    /// Whether the tail of the clip interpolates back into its first keyframe.
    ///
    /// This is `animation_loop_wrapping` from the format definition combined
    /// with the clip's own loop mode.
    pub fn wraps(&self) -> bool {
        !self.hold_last_keyframe
    }

    /// Sorts every track by time. Called on load; sampling assumes sorted keys.
    pub fn prepare(&mut self) {
        for tracks in self.node_animations.values_mut() {
            tracks.sort();
        }
    }

    /// Samples one bone's tracks at `frame` (in `0..duration`).
    ///
    /// Returns `None` when the clip has no track for that name.
    pub fn sample_node(&self, name: &str, frame: f32) -> Option<ChannelDelta> {
        let tracks = self.node_animations.get(name)?;
        let mut delta = ChannelDelta::default();
        tracks.sample(frame, self.duration, self.wraps(), &mut delta);
        Some(delta)
    }
}

/// The five keyframe tracks for one bone. Any of them may be missing or empty.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NodeTracks {
    #[serde(default)]
    pub position: Vec<PositionKey>,
    #[serde(default)]
    pub orientation: Vec<OrientationKey>,
    #[serde(default)]
    pub shape_stretch: Vec<StretchKey>,
    #[serde(default)]
    pub shape_visible: Vec<VisibleKey>,
    #[serde(default)]
    pub shape_uv_offset: Vec<UvOffsetKey>,
}

impl NodeTracks {
    /// True when every track is empty.
    pub fn is_empty(&self) -> bool {
        self.position.is_empty()
            && self.orientation.is_empty()
            && self.shape_stretch.is_empty()
            && self.shape_visible.is_empty()
            && self.shape_uv_offset.is_empty()
    }

    fn sort(&mut self) {
        self.position.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.orientation.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.shape_stretch.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.shape_visible.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.shape_uv_offset
            .sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    /// Samples all five channels into `out`, leaving untouched channels alone.
    pub fn sample(&self, frame: f32, duration: f32, wrap: bool, out: &mut ChannelDelta) {
        if let Some(v) = sample_spline(self.position.as_slice(), frame, duration, wrap) {
            out.position = v;
        }
        if let Some(v) = sample_spline(self.shape_stretch.as_slice(), frame, duration, wrap) {
            out.stretch = v;
        }

        // `shapeUvOffset` is stored with a negated Y (Blockbench flips it on
        // import and back on export), so undo that here to get the pixel
        // offset that actually lands on the texture.
        if let Some(v) = sample_spline(self.shape_uv_offset.as_slice(), frame, duration, wrap) {
            out.uv_offset = Vec2::new(v.x, -v.y);
        }

        // Visibility is a boolean: no interpolation, just "the last key at or
        // before now". Falling back to the first key matches Blockbench, which
        // picks the *after* keyframe when there is nothing before.
        if let Some(k) =
            last_key_at_or_before(&self.shape_visible, frame).or_else(|| self.shape_visible.first())
        {
            out.visible = Some(k.delta);
        }

        // Rotation takes a completely different path: slerp, never a spline.
        if let Some(q) = sample_rotation(self.orientation.as_slice(), frame, duration, wrap) {
            out.rotation = q;
        }
    }
}

/// The per-bone result of sampling an animation.
///
/// These are **deltas relative to the bind pose**, except `visible`, which is
/// absolute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelDelta {
    /// Added to the bone's bind position.
    pub position: Vec3<f32>,
    /// Multiplied on the right of the bone's bind rotation: `bind * delta`.
    pub rotation: Quaternion<f32>,
    /// Multiplied component-wise with the shape's bind stretch.
    pub stretch: Vec3<f32>,
    /// Absolute visibility, or `None` when the clip has no visibility track
    /// and the shape's own `visible` flag should be used instead.
    pub visible: Option<bool>,
    /// Pixel offset applied to the face's UV rectangle.
    pub uv_offset: Vec2<f32>,
}

impl Default for ChannelDelta {
    fn default() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Quaternion::identity(),
            stretch: Vec3::one(),
            visible: None,
            uv_offset: Vec2::zero(),
        }
    }
}

/// `interpolationType`. Missing means `linear`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Interpolation {
    #[default]
    Linear,
    Smooth,
    /// Not produced by the Hytale plugin, but Blockbench understands it.
    #[serde(other)]
    Step,
}

/// A positional keyframe. `delta` is added to the bind position.
#[derive(Debug, Clone, Deserialize)]
pub struct PositionKey {
    pub time: f32,
    pub delta: JsonVec3,
    #[serde(rename = "interpolationType", default)]
    pub interpolation: Interpolation,
}

/// A rotational keyframe. `delta` is applied as `bind * delta`.
#[derive(Debug, Clone, Deserialize)]
pub struct OrientationKey {
    pub time: f32,
    pub delta: JsonQuat,
    #[serde(rename = "interpolationType", default)]
    pub interpolation: Interpolation,
}

/// A scale keyframe. `delta` is a **multiplier**, not an additive term.
#[derive(Debug, Clone, Deserialize)]
pub struct StretchKey {
    pub time: f32,
    pub delta: JsonVec3,
    #[serde(rename = "interpolationType", default)]
    pub interpolation: Interpolation,
}

/// A visibility keyframe. Note that `delta` is a bare boolean, not an object.
#[derive(Debug, Clone, Deserialize)]
pub struct VisibleKey {
    pub time: f32,
    pub delta: bool,
    #[serde(rename = "interpolationType", default)]
    pub interpolation: Interpolation,
}

/// A UV-offset keyframe, in texture pixels.
#[derive(Debug, Clone, Deserialize)]
pub struct UvOffsetKey {
    pub time: f32,
    pub delta: JsonVec2,
    #[serde(rename = "interpolationType", default)]
    pub interpolation: Interpolation,
}

// ---------------------------------------------------------------------------
// Generic sampling machinery
// ---------------------------------------------------------------------------

/// A value that can be interpolated.
pub trait ChannelValue: Copy {
    fn lerp(self, other: Self, t: f32) -> Self;
    /// Uniform Catmull-Rom, matching THREE.js `SplineCurve.getPoint`.
    fn catmull_rom(p0: Self, p1: Self, p2: Self, p3: Self, t: f32) -> Self;
}

impl ChannelValue for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
    fn catmull_rom(p0: Self, p1: Self, p2: Self, p3: Self, t: f32) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * ((2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }
}

impl ChannelValue for Vec2<f32> {
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec2::new(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
    fn catmull_rom(p0: Self, p1: Self, p2: Self, p3: Self, t: f32) -> Self {
        Vec2::new(
            f32::catmull_rom(p0.x, p1.x, p2.x, p3.x, t),
            f32::catmull_rom(p0.y, p1.y, p2.y, p3.y, t),
        )
    }
}

impl ChannelValue for Vec3<f32> {
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec3::new(
            self.x.lerp(other.x, t),
            self.y.lerp(other.y, t),
            self.z.lerp(other.z, t),
        )
    }
    fn catmull_rom(p0: Self, p1: Self, p2: Self, p3: Self, t: f32) -> Self {
        Vec3::new(
            f32::catmull_rom(p0.x, p1.x, p2.x, p3.x, t),
            f32::catmull_rom(p0.y, p1.y, p2.y, p3.y, t),
            f32::catmull_rom(p0.z, p1.z, p2.z, p3.z, t),
        )
    }
}

/// Just the timing of a keyframe track. Rotation tracks need only this much,
/// which is why it is separate from [`KeyTrack`].
pub trait KeyTimes {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn time(&self, i: usize) -> f32;
}

/// A keyframe track, viewed uniformly so the sampler needs no conversions and
/// no per-frame allocations.
pub trait KeyTrack: KeyTimes {
    type Value: ChannelValue;
    fn interpolation(&self, i: usize) -> Interpolation;
    fn value(&self, i: usize) -> Self::Value;
}

macro_rules! impl_key_times {
    ($ty:ty) => {
        impl KeyTimes for [$ty] {
            fn len(&self) -> usize {
                <[$ty]>::len(self)
            }
            fn time(&self, i: usize) -> f32 {
                self[i].time
            }
        }
    };
}

macro_rules! impl_key_track {
    ($ty:ty, $json:ty, $value:ty) => {
        impl KeyTrack for [$ty] {
            type Value = $value;

            fn interpolation(&self, i: usize) -> Interpolation {
                self[i].interpolation
            }
            fn value(&self, i: usize) -> $value {
                <$json as ToVek>::to_vek(self[i].delta)
            }
        }
    };
}

impl_key_times!(PositionKey);
impl_key_times!(OrientationKey);
impl_key_times!(StretchKey);
impl_key_times!(UvOffsetKey);
impl_key_times!(VisibleKey);

impl_key_track!(PositionKey, JsonVec3, Vec3<f32>);
impl_key_track!(StretchKey, JsonVec3, Vec3<f32>);
impl_key_track!(UvOffsetKey, JsonVec2, Vec2<f32>);

/// Blockbench's weighted cubic bezier easing, used on rotations only.
///
/// Ported from the Hytale plugin's `src/animations.ts`. It is close to
/// smoothstep but not identical — do not substitute `t*t*(3-2t)`.
pub fn weighted_cubic_bezier(t: f32) -> f32 {
    const P: [f32; 4] = [0.0, 0.05, 0.95, 1.0];
    const W: [f32; 4] = [2.0, 1.0, 2.0, 1.0];

    let mt = 1.0 - t;
    let b = [mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t];

    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..4 {
        num += b[i] * W[i] * P[i];
        den += b[i] * W[i];
    }
    num / den
}

/// The keyframes bracketing a moment in time, plus their (possibly
/// wrap-shifted) times.
///
/// Either side may be absent: before the first keyframe only `after` exists,
/// and past the last one (with wrapping off) only `before` does.
#[derive(Debug, Clone, Copy, Default)]
struct Bracket {
    before: Option<(usize, f32)>,
    after: Option<(usize, f32)>,
}

/// Locates the keyframes surrounding `frame`.
///
/// With `wrap` enabled (looping clips), a frame past the last keyframe
/// interpolates towards a virtual copy of the first keyframe placed one clip
/// length later — and symmetrically for frames before the first keyframe.
fn bracket<K: KeyTimes + ?Sized>(keys: &K, frame: f32, duration: f32, wrap: bool) -> Bracket {
    if keys.is_empty() {
        return Bracket::default();
    }

    let mut before: Option<usize> = None;
    let mut after: Option<usize> = None;
    for i in 0..keys.len() {
        let t = keys.time(i);
        if t < frame {
            if before.is_none_or(|b| t > keys.time(b)) {
                before = Some(i);
            }
        } else if after.is_none_or(|a| t < keys.time(a)) {
            after = Some(i);
        }
    }

    let mut before_time = before.map(|i| keys.time(i)).unwrap_or(0.0);
    let mut after_time = after.map(|i| keys.time(i)).unwrap_or(0.0);

    if wrap && keys.len() >= 2 {
        if before.is_none() {
            let last = argmax(keys);
            before = Some(last);
            before_time = keys.time(last) - duration;
        }
        if after.is_none() {
            let first = argmin(keys);
            after = Some(first);
            after_time = keys.time(first) + duration;
        }
    }

    Bracket {
        before: before.map(|i| (i, before_time)),
        after: after.map(|i| (i, after_time)),
    }
}

fn argmax<K: KeyTimes + ?Sized>(keys: &K) -> usize {
    let mut best = 0;
    for i in 1..keys.len() {
        if keys.time(i) > keys.time(best) {
            best = i;
        }
    }
    best
}

fn argmin<K: KeyTimes + ?Sized>(keys: &K) -> usize {
    let mut best = 0;
    for i in 1..keys.len() {
        if keys.time(i) < keys.time(best) {
            best = i;
        }
    }
    best
}

/// The neighbouring keyframe before `i`, wrapping for looping clips.
fn neighbour_before<K: KeyTimes + ?Sized>(keys: &K, i: usize, wrap: bool) -> usize {
    if i > 0 {
        return i - 1;
    }
    if wrap && keys.len() >= 3 {
        // THREE.js/Blockbench use `sorted.at(-2)`: the second-to-last keyframe.
        return keys.len() - 2;
    }
    i
}

/// The neighbouring keyframe after `i`, wrapping for looping clips.
fn neighbour_after<K: KeyTimes + ?Sized>(keys: &K, i: usize, wrap: bool) -> usize {
    if i + 1 < keys.len() {
        return i + 1;
    }
    if wrap && keys.len() >= 3 {
        // THREE.js/Blockbench use `sorted[1]`: the second keyframe.
        return 1;
    }
    i
}

/// Samples a position/scale/uv track: linear for `linear`, Catmull-Rom as soon
/// as either bracketing keyframe is `smooth`.
fn sample_spline<K: KeyTrack + ?Sized>(
    keys: &K,
    frame: f32,
    duration: f32,
    wrap: bool,
) -> Option<K::Value> {
    if keys.is_empty() {
        return None;
    }

    let b = bracket(keys, frame, duration, wrap);

    // Only one side exists: hold it. This is Blockbench's `before && !after`
    // rule, and it is what makes `holdLastKeyframe` clips stay on the last
    // pose instead of snapping back to the first keyframe.
    let (Some((before_index, before_time)), Some((after_index, after_time))) = (b.before, b.after)
    else {
        return b.before.or(b.after).map(|(i, _)| keys.value(i));
    };

    if (frame - before_time).abs() <= EPSILON_FRAMES {
        return Some(keys.value(before_index));
    }
    if (after_time - frame).abs() <= EPSILON_FRAMES {
        return Some(keys.value(after_index));
    }

    let span = after_time - before_time;
    let alpha = if span.abs() < f32::EPSILON {
        0.0
    } else {
        ((frame - before_time) / span).clamp(0.0, 1.0)
    };

    let before_interp = keys.interpolation(before_index);
    let after_interp = keys.interpolation(after_index);

    if before_interp == Interpolation::Linear
        && matches!(after_interp, Interpolation::Linear | Interpolation::Step)
    {
        return Some(
            keys.value(before_index)
                .lerp(keys.value(after_index), alpha),
        );
    }

    if before_interp == Interpolation::Smooth || after_interp == Interpolation::Smooth {
        let p0 = neighbour_before(keys, before_index, wrap);
        let p3 = neighbour_after(keys, after_index, wrap);
        return Some(K::Value::catmull_rom(
            keys.value(p0),
            keys.value(before_index),
            keys.value(after_index),
            keys.value(p3),
            alpha,
        ));
    }

    Some(
        keys.value(before_index)
            .lerp(keys.value(after_index), alpha),
    )
}

/// Rotation path: always slerp, with Blockbench's bezier easing applied when
/// both bracketing keyframes are `smooth`.
///
/// This is what `quaternion_interpolation: true` buys us in the Hytale format
/// definition. Neighbouring keyframes are never consulted.
fn sample_rotation(
    keys: &[OrientationKey],
    frame: f32,
    duration: f32,
    wrap: bool,
) -> Option<Quaternion<f32>> {
    if keys.is_empty() {
        return None;
    }

    let b = bracket(keys, frame, duration, wrap);

    let (Some((before_index, before_time)), Some((after_index, after_time))) = (b.before, b.after)
    else {
        return b.before.or(b.after).map(|(i, _)| keys[i].delta.to_quat());
    };

    if (frame - before_time).abs() <= EPSILON_FRAMES {
        return Some(keys[before_index].delta.to_quat());
    }
    if (after_time - frame).abs() <= EPSILON_FRAMES {
        return Some(keys[after_index].delta.to_quat());
    }

    let span = after_time - before_time;
    let mut alpha = if span.abs() < f32::EPSILON {
        0.0
    } else {
        ((frame - before_time) / span).clamp(0.0, 1.0)
    };

    if keys[before_index].interpolation == Interpolation::Smooth
        && keys[after_index].interpolation == Interpolation::Smooth
    {
        alpha = weighted_cubic_bezier(alpha);
    }

    Some(Quaternion::slerp(
        keys[before_index].delta.to_quat(),
        keys[after_index].delta.to_quat(),
        alpha,
    ))
}

/// The last visibility key at or before `frame`.
fn last_key_at_or_before(keys: &[VisibleKey], frame: f32) -> Option<&VisibleKey> {
    let mut best: Option<&VisibleKey> = None;
    for k in keys {
        if k.time <= frame && best.is_none_or(|b| k.time > b.time) {
            best = Some(k);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_is_monotonic_and_pinned() {
        assert!((weighted_cubic_bezier(0.0) - 0.0).abs() < 1e-6);
        assert!((weighted_cubic_bezier(1.0) - 1.0).abs() < 1e-6);
        let mid = weighted_cubic_bezier(0.5);
        assert!(mid > 0.25 && mid < 0.75, "mid = {mid}");
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = weighted_cubic_bezier(i as f32 / 100.0);
            assert!(v >= prev - 1e-6);
            prev = v;
        }
    }

    #[test]
    fn catmull_rom_without_neighbours_is_symmetric_not_linear() {
        // THREE.js `SplineCurve` duplicates the control points when there is
        // no neighbour, which makes the single segment a shallow curve rather
        // than a straight line. It is symmetric about the midpoint, and that
        // is what Blockbench produces too.
        let v = f32::catmull_rom(1.0, 1.0, 3.0, 3.0, 0.5);
        assert!((v - 2.0).abs() < 1e-5, "midpoint got {v}");

        let quarter = f32::catmull_rom(1.0, 1.0, 3.0, 3.0, 0.25);
        assert!((quarter - 1.40625).abs() < 1e-5, "got {quarter}");
        let three_quarter = f32::catmull_rom(1.0, 1.0, 3.0, 3.0, 0.75);
        assert!(
            (three_quarter - 2.59375).abs() < 1e-5,
            "got {three_quarter}"
        );
    }

    fn position_key(time: f32, x: f32) -> PositionKey {
        PositionKey {
            time,
            delta: JsonVec3 { x, y: 0.0, z: 0.0 },
            interpolation: Interpolation::Linear,
        }
    }

    #[test]
    fn linear_track_interpolates() {
        let keys = [position_key(0.0, 0.0), position_key(10.0, 10.0)];
        let v = sample_spline(keys.as_slice(), 5.0, 20.0, true).unwrap();
        assert!((v.x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn loop_wrapping_interpolates_into_the_first_keyframe() {
        // Last key at 60, clip length 80: frame 70 is halfway back to the
        // first keyframe (placed at 80).
        let keys = [position_key(0.0, 0.0), position_key(60.0, 60.0)];

        let v = sample_spline(keys.as_slice(), 70.0, 80.0, true).unwrap();
        assert!((v.x - 30.0).abs() < 1e-4, "got {}", v.x);

        // Without wrapping the last value is held.
        let v = sample_spline(keys.as_slice(), 70.0, 80.0, false).unwrap();
        assert!((v.x - 60.0).abs() < 1e-4, "got {}", v.x);
    }

    #[test]
    fn a_lone_keyframe_is_constant() {
        let keys = [StretchKey {
            time: 12.0,
            delta: JsonVec3 {
                x: 2.0,
                y: 2.0,
                z: 2.0,
            },
            interpolation: Interpolation::Smooth,
        }];

        for frame in [0.0, 12.0, 40.0] {
            let v = sample_spline(keys.as_slice(), frame, 50.0, true).unwrap();
            assert_eq!(v.x, 2.0, "at frame {frame}");
            assert_eq!(v.y, 2.0, "at frame {frame}");
            assert_eq!(v.z, 2.0, "at frame {frame}");
        }
    }

    #[test]
    fn frames_before_the_first_keyframe_hold_it() {
        let keys = [position_key(10.0, 5.0), position_key(20.0, 15.0)];
        let v = sample_spline(keys.as_slice(), 2.0, 40.0, false).unwrap();
        assert!((v.x - 5.0).abs() < 1e-5, "got {}", v.x);
    }
}
