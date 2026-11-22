//! The low crawl (Patch: Animations 3): body flat on the ground, the forearms reaching and pulling in turn, one knee
//! drawn up to the side along the ground while the other leg lies straight back, then the other side.
use glam::Vec3;
use scalia_render::contact::Body;
use scalia_render::crawl;
use scalia_render::mesh::Part;
use scalia_render::pose::{params_dir, pose_rigid, Pose, PoseParams, CRAWL_STRIDE, ELBOW_X, ELBOW_Z, HIGH_CRAWL, HIP_X, HIP_Z, KNEE_Z, SHOULDER_X, SHOULDER_Z};

#[test]
fn baked_table_matches_the_solver() {
    let rows = crawl::solve();
    assert_eq!(rows.len(), HIGH_CRAWL.len());
    for (k, (a, b)) in rows.iter().zip(HIGH_CRAWL.iter()).enumerate() {
        for f in 0..crawl::F {
            // Loose enough for another machine's floating point through the iterative solve; any change to the
            // solver itself moves the table far more. What the table does is checked by the tests below.
            assert!((a[f] - b[f]).abs() < 0.02, "row {k} field {} is {} but crawl::solve gives {} (run the crawl_solve example with `rows`)", crawl::FIELDS[f], b[f], a[f]);
        }
    }
    assert_eq!(CRAWL_STRIDE, crawl::STRIDE);
}

fn at(p: &PoseParams, part: Part, v: Vec3) -> Vec3 {
    pose_rigid(v, part, p)
}

/// "Standing on knees and elbows" is what the high crawl looked like: the low crawl keeps the body on the ground.
#[test]
fn the_body_lies_flat_on_the_ground() {
    let body = Body::new();
    for k in 0..32 {
        let p = params_dir(Pose::Crawl, k as f32 / 32.0, 0.0);
        let s = body.stats(&p);
        assert!(s.belly < 0.04, "phase {k}/32: the belly is {:.3} m off the ground", s.belly);
        assert!(p.raise < 0.03, "phase {k}/32: the body is raised {:.3} m", p.raise);
        let hips = at(&p, Part::LeftThigh, Vec3::new(-HIP_X, 0.0, HIP_Z)).z.max(at(&p, Part::RightThigh, Vec3::new(HIP_X, 0.0, HIP_Z)).z);
        assert!(hips < 0.2, "phase {k}/32: the hips are {hips:.3} m up");
    }
}

#[test]
fn limbs_drag_on_the_ground_without_sinking_and_nothing_clips() {
    let body = Body::new();
    for k in 0..32 {
        let s = body.stats(&params_dir(Pose::Crawl, k as f32 / 32.0, 0.0));
        for (what, z) in [("knee", s.knee), ("elbow", s.elbow), ("boot", s.boot)] {
            assert!((-0.02..0.06).contains(&z), "phase {k}/32: lowest {what} at {z:.3} m");
        }
        assert!(s.rifle > -0.01, "phase {k}/32: the rifle sinks {:.3} m into the ground", -s.rifle);
        assert!(s.head > 0.15, "phase {k}/32: head {:.3} m, he must see ahead", s.head);
        assert!(s.arm_in < 0.03, "phase {k}/32: an arm sinks {:.3} m into the torso", s.arm_in);
    }
}

/// How far each knee sits out from its hip, over the cycle.
fn knee_out(phase: f32, dir: f32) -> [f32; 2] {
    let p = params_dir(Pose::Crawl, phase, dir);
    [0, 1].map(|i| {
        let g = if i == 0 { -1.0 } else { 1.0 };
        let shin = if i == 0 { Part::LeftShin } else { Part::RightShin };
        let thigh = if i == 0 { Part::LeftThigh } else { Part::RightThigh };
        (at(&p, shin, Vec3::new(g * HIP_X, 0.0, KNEE_Z)).x - at(&p, thigh, Vec3::new(g * HIP_X, 0.0, HIP_Z)).x) * g
    })
}

#[test]
fn one_knee_draws_up_to_the_side_while_the_other_leg_lies_back_then_they_swap() {
    let k: Vec<[f32; 2]> = (0..32).map(|j| knee_out(j as f32 / 32.0, 0.0)).collect();
    for i in 0..2 {
        let most = k.iter().map(|o| o[i]).fold(f32::MIN, f32::max);
        let least = k.iter().map(|o| o[i]).fold(f32::MAX, f32::min);
        assert!(most > 0.28, "leg {i}: the knee draws out only {most:.2} m");
        assert!(least < 0.1, "leg {i}: the leg never lies straight back ({least:.2} m out)");
        // While this knee is drawn up, the other leg lies back.
        let at_most = k.iter().position(|o| o[i] == most).unwrap();
        assert!(k[at_most][1 - i] < 0.15, "leg {i} drawn while the other is {:.2} m out", k[at_most][1 - i]);
    }
}

/// The forearms reach ahead and pull back in turn, the reaching arm opposite the drawn knee.
#[test]
fn the_forearms_reach_and_pull_opposite_the_drawn_knee() {
    let elbow_ahead = |phase: f32, i: usize| {
        let p = params_dir(Pose::Crawl, phase, 0.0);
        let g = if i == 0 { -1.0 } else { 1.0 };
        let (arm, fore) = if i == 0 { (Part::LeftArm, Part::LeftForearm) } else { (Part::RightArm, Part::RightForearm) };
        at(&p, fore, Vec3::new(g * ELBOW_X, 0.0, ELBOW_Z)).y - at(&p, arm, Vec3::new(g * SHOULDER_X, 0.0, SHOULDER_Z)).y
    };
    for i in 0..2 {
        let y: Vec<f32> = (0..32).map(|j| elbow_ahead(j as f32 / 32.0, i)).collect();
        let range = y.iter().copied().fold(f32::MIN, f32::max) - y.iter().copied().fold(f32::MAX, f32::min);
        assert!(range > 0.2, "arm {i}: the elbow reaches and pulls over only {range:.2} m");
    }
    // At the left knee's most drawn, the right elbow is well ahead of the left.
    let k: Vec<f32> = (0..32).map(|j| knee_out(j as f32 / 32.0, 0.0)[0]).collect();
    let j = k.iter().position(|o| *o == k.iter().copied().fold(f32::MIN, f32::max)).unwrap() as f32 / 32.0;
    assert!(elbow_ahead(j, 1) > elbow_ahead(j, 0) + 0.1, "the right arm reaches as the left knee draws up");
}

#[test]
fn every_direction_keeps_the_body_down() {
    let body = Body::new();
    for dir in [std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        for k in 0..16 {
            let s = body.stats(&params_dir(Pose::Crawl, k as f32 / 16.0, dir));
            assert!(s.belly < 0.05 && s.knee > -0.03 && s.elbow > -0.03 && s.boot > -0.03, "dir {dir:.2} phase {k}/16: {s:?}");
        }
    }
    let f = params_dir(Pose::Crawl, 0.3, 0.0);
    let b = params_dir(Pose::Crawl, 0.7, std::f32::consts::PI);
    assert!((f.hip[0] - b.hip[0]).abs() < 1e-4, "backwards is the loop reversed");
}

fn palm_ahead(phase: f32, i: usize) -> (f32, f32) {
    use scalia_render::pose::PALM;
    let p = params_dir(Pose::Crawl, phase, 0.0);
    let g = if i == 0 { -1.0 } else { 1.0 };
    let (arm, fore) = if i == 0 { (Part::LeftArm, Part::LeftForearm) } else { (Part::RightArm, Part::RightForearm) };
    let sh = at(&p, arm, Vec3::new(g * SHOULDER_X, 0.0, SHOULDER_Z));
    let h = at(&p, fore, Vec3::new(g * PALM.x, PALM.y, PALM.z));
    (h.y - sh.y, (h.x - sh.x) * g)
}

/// Patch: Animations 4: the forearms reach forward, ahead of the head and in line with the shoulders, not out to the
/// side like a swimmer's.
#[test]
fn the_forearms_reach_forward_in_line_with_the_shoulders() {
    for i in 0..2 {
        let reach: Vec<(f32, f32)> = (0..32).map(|k| palm_ahead(k as f32 / 32.0, i)).collect();
        let most = reach.iter().map(|r| r.0).fold(f32::MIN, f32::max);
        assert!(most > 0.45, "arm {i}: the hand reaches only {most:.2} m ahead of the shoulder");
        // Reached, the hand is in line with the shoulder (it ends the pull beside the ribs, as a real crawl does).
        let at_most = reach.iter().find(|r| r.0 == most).unwrap();
        assert!(at_most.1 < 0.15, "arm {i}: reaching, the hand is {:.2} m out to the side", at_most.1);
    }
}

/// The rifle lies along the right forearm, muzzle forward, clear of the ground and the body (army low crawl).
#[test]
fn the_rifle_lies_along_the_right_forearm_muzzle_forward() {
    use scalia_render::pose::{barrel_with_phases, RIFLE};
    let body = Body::new();
    for k in 0..32 {
        let phase = k as f32 / 32.0;
        let (_, bore) = barrel_with_phases(RIFLE, Pose::Crawl, None, phase, 0.0, 0.0);
        assert!(bore.y > 0.9, "phase {k}/32: the muzzle points {bore:?}, not ahead");
        let s = body.stats(&params_dir(Pose::Crawl, phase, 0.0));
        assert!(s.rifle > 0.005, "phase {k}/32: the rifle touches the ground ({:.3} m)", s.rifle);
        assert!(s.rifle_in < 0.02, "phase {k}/32: the rifle sinks {:.3} m into the body", s.rifle_in);
    }
}

/// Only one knee is drawn up at a time: the other leg lies straight back.
#[test]
fn one_leg_always_lies_straight_back() {
    for k in 0..32 {
        let phase = k as f32 / 32.0;
        let p = params_dir(Pose::Crawl, phase, 0.0);
        let out = knee_out(phase, 0.0);
        let straight = (0..2).any(|i| p.knee[i] < 0.3 && out[i] < 0.1);
        assert!(straight, "phase {k}/32: both legs bent (knee {:.2?}, out {:.2?})", p.knee, out);
    }
}

/// While a knee pushes and a forearm pulls they hold their place along the way he moves (the body slides over them,
/// STRIDE per cycle): no skating.
#[test]
fn pushing_knees_and_pulling_forearms_hold_their_place() {
    let n = 128;
    let pts = [
        (Part::LeftShin, Vec3::new(-HIP_X, 0.0, KNEE_Z), 0.0, 0.03),
        (Part::RightShin, Vec3::new(HIP_X, 0.0, KNEE_Z), 0.5, 0.03),
        (Part::RightForearm, Vec3::new(ELBOW_X, 0.0, ELBOW_Z), 0.0, 0.05),
        (Part::LeftForearm, Vec3::new(-ELBOW_X, 0.0, ELBOW_Z), 0.5, 0.05),
    ];
    for (part, v, off, limit) in pts {
        let y = |u: f32| at(&params_dir(Pose::Crawl, u, 0.0), part, v).y + crawl::STRIDE * u;
        let us: Vec<f32> = (0..n).map(|k| k as f32 / n as f32).filter(|u| crawl::planted(u + off)).collect();
        assert!(us.len() > n / 4, "{part:?} is planted only {} of {n} samples", us.len());
        let y0 = y(us[0]);
        let wander = us.iter().map(|u| (y(*u) - y0).abs()).fold(0.0f32, f32::max);
        assert!(wander < limit, "{part:?} skates {wander:.3} m along the way he moves while it pushes");
    }
}

/// How far the left knee is drawn up less the right: 1 with the left knee fully up, -1 with the right.
fn drawn(phase: f32) -> f32 {
    let d = |u: f32| 1.0 - crawl::stroke(u).0 / crawl::PUSH;
    d(phase) - d(phase + 0.5)
}

/// Patch: Animations 5: the body works with the stroke. The hip of the drawn knee comes up off the ground (the body
/// rolls onto the other side) and the spine bends toward the drawn knee, swinging the shoulders that way; the head
/// rides steadier than the shoulders, low, eyes ahead.
#[test]
fn the_body_rolls_and_bends_toward_the_drawn_knee() {
    use scalia_render::pose::NECK_Z;
    for k in 0..32 {
        let phase = k as f32 / 32.0;
        let p = params_dir(Pose::Crawl, phase, 0.0);
        let s = drawn(phase);
        let side = |x: f32, z: f32| (at(&p, Part::Torso, Vec3::new(-x, 0.0, z)), at(&p, Part::Torso, Vec3::new(x, 0.0, z)));
        let ((hl, hr), (sl, sr)) = (side(HIP_X, HIP_Z), side(SHOULDER_X, SHOULDER_Z));
        let (el, er) = (at(&p, Part::Head, Vec3::new(-0.05, 0.1, NECK_Z + 0.12)), at(&p, Part::Head, Vec3::new(0.05, 0.1, NECK_Z + 0.12)));
        let (chest, head) = (((sl.z - sr.z) / sl.distance(sr)).asin(), ((el.z - er.z) / el.distance(er)).asin());
        assert!(head.abs() <= 0.5 * chest.abs() + 0.01, "phase {k}/32: the head rolls {head:.3} rad with the chest's {chest:.3}");
        let face = at(&p, Part::Head, Vec3::new(0.0, 0.1, NECK_Z + 0.12)) - at(&p, Part::Head, Vec3::new(0.0, 0.0, NECK_Z + 0.12));
        let down = (-face.z / face.length()).asin();
        assert!((0.3..0.7).contains(&down), "phase {k}/32: eyes {down:.3} rad below level, want the head low, eyes ahead");
        if s.abs() < 0.8 {
            continue;
        }
        let lift = (hl.z - hr.z) * s.signum();
        assert!((0.02..0.06).contains(&lift), "phase {k}/32: the drawn knee's hip is up {lift:.3} m");
        let swing = ((sl.x + sr.x) / 2.0 - (hl.x + hr.x) / 2.0) * -s.signum();
        assert!((0.02..0.08).contains(&swing), "phase {k}/32: the shoulders swing {swing:.3} m toward the drawn knee");
    }
}
