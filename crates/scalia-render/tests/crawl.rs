//! The high crawl (Patch: Animations 2): on elbows and knees, body off the ground, rifle cradled.
use glam::Vec3;
use scalia_render::contact::Body;
use scalia_render::crawl::{self, stroke, STRIDE};
use scalia_render::mesh::Part;
use scalia_render::pose::{params_dir, pose_rigid, Pose, CRAWL_STRIDE, ELBOW_X, ELBOW_Z, HIGH_CRAWL, HIP_X, KNEE_Z};

#[test]
fn baked_table_matches_the_solver() {
    let rows = crawl::solve();
    assert_eq!(rows.len(), HIGH_CRAWL.len());
    for (k, (a, b)) in rows.iter().zip(HIGH_CRAWL.iter()).enumerate() {
        for f in 0..crawl::F {
            assert!((a[f] - b[f]).abs() < 2e-4, "row {k} field {} is {} but crawl::solve gives {} (run the crawl_solve example with `rows`)", crawl::FIELDS[f], b[f], a[f]);
        }
    }
    assert_eq!(CRAWL_STRIDE, STRIDE);
}

#[test]
fn knees_elbows_and_toes_touch_the_ground_and_the_body_stays_off_it() {
    let body = Body::new();
    for k in 0..32 {
        let p = params_dir(Pose::Crawl, k as f32 / 32.0, 0.0);
        let s = body.stats(&p);
        for (what, z) in [("knee", s.knee), ("elbow", s.elbow), ("boot", s.boot)] {
            assert!((-0.015..0.03).contains(&z), "phase {k}/32: lowest {what} at {z:.3} m (ground contact wanted)");
        }
        assert!(s.belly > 0.08, "phase {k}/32: belly {:.3} m off the ground", s.belly);
        assert!(s.rifle > 0.03, "phase {k}/32: rifle {:.3} m off the ground", s.rifle);
        assert!(s.head > 0.15, "phase {k}/32: head {:.3} m", s.head);
        assert!(s.arm_in < 0.03, "phase {k}/32: an arm sinks {:.3} m into the torso", s.arm_in);
    }
}

fn joint(p: &scalia_render::pose::PoseParams, part: Part, at: Vec3) -> Vec3 {
    pose_rigid(at, part, p)
}

/// Planted knees and elbows hold their place on the ground while the body moves over them (STRIDE per cycle):
/// from landing to lifting, each wanders this little from where it landed (forearms may drag a little more).
#[test]
fn planted_knees_and_elbows_do_not_slide() {
    let n = 128;
    let pts = [
        (Part::LeftShin, Vec3::new(-HIP_X, 0.0, KNEE_Z), 0.0, 0.03),
        (Part::RightShin, Vec3::new(HIP_X, 0.0, KNEE_Z), 0.5, 0.03),
        (Part::RightForearm, Vec3::new(ELBOW_X, 0.0, ELBOW_Z), 0.0, 0.06),
        (Part::LeftForearm, Vec3::new(-ELBOW_X, 0.0, ELBOW_Z), 0.5, 0.06),
    ];
    for (part, at, off, limit) in pts {
        let world = |u: f32| (joint(&params_dir(Pose::Crawl, u, 0.0), part, at) + Vec3::Y * STRIDE * u).truncate();
        // Landing is at stroke u = 0, i.e. phase = -off.
        let land = world(-off);
        let mut worst = 0.0f32;
        for k in 0..n {
            let u = crawl::DUTY * k as f32 / n as f32;
            assert_eq!(stroke(u).1, 0.0);
            worst = worst.max(world(u - off).distance(land));
        }
        assert!(worst < limit, "{part:?} wanders {worst:.3} m from where it landed while planted");
    }
}

/// Opposite elbow and knee work together: the right elbow reaches as the left knee draws up.
#[test]
fn opposite_elbow_and_knee_stroke_together() {
    let y = |phase: f32, part: Part, at: Vec3| joint(&params_dir(Pose::Crawl, phase, 0.0), part, at).y;
    let lk = |ph| y(ph, Part::LeftShin, Vec3::new(-HIP_X, 0.0, KNEE_Z));
    let re = |ph| y(ph, Part::RightForearm, Vec3::new(ELBOW_X, 0.0, ELBOW_Z));
    let rk = |ph| y(ph, Part::RightShin, Vec3::new(HIP_X, 0.0, KNEE_Z));
    // Both forward at the end of their swing (u = 1 - small), both back at the end of stance.
    let (f, b) = (0.99, crawl::DUTY - 0.01);
    assert!(lk(f) - lk(b) > 0.15 && re(f) - re(b) > 0.15, "left knee and right elbow move together");
    assert!(rk(f + 0.5) - rk(b + 0.5) > 0.15, "the right knee is half a cycle on");
}

/// Backwards plays the same loop in reverse; sideways keeps the body off the ground too.
#[test]
fn every_direction_keeps_the_body_off_the_ground() {
    let body = Body::new();
    for dir in [std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        for k in 0..16 {
            let s = body.stats(&params_dir(Pose::Crawl, k as f32 / 16.0, dir));
            assert!(s.belly > 0.08 && s.knee > -0.03 && s.elbow > -0.03, "dir {dir:.2} phase {k}/16: {s:?}");
        }
    }
    let f = params_dir(Pose::Crawl, 0.3, 0.0);
    let b = params_dir(Pose::Crawl, 0.7, std::f32::consts::PI);
    assert!((f.hip[0] - b.hip[0]).abs() < 1e-4, "backwards is the loop reversed");
}
