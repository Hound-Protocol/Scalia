use glam::{Vec3, Vec4};
use scalia_render::camera::{iso_view, light_matrix, snap_target, ELEV};

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn target_maps_to_screen_centre_at_mid_depth() {
    let t = Vec3::new(12.0, -7.0, 3.0);
    let (m, _) = iso_view(t, 0.7, 50.0, 16.0 / 9.0, 200.0);
    let c = m * t.extend(1.0);
    assert!(approx(c.x, 0.0, 1e-5) && approx(c.y, 0.0, 1e-5) && approx(c.z, 0.5, 1e-5), "{c:?}");
}

#[test]
fn up_is_up_on_screen_and_nearer_is_smaller_depth() {
    let t = Vec3::ZERO;
    let (m, vdir) = iso_view(t, 0.0, 50.0, 1.0, 200.0);
    let high = m * Vec4::new(0.0, 0.0, 10.0, 1.0);
    assert!(high.y > 0.0, "higher points draw higher");
    let toward = m * (vdir * 10.0).extend(1.0);
    assert!(toward.z < 0.5, "points toward the camera are nearer");
}

#[test]
fn view_height_spans_clip_space() {
    let (m, _) = iso_view(Vec3::ZERO, 0.3, 40.0, 2.0, 100.0);
    // A point half the view height "up" in the camera plane lands on the top edge.
    let up = Vec3::new(-0.3f32.sin() * ELEV.sin(), 0.3f32.cos() * ELEV.sin(), ELEV.cos());
    let c = m * (up * 20.0).extend(1.0);
    assert!(approx(c.y, 1.0, 1e-4), "{c:?}");
}

#[test]
fn snapped_target_sits_on_the_texel_grid() {
    let (yaw, texel) = (0.7f32, 0.37f32);
    let s = snap_target(Vec3::new(123.456, -78.9, 5.0), yaw, texel);
    let r = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
    let u = Vec3::new(-yaw.sin() * ELEV.sin(), yaw.cos() * ELEV.sin(), ELEV.cos());
    for v in [s.target.dot(r) / texel, s.target.dot(u) / texel] {
        assert!((v - v.round()).abs() < 1e-3, "{v}");
    }
    assert!(s.ex.abs() <= 0.5 + 1e-4 && s.ey.abs() <= 0.5 + 1e-4);
}

#[test]
fn light_matrix_centres_target_and_raises_toward_the_sun() {
    let t = Vec3::new(10.0, 20.0, 5.0);
    let m = light_matrix(glam::Vec3::new(0.4, -0.3, 0.85).normalize(), t, 50.0, 200.0, 2048);
    let c = m * t.extend(1.0);
    assert!(c.x.abs() < 0.03 && c.y.abs() < 0.03 && approx(c.z, 0.5, 1e-4), "{c:?}");
    let higher = m * (t + Vec3::Z * 10.0).extend(1.0);
    assert!(higher.z < c.z, "higher points are nearer the sun");
}

#[test]
fn camera_angle_can_change_and_defaults_to_isometric() {
    use scalia_render::camera::{basis, basis_at, ELEV};
    let (a, b) = (basis(0.7), basis_at(0.7, ELEV));
    assert!((a.0 - b.0).length() < 1e-6 && (a.1 - b.1).length() < 1e-6 && (a.2 - b.2).length() < 1e-6);
    for elev in [0.05f32, 0.4, 1.2, 1.5] {
        let (r, u, to_cam) = basis_at(0.3, elev);
        assert!(r.dot(u).abs() < 1e-5 && r.dot(to_cam).abs() < 1e-5 && u.dot(to_cam).abs() < 1e-5, "orthogonal at {elev}");
        assert!((to_cam.z - elev.sin()).abs() < 1e-5, "camera is {elev} rad above the horizon");
    }
}
