//! Isometric orthographic camera, texel snapping and the sun's shadow camera.

use glam::{Mat4, Vec3, Vec4};

/// True isometric elevation, about 35.26 degrees.
pub const ELEV: f32 = 0.615_479_7;

/// Builds a matrix from three rows (x, y, depth) that map world points to clip space,
/// with the target at the centre and at depth 0.5.
fn from_rows(rows: [Vec3; 3], target: Vec3) -> Mat4 {
    let off = [0.0, 0.0, 0.5];
    let r = |i: usize| Vec4::new(rows[i].x, rows[i].y, rows[i].z, off[i] - rows[i].dot(target));
    Mat4::from_cols(r(0), r(1), r(2), Vec4::W).transpose()
}

/// Camera basis for a yaw: right and up in the camera plane, and the direction towards the camera.
pub fn basis(yaw: f32) -> (Vec3, Vec3, Vec3) {
    let (s, c) = yaw.sin_cos();
    let (se, ce) = ELEV.sin_cos();
    (Vec3::new(c, s, 0.0), Vec3::new(-s * se, c * se, ce), Vec3::new(s * ce, -c * ce, se))
}

/// View-projection for an isometric camera `view_height` world units tall, and the direction towards the camera.
pub fn iso_view(target: Vec3, yaw: f32, view_height: f32, aspect: f32, range: f32) -> (Mat4, Vec3) {
    let (r, u, to_cam) = basis(yaw);
    let hh = view_height / 2.0;
    let hw = hh * aspect;
    let m = from_rows([r / hw, u / hh, -to_cam / (2.0 * range)], target);
    (m, to_cam)
}

pub struct Snap {
    pub target: Vec3,
    /// Snap error in texels, to the right and down the screen.
    pub ex: f32,
    pub ey: f32,
}

/// Moves the target onto the camera-plane texel grid and returns the error the upscale must undo.
pub fn snap_target(target: Vec3, yaw: f32, texel: f32) -> Snap {
    let (r, u, _) = basis(yaw);
    let (tr, tu) = (target.dot(r), target.dot(u));
    let er = tr - (tr / texel).round() * texel;
    let eu = tu - (tu / texel).round() * texel;
    Snap { target: target - r * er - u * eu, ex: er / texel, ey: -eu / texel }
}

/// Orthographic camera for the light coming from `to_light` (unit), centred on `target`, `half` units
/// each way, snapped to its own texels so shadows do not shimmer.
pub fn light_matrix(to_light: Vec3, target: Vec3, half: f32, range: f32, size: u32) -> Mat4 {
    let f = -to_light;
    let r = f.cross(Vec3::Z).normalize();
    let u = r.cross(f);
    let texel = 2.0 * half / size as f32;
    let er = target.dot(r) - (target.dot(r) / texel).round() * texel;
    let eu = target.dot(u) - (target.dot(u) / texel).round() * texel;
    let t = target - r * er - u * eu;
    from_rows([r / half, u / half, f / (2.0 * range)], t)
}
