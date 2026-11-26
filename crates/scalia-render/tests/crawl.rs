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
        // AN-07: the hips rise a few centimetres as a leg pushes (crawl::HIP_RISE), no more.
        assert!(p.raise < 0.065, "phase {k}/32: the body is raised {:.3} m", p.raise);
        let hips = at(&p, Part::LeftThigh, Vec3::new(-HIP_X, 0.0, HIP_Z)).z.max(at(&p, Part::RightThigh, Vec3::new(HIP_X, 0.0, HIP_Z)).z);
        // AN-07: 22 cm, the drawn knee's hip rolled up and the hips risen with the push (measured 21.1; it was 20 cm with no rise);
        // on knees and elbows they stood near 30 cm.
        assert!(hips < 0.22, "phase {k}/32: the hips are {hips:.3} m up");
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
    // AN-07: the hand is what grips (the elbow swings out round it as the arm folds), so it is the hand that reaches
    // and pulls.
    let hand_ahead = |phase: f32, i: usize| palm_ahead(phase, i).0;
    for i in 0..2 {
        let y: Vec<f32> = (0..32).map(|j| hand_ahead(j as f32 / 32.0, i)).collect();
        let range = y.iter().copied().fold(f32::MIN, f32::max) - y.iter().copied().fold(f32::MAX, f32::min);
        assert!(range > 0.3, "arm {i}: the hand reaches and pulls over only {range:.2} m");
    }
    // At the left knee's most drawn, the right hand is well ahead of the left.
    let k: Vec<f32> = (0..32).map(|j| knee_out(j as f32 / 32.0, 0.0)[0]).collect();
    let j = k.iter().position(|o| *o == k.iter().copied().fold(f32::MIN, f32::max)).unwrap() as f32 / 32.0;
    assert!(hand_ahead(j, 1) > hand_ahead(j, 0) + 0.1, "the right arm reaches as the left knee draws up");
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

/// Where the boot and the hand of each side are in the world (the root moving STRIDE per cycle, as the game moves him).
fn grips(u: f32) -> [[Vec3; 2]; 2] {
    use scalia_render::pose::{ANKLE_Z, PALM};
    let p = params_dir(Pose::Crawl, u, 0.0);
    let ahead = Vec3::new(0.0, crawl::STRIDE * u, 0.0);
    [
        [at(&p, Part::LeftShin, Vec3::new(-HIP_X, 0.0, ANKLE_Z)) + ahead, at(&p, Part::RightShin, Vec3::new(HIP_X, 0.0, ANKLE_Z)) + ahead],
        [at(&p, Part::LeftForearm, Vec3::new(-PALM.x, PALM.y, PALM.z)) + ahead, at(&p, Part::RightForearm, Vec3::new(PALM.x, PALM.y, PALM.z)) + ahead],
    ]
}

/// AN-07: a pushing boot and a pulling hand hold their spot on the ground, along the way he moves and across it, while
/// the body surges over them. Before, the knees "pushed" while sweeping 32 cm in toward the body and the elbows
/// "pulled" while sweeping 20 cm out: the crawl skated.
#[test]
fn pushing_boots_and_pulling_hands_hold_their_spot() {
    let n = 128;
    let planted = |u: f32, c: usize, i: usize| if c == 0 { crawl::planted(u + if i == 0 { 0.0 } else { 0.5 }) } else { crawl::arm_planted(crawl::arm_u(u, i)) };
    for (c, what, limit) in [(0, "boot", 0.015), (1, "hand", 0.015)] {
        for i in 0..2 {
            let mut land: Option<Vec3> = None;
            let mut worst = 0.0f32;
            let mut planted_for = 0;
            // Twice round, so a stroke that spans the loop's seam is measured whole.
            for k in 0..2 * n {
                let u = k as f32 / n as f32;
                if planted(u, c, i) {
                    let at = grips(u)[c][i];
                    let l = *land.get_or_insert(at);
                    worst = worst.max((at - l).truncate().length());
                    planted_for += 1;
                } else {
                    land = None;
                }
            }
            assert!(planted_for > n / 2, "{what} {i} is planted only {planted_for} of {} samples", 2 * n);
            assert!(worst < limit, "{what} {i} skates {worst:.3} m over the ground while it grips");
        }
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
        // AN-04: about half as much again as Patch 5's roll and bend (the player found them too slight; much more and
        // the rifle side rolls onto the rifle and the forearms cannot hold their place).
        let lift = (hl.z - hr.z) * s.signum();
        assert!((0.04..0.10).contains(&lift), "phase {k}/32: the drawn knee's hip is up {lift:.3} m");
        let swing = ((sl.x + sr.x) / 2.0 - (hl.x + hr.x) / 2.0) * -s.signum();
        assert!((0.05..0.12).contains(&swing), "phase {k}/32: the shoulders swing {swing:.3} m toward the drawn knee");
    }
}

/// Patch: Animations 5: an unhurried rhythm. At the ordinary crawl (0.55 m/s, scalia-wasm `CMD_CRAWL`) he makes at
/// most 1.4 strokes a second (one per arm; it was 1.6, frantic sprinting); each stroke carries him further instead.
#[test]
fn the_crawl_keeps_an_unhurried_rhythm() {
    let strokes = 2.0 * 0.55 / CRAWL_STRIDE;
    assert!(strokes <= 1.4, "{strokes:.2} strokes a second at the ordinary crawl");
}

/// Setting off into the crawl from lying prone and stopping back onto the prone legs, as drawn (the support hand's grip
/// blended in, and the chest's prop, `pose::crawl_prop`): no joint snaps (faster than 15 rad/s over the 0.35 s
/// blend, the anim_audit standard), the rifle never swings into the ground, and nothing presses more than 1 cm further
/// into the ground on the way than it lies at either end. AN-04: the crawl starts and settles only with both legs
/// lying straight back (stroke 0 or a half, `anim.rs`), where nothing has to swing past the ground; before, from any
/// moment of the stroke, knees and elbows dipped up to 5 cm.
#[test]
fn starting_and_stopping_the_crawl_blend_smoothly() {
    use scalia_render::pose::{blend, crawl_prop, grip, grip_weight, params, BLEND_S};
    let angles = |p: &PoseParams| {
        let mut v = vec![p.lean, p.head, p.rifle, p.rifle_yaw, p.twist, p.look, p.tilt, p.yaw, p.raise];
        for pair in [p.hip, p.knee, p.side, p.shoulder, p.elbow, p.reach, p.abduct, p.roll] {
            v.extend(pair);
        }
        v
    };
    let body = Body::new();
    let drawn = |a: Pose, pa: &PoseParams, b: Pose, pb: &PoseParams, w: f32| {
        let mut p = blend(pa, pb, w);
        p.lean -= crawl_prop(a, b, w);
        grip(&p, grip_weight(a, b, w))
    };
    let prone = params(Pose::Prone, 0.0);
    for settle in [0.0, 0.5] {
        let crawl = params_dir(Pose::Crawl, settle, 0.0);
        for (a, pa, b, pb, what) in [(Pose::Prone, &prone, Pose::Crawl, &crawl, "setting off"), (Pose::Crawl, &crawl, Pose::Prone, &prone, "stopping")] {
            let (ea, eb) = (body.stats(&drawn(a, pa, b, pb, 0.0)), body.stats(&drawn(a, pa, b, pb, 1.0)));
            let mut prev = angles(&drawn(a, pa, b, pb, 0.0));
            for i in 1..=20 {
                let p = drawn(a, pa, b, pb, i as f32 / 20.0);
                let cur = angles(&p);
                let fastest = cur.iter().zip(&prev).map(|(x, y)| (x - y).abs() / (BLEND_S / 20.0)).fold(0.0f32, f32::max);
                assert!(fastest <= 15.0, "{what} at stroke {settle}: a joint turns {fastest:.1} rad/s");
                prev = cur;
                let s = body.stats(&p);
                for (name, now, ends) in [("knee", s.knee, ea.knee.min(eb.knee)), ("elbow", s.elbow, ea.elbow.min(eb.elbow)), ("boot", s.boot, ea.boot.min(eb.boot)), ("rifle", s.rifle, ea.rifle.min(eb.rifle).min(0.01))] {
                    assert!(now > ends - 0.01, "{what} at stroke {settle}, {i}/20 of the way: the {name} dips to {now:.3} m (ends at {ends:.3})");
                }
            }
        }
    }
}

/// AN-04: a forearm is either planted and pulling (holding its place) or reaching forward off the ground; it never
/// lies on the ground being dragged along with the body (before AN-04 each forearm rested half the cycle at the body's
/// side, skating some 40 cm a stroke).
#[test]
fn forearms_never_drag_along_the_ground() {
    // AN-07: the hand grips; down on the ground it never moves ahead with the body (only reaching, lifted, does it).
    let n = 128;
    for i in 0..2 {
        let body_step = crawl::STRIDE / n as f32;
        let dragged = (0..n)
            .filter(|k| {
                let (u0, u1) = (*k as f32 / n as f32, (*k + 1) as f32 / n as f32);
                let (a, b) = (grips(u0)[1][i], grips(u1)[1][i]);
                a.z.min(b.z) < crawl::PALM_H[i] + 0.008 && b.y - a.y > 0.3 * body_step
            })
            .count();
        assert!(dragged <= n / 16, "hand {i} is dragged along the ground {dragged} of {n} steps");
    }
}

/// AN-04: through the whole stroke the small parts never poke into the ground: elbows and boots at most 5 mm (cloth on
/// dirt), the legs (thighs at the hips and knees, where his weight rests with the belly) at most 1.5 cm, and the rifle
/// stays at least 1 cm clear.
#[test]
fn nothing_sinks_into_the_ground_through_the_stroke() {
    let body = Body::new();
    for k in 0..64 {
        let s = body.stats(&params_dir(Pose::Crawl, k as f32 / 64.0, 0.0));
        assert!(s.knee > -0.015 && s.elbow > -0.005 && s.boot > -0.005, "phase {k}/64: {s:?}");
        assert!(s.rifle > 0.01, "phase {k}/64: the rifle is {:.3} m off the ground", s.rifle);
    }
}

/// AN-07: the body works like a body, not a sled. It surges: fastest while a leg pushes, slowest while it draws up
/// (the pelvis at under 0.6 and over 1.4 times the crawl's average speed; before, exactly 1). It rises and falls
/// with the stroke, 1.5 to 4 cm at the hips and at the chest (before, flat to the millimetre), the chest a little
/// after the hips; and the head lags the surge.
#[test]
fn the_body_surges_and_rises_with_the_stroke() {
    use scalia_render::pose::NECK_Z;
    let n = 64;
    let mid = |p: &PoseParams, x: f32, z: f32| (at(p, Part::Torso, Vec3::new(-x, 0.0, z)) + at(p, Part::Torso, Vec3::new(x, 0.0, z))) / 2.0;
    let (mut hips, mut chest, mut head) = (vec![], vec![], vec![]);
    for k in 0..=n {
        let u = k as f32 / n as f32;
        let p = params_dir(Pose::Crawl, u, 0.0);
        let ahead = Vec3::new(0.0, crawl::STRIDE * u, 0.0);
        hips.push(mid(&p, HIP_X, HIP_Z) + ahead);
        chest.push(mid(&p, SHOULDER_X, SHOULDER_Z) + ahead);
        head.push(at(&p, Part::Head, Vec3::new(0.0, 0.1, NECK_Z + 0.12)) + ahead);
    }
    let speed = |s: &[Vec3]| -> Vec<f32> { (0..n).map(|k| (s[k + 1].y - s[k].y) / (crawl::STRIDE / n as f32)).collect() };
    let span = |v: &[f32]| (v.iter().copied().fold(f32::MAX, f32::min), v.iter().copied().fold(f32::MIN, f32::max));
    let (slow, fast) = span(&speed(&hips));
    assert!(slow < 0.6 && fast > 1.4, "the pelvis moves at {slow:.2} to {fast:.2} times the average: no surge");
    let height = |s: &[Vec3]| s[..n].iter().map(|v| v.z).collect::<Vec<f32>>();
    for (what, s) in [("hips", &hips), ("chest", &chest)] {
        let (lo, hi) = span(&height(s));
        assert!((0.015..0.04).contains(&(hi - lo)), "the {what} rise and fall {:.3} m over the stroke", hi - lo);
    }
    let peak = |s: &[Vec3]| (0..n / 2).max_by(|a, b| s[*a].z.total_cmp(&s[*b].z)).unwrap();
    let (h, c) = (peak(&hips), peak(&chest));
    assert!(c > h && c - h <= n / 8, "the chest peaks at {c}/{n}, the hips at {h}/{n}: want the chest a little after");
    // The head lags: it tips back as the body speeds up, a little after (its nod against the chest's pitch).
    let sp = speed(&hips);
    let accel: Vec<f32> = (0..n / 2).map(|k| sp[k + 1] - sp[k]).collect();
    let nods: Vec<f32> = (0..n / 2).map(|k| { let p = params_dir(Pose::Crawl, (k as f32 + 0.5) / n as f32, 0.0); p.head - 1.0 - p.lean }).collect();
    let top = |v: &[f32]| (0..v.len()).max_by(|a, b| v[*a].total_cmp(&v[*b])).unwrap();
    let (ta, tn) = (top(&accel), top(&nods));
    let (lo, hi) = span(&nods);
    assert!(hi - lo > 0.06, "the head nods only {:.3} rad with the surge", hi - lo);
    assert!(tn > ta && tn - ta <= n / 16, "the head tips back most at {tn}/{n}, the body speeds up most at {ta}/{n}");
}
