use glam::Vec3;
use scalia_render::prims::{two_bone, Spring};

fn run(s: &mut Spring, target: f32, dt: f32, secs: f32) -> (f32, f32) {
    let mut peak = f32::MIN;
    let mut t = 0.0;
    while t < secs {
        let y = s.step(target, dt);
        peak = peak.max(y);
        t += dt;
    }
    (s.y, peak)
}

#[test]
fn spring_settles_on_its_target() {
    let mut s = Spring::new(2.0, 0.7, 0.0, 0.0);
    let (y, _) = run(&mut s, 1.0, 1.0 / 60.0, 3.0);
    assert!((y - 1.0).abs() < 1e-3, "settled at {y}");
}

#[test]
fn underdamped_spring_overshoots_and_critical_does_not() {
    let (_, under) = run(&mut Spring::new(2.0, 0.4, 0.0, 0.0), 1.0, 1.0 / 60.0, 3.0);
    let (_, crit) = run(&mut Spring::new(2.0, 1.0, 0.0, 0.0), 1.0, 1.0 / 60.0, 3.0);
    assert!(under > 1.05, "underdamped peak {under}");
    assert!(crit <= 1.0 + 1e-4, "critical peak {crit}");
}

#[test]
fn negative_response_anticipates() {
    let mut s = Spring::new(2.0, 0.7, -1.0, 0.0);
    s.step(0.0, 1.0 / 60.0);
    let first = s.step(1.0, 1.0 / 60.0);
    assert!(first < 0.0, "first move {first} should go the other way");
}

#[test]
fn spring_is_frame_rate_independent() {
    let (a, _) = run(&mut Spring::new(3.0, 0.5, 0.0, 0.0), 1.0, 1.0 / 120.0, 0.3);
    let (b, _) = run(&mut Spring::new(3.0, 0.5, 0.0, 0.0), 1.0, 1.0 / 30.0, 0.3);
    assert!((a - b).abs() < 0.06, "120 Hz {a} vs 30 Hz {b}");
}

#[test]
fn spring_survives_a_huge_frame() {
    let mut s = Spring::new(8.0, 0.2, 0.0, 0.0);
    let y = s.step(1.0, 2.0);
    assert!(y.is_finite() && y.abs() < 3.0, "after a 2 s frame {y}");
}

#[test]
fn two_bone_keeps_lengths_reaches_and_bends_toward_the_pole() {
    let root = Vec3::new(0.0, 0.0, 0.88);
    for target in [Vec3::new(0.1, 0.2, 0.15), Vec3::new(-0.2, 0.3, 0.5), Vec3::new(0.0, 0.05, 0.2)] {
        let pole = Vec3::Y;
        let mid = two_bone(root, target, 0.42, 0.375, pole);
        assert!(((mid - root).length() - 0.42).abs() < 1e-4);
        assert!(((target - mid).length() - 0.375).abs() < 1e-4, "end misses {target}");
        let axis = (target - root).normalize();
        let off = (mid - root) - axis * (mid - root).dot(axis);
        assert!(off.dot(pole) > 0.0, "knee bends away from the pole for {target}");
    }
}

#[test]
fn two_bone_straightens_toward_an_unreachable_target() {
    let root = Vec3::ZERO;
    let target = Vec3::new(0.0, 0.0, -5.0);
    let mid = two_bone(root, target, 0.4, 0.4, Vec3::Y);
    assert!((mid - Vec3::new(0.0, 0.0, -0.4)).length() < 1e-3, "{mid}");
}
