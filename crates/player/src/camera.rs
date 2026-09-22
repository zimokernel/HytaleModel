//! A small orbit camera, in the same `vek` types the rest of the workspace
//! uses.

use blockymodel::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct OrbitCamera {
    pub target: Vec3<f32>,
    /// Radians around +Y. Zero looks at the model from the front (+Z).
    pub yaw: f32,
    /// Radians above the horizon.
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
}

impl OrbitCamera {
    /// Frames a sphere of `radius` comfortably inside the viewport.
    pub fn framing(center: Vec3<f32>, radius: f32) -> Self {
        let fov_y: f32 = 45f32.to_radians();
        let distance = (radius / (fov_y * 0.5).sin()) * 1.15;
        Self {
            target: center,
            yaw: 0.0,
            pitch: 0.18,
            distance: distance.max(0.001),
            fov_y,
        }
    }

    pub fn eye(&self) -> Vec3<f32> {
        let cp = self.pitch.cos();
        self.target
            + Vec3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp) * self.distance
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.5, 1.5);
    }

    pub fn zoom(&mut self, scroll: f32) {
        let factor = (1.0 - scroll * 0.0015).clamp(0.2, 5.0);
        self.distance = (self.distance * factor).max(0.001);
    }

    /// Slides the pivot in the camera's screen plane.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let forward = blockymodel::normalize_or_zero(self.target - self.eye());
        let right = blockymodel::normalize_or_zero(forward.cross(Vec3::new(0.0, 1.0, 0.0)));
        let up = right.cross(forward);
        let scale = self.distance * 0.002;
        self.target += (-right * dx + up * dy) * scale;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4<f32> {
        let near = (self.distance * 0.01).max(0.001);
        let far = self.distance * 20.0 + 10.0;
        let projection = Mat4::perspective_rh_zo(self.fov_y, aspect.max(0.0001), near, far);
        let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::new(0.0, 1.0, 0.0));
        projection * view
    }
}

#[cfg(test)]
mod tests {
    use blockymodel::Vec4;

    use super::*;

    fn clip(vp: Mat4<f32>, p: Vec3<f32>) -> Vec4<f32> {
        vp * Vec4::new(p.x, p.y, p.z, 1.0)
    }

    fn ndc(vp: Mat4<f32>, p: Vec3<f32>) -> Vec3<f32> {
        let c = clip(vp, p);
        Vec3::new(c.x / c.w, c.y / c.w, c.z / c.w)
    }

    /// vek's `Mat4::new` and friends are shared between the row-major and
    /// column-major modules, so the argument order is easy to get backwards.
    /// These pin the conventions the shader relies on.
    #[test]
    fn composition_is_a_standard_matrix_product() {
        let a: Mat4<f32> = Mat4::translation_3d(Vec3::new(1.0, 0.0, 0.0));
        let b: Mat4<f32> = Mat4::scaling_3d(Vec3::broadcast(2.0_f32));
        let (p, q) = (Vec4::new(1.0, 1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0));

        let ab = (a * b) * p;
        let expected = a * (b * p);
        assert!(
            (ab - expected).magnitude() < 1e-5,
            "(a*b)v = {ab:?} but a(bv) = {expected:?}"
        );
        assert!((ab.xyz() - (a * b).mul_point(q)).magnitude() < 1e-5);
    }

    #[test]
    fn translation_moves_points_forward() {
        let m: Mat4<f32> = Mat4::translation_3d(Vec3::new(3.0, -2.0, 5.0));
        let p = m.mul_point(Vec3::new(1.0, 1.0, 1.0));
        assert!((p - Vec3::new(4.0, -1.0, 6.0)).magnitude() < 1e-5, "{p:?}");
    }

    /// A right-handed camera looking down `-Z` with zero-to-one depth: the
    /// target must land dead centre, in front of the near plane.
    #[test]
    fn look_at_puts_the_target_on_the_axis() {
        let camera = OrbitCamera::framing(Vec3::new(0.0, 50.0, 0.0), 60.0);
        let vp = camera.view_proj(16.0 / 9.0);

        let n = ndc(vp, camera.target);
        assert!(
            n.x.abs() < 1e-3 && n.y.abs() < 1e-3,
            "target is off-centre at {n:?}"
        );
        assert!(
            (0.0..1.0).contains(&n.z),
            "target depth {} is outside the 0..1 clip range",
            n.z
        );
    }

    /// Moving the sample point along the camera's own right vector must move
    /// it right on screen. This is the check that catches a transposed view.
    #[test]
    fn screen_axes_are_not_mirrored() {
        let camera = OrbitCamera::framing(Vec3::zero(), 50.0);
        let vp = camera.view_proj(1.0);

        let forward = blockymodel::normalize_or_zero(camera.target - camera.eye());
        let right = blockymodel::normalize_or_zero(forward.cross(Vec3::new(0.0, 1.0, 0.0)));
        let up = right.cross(forward);

        let base = ndc(vp, camera.target);
        let moved_right = ndc(vp, camera.target + right * 10.0);
        let moved_up = ndc(vp, camera.target + up * 10.0);

        assert!(
            moved_right.x > base.x,
            "moving right did not move +x on screen"
        );
        assert!(moved_up.y > base.y, "moving up did not move +y on screen");
    }

    /// The model must project inside the viewport, not off the bottom-right
    /// corner.
    #[test]
    fn a_framed_model_fills_a_reasonable_part_of_the_view() {
        let center = Vec3::new(0.0, 43.9, 3.7);
        let camera = OrbitCamera::framing(center, 67.8);
        let vp = camera.view_proj(16.0 / 9.0);

        let extent = Vec3::new(43.9, 88.5, 94.9) * 0.5;
        let mut max_ndc = 0.0_f32;
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    let corner = center + Vec3::new(extent.x * sx, extent.y * sy, extent.z * sz);
                    let c = clip(vp, corner);
                    assert!(c.w > 0.0, "corner {corner:?} is behind the camera");
                    let n = Vec3::new(c.x / c.w, c.y / c.w, c.z / c.w);
                    max_ndc = max_ndc.max(n.x.abs()).max(n.y.abs());
                }
            }
        }
        assert!(
            max_ndc < 1.0,
            "the model overflows the viewport (max |ndc| = {max_ndc})"
        );
        assert!(
            max_ndc > 0.3,
            "the model is too small (max |ndc| = {max_ndc})"
        );
    }
}
