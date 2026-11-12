use glam::Vec3;
use scalia_render::bend::{bend, Bend};

const R: f32 = 652_000.0;

fn close(a: Vec3, b: Vec3, tol: f32) -> bool {
    (a - b).length() <= tol
}

#[test]
fn flat_when_not_bending() {
    let b = Bend { tx: 1234.0, ty: -50.0, radius: R, t: 0.0 };
    let p = Vec3::new(5000.0, 300.0, 12.0);
    assert_eq!(bend(p, &b), p);
}

#[test]
fn the_target_and_its_surroundings_barely_move() {
    let b = Bend { tx: 100.0, ty: 20_000.0, radius: R, t: 1.0 };
    let p = Vec3::new(100.0, 20_000.0, 7.0);
    assert!(close(bend(p, &b), p, 0.05), "{:?}", bend(p, &b));
    // 100 m away the curvature drops the ground by only about d^2 / 2R (under 1 cm).
    let q = Vec3::new(200.0, 20_000.0, 0.0);
    assert!(close(bend(q, &b), q, 0.05), "{:?}", bend(q, &b));
}

#[test]
fn going_all_the_way_round_east_returns_to_the_same_point() {
    let b = Bend { tx: 0.0, ty: 0.0, radius: R, t: 1.0 };
    let w = std::f32::consts::TAU * R;
    let a = bend(Vec3::new(1000.0, 5000.0, 3.0), &b);
    let c = bend(Vec3::new(1000.0 + w, 5000.0, 3.0), &b);
    assert!(close(a, c, 2.0), "{a:?} vs {c:?}");
}

#[test]
fn fully_bent_map_is_a_sphere_hanging_below_the_target() {
    let b = Bend { tx: 0.0, ty: 300_000.0, radius: R, t: 1.0 };
    let centre = Vec3::new(0.0, 300_000.0, -R);
    for (x, y) in [(0.0, 0.0), (500_000.0, -600_000.0), (-1_900_000.0, 900_000.0), (1_000_000.0, 300_000.0)] {
        let p = bend(Vec3::new(x, y, 0.0), &b);
        let d = (p - centre).length();
        assert!((d - R).abs() < R * 1e-4, "({x}, {y}) is {d} from the centre");
    }
    // The north pole (y = R * pi / 2) sits on the axis through the centre.
    let pole = bend(Vec3::new(123_456.0, R * std::f32::consts::FRAC_PI_2, 0.0), &b);
    let pole2 = bend(Vec3::new(-777_000.0, R * std::f32::consts::FRAC_PI_2, 0.0), &b);
    assert!(close(pole, pole2, 5.0), "{pole:?} vs {pole2:?}");
    assert!(pole.y > centre.y, "the pole points north");
}

#[test]
fn height_raises_points_away_from_the_centre() {
    let b = Bend { tx: 0.0, ty: 0.0, radius: R, t: 1.0 };
    let centre = Vec3::new(0.0, 0.0, -R);
    let p = bend(Vec3::new(400_000.0, 100_000.0, 5_000.0), &b);
    assert!(((p - centre).length() - (R + 5_000.0)).abs() < 2.0);
}
