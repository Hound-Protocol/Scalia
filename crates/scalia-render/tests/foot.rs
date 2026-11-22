//! Foot roll (Patch: Animations 2): the boots strike on the heel, roll flat, push off from the toe and hang in the
//! swing, without sinking into the ground.
use scalia_render::contact::Body;
use scalia_render::pose::{params_dir, Pose};

#[test]
fn straight_gaits_strike_toe_up_and_push_off_heel_up_without_sinking() {
    let body = Body::new();
    for (pose, up, down) in [(Pose::Walk, 0.12, -0.45), (Pose::Run, 0.08, -0.5), (Pose::Sprint, 0.02, -0.25)] {
        for dir in [0.0, std::f32::consts::PI] {
            let p: Vec<_> = (0..48).map(|k| params_dir(pose, k as f32 / 48.0, dir)).collect();
            let hi = p.iter().map(|q| q.toe[0]).fold(f32::MIN, f32::max);
            let lo = p.iter().map(|q| q.toe[0]).fold(f32::MAX, f32::min);
            // Backing up the roll runs the other way (lands on the ball, leaves from the heel): it must still roll.
            let ok = if dir == 0.0 { hi > up && lo < down } else { hi > 0.1 && lo < -0.2 };
            assert!(ok, "{pose:?} dir {dir:.2}: the boot rolls only {lo:.2}..{hi:.2} rad");
            let sink = p.iter().map(|q| body.stats(q).boot).fold(f32::MAX, f32::min);
            assert!(sink > -0.015, "{pose:?} dir {dir:.2}: a boot sinks {:.3} m into the ground", -sink);
        }
    }
}

/// Footsteps sound on contact: each touchdown is a moment the boot (crawling, the knee) comes down onto the ground,
/// lifted shortly before and down at that moment, and the two legs land about half a cycle apart.
#[test]
fn touchdowns_are_where_the_boots_land() {
    use glam::Vec3;
    use scalia_render::mesh::Part;
    use scalia_render::pose::{pose_rigid, touchdowns, HEEL_Y, HIP_X, KNEE_Z, TOE_Y};
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint, Pose::CrouchWalk, Pose::Crawl] {
        for dir in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
            let t = touchdowns(pose, dir).unwrap();
            let apart = (t[1] - t[0]).rem_euclid(1.0);
            assert!((0.35..0.65).contains(&apart), "{pose:?} dir {dir:.2}: legs land {apart:.2} of a cycle apart");
            for i in 0..2 {
                let g = if i == 0 { -1.0 } else { 1.0 };
                let low = |phase: f32| {
                    let p = params_dir(pose, phase, dir);
                    if pose == Pose::Crawl {
                        pose_rigid(Vec3::new(g * HIP_X, 0.0, KNEE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, &p).z
                    } else {
                        let part = if i == 0 { Part::LeftFoot } else { Part::RightFoot };
                        [HEEL_Y, TOE_Y].map(|y| pose_rigid(Vec3::new(g * HIP_X, y, 0.0), part, &p).z).into_iter().fold(f32::MAX, f32::min)
                    }
                };
                let floor = (0..96).map(|k| low(k as f32 / 96.0)).fold(f32::MAX, f32::min);
                assert!(low(t[i]) < floor + 0.01, "{pose:?} dir {dir:.2} leg {i}: not down at its touchdown");
                assert!(low(t[i] - 0.06) > floor + 0.005, "{pose:?} dir {dir:.2} leg {i}: already down before its touchdown");
            }
        }
    }
}
