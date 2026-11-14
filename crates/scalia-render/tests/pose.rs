use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{reach_left, reload_target, blend, grip, grip_weight, two_handed, hand, held, idle, params, pose_vertex, Pose, PoseParams, Side, RIFLE_GUARD};

/// All figure vertices in a pose at a gait phase, in the figure's own frame (facing +y).
fn posed(pose: Pose, phase: f32) -> Vec<(Part, Vec3)> {
    let pp = params(pose, phase);
    figure().vertices.iter().map(|v| (part_of(v.color), pose_vertex(Vec3::from(v.pos), part_of(v.color), &pp))).collect()
}

fn top(v: &[(Part, Vec3)]) -> f32 {
    v.iter().map(|p| p.1.z).fold(f32::MIN, f32::max)
}
fn bottom(v: &[(Part, Vec3)]) -> f32 {
    v.iter().map(|p| p.1.z).fold(f32::MAX, f32::min)
}

#[test]
fn standing_legs_and_body_are_the_mesh_itself() {
    // Standing changes only the arms and rifle (safe hang); legs, torso and head stay as modelled.
    let pp = params(Pose::Stand, 0.0);
    for v in figure().vertices {
        let part = part_of(v.color);
        if matches!(part, Part::LeftThigh | Part::RightThigh | Part::LeftShin | Part::RightShin | Part::LeftFoot | Part::RightFoot | Part::Torso | Part::Head) {
            let p = Vec3::from(v.pos);
            assert!((pose_vertex(p, part, &pp) - p).length() < 1e-5, "{part:?} moved");
        }
    }
}

#[test]
fn feet_stay_on_the_ground_in_every_pose_and_phase() {
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::Cover, Pose::CoverAim, Pose::StandAim] {
        for k in 0..16 {
            let v = posed(pose, k as f32 / 16.0);
            let b = bottom(&v);
            assert!((-0.04..=0.06).contains(&b), "{pose:?} phase {k}: lowest point {b}");
        }
    }
}

#[test]
fn crouching_is_lower_and_prone_is_flat() {
    let stand = top(&posed(Pose::Stand, 0.0));
    let crouch = top(&posed(Pose::Crouch, 0.0));
    let prone = posed(Pose::Prone, 0.0);
    assert!((1.7..=1.85).contains(&stand), "stand {stand}");
    assert!(crouch < 1.5, "crouch (kneeling) {crouch}");
    assert!(top(&prone) < 0.6, "prone height {}", top(&prone));
    assert!(bottom(&prone) > -0.05, "prone sinks: {}", bottom(&prone));
    let len = prone.iter().map(|p| p.1.y).fold(f32::MIN, f32::max) - prone.iter().map(|p| p.1.y).fold(f32::MAX, f32::min);
    assert!(len > 1.5, "lying length {len}");
}

#[test]
fn aiming_points_the_rifle_forward_at_shoulder_height() {
    for (pose, lo, hi) in [(Pose::StandAim, 1.2, 1.55), (Pose::CoverAim, 0.75, 1.25), (Pose::Prone, 0.05, 0.45)] {
        let v = posed(pose, 0.0);
        let rifle: Vec<Vec3> = v.iter().filter(|p| p.0 == Part::Rifle).map(|p| p.1).collect();
        let tip = rifle.iter().fold(Vec3::splat(f32::MIN), |a, b| a.max(*b));
        let low = rifle.iter().fold(Vec3::splat(f32::MAX), |a, b| a.min(*b));
        assert!(tip.y > 0.55, "{pose:?}: rifle reaches only y {}", tip.y);
        assert!(low.z > lo - 0.15 && tip.z < hi + 0.15, "{pose:?}: rifle z {}..{}", low.z, tip.z);
        assert!(tip.z - low.z < 0.4, "{pose:?}: rifle is not level ({}..{})", low.z, tip.z);
    }
}

#[test]
fn walking_swings_the_legs_and_running_leans_forward() {
    // The left foot strikes in front at phase 0 and leaves the ground behind at the end of its stance (0.6).
    let a = posed(Pose::Walk, 0.0);
    let b = posed(Pose::Walk, 0.6);
    let foot = |v: &[(Part, Vec3)]| v.iter().filter(|p| p.0 == Part::LeftFoot).map(|p| p.1.y).fold(f32::MIN, f32::max);
    assert!((foot(&a) - foot(&b)).abs() > 0.3, "left foot barely moves: {} vs {}", foot(&a), foot(&b));
    let head_y = |v: &[(Part, Vec3)]| v.iter().filter(|p| p.0 == Part::Head).map(|p| p.1.y).fold(f32::MIN, f32::max);
    assert!(head_y(&posed(Pose::Run, 0.0)) > head_y(&posed(Pose::Stand, 0.0)) + 0.1);
}

#[test]
fn pose_ids_round_trip() {
    for p in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::Cover, Pose::CoverAim, Pose::Prone, Pose::StandAim] {
        assert_eq!(Pose::from_id(p as u32), p);
    }
    assert_eq!(Pose::from_id(99), Pose::Stand);
}

#[test]
fn blending_starts_at_the_old_pose_and_ends_at_the_new() {
    use scalia_render::pose::blend;
    let (a, b) = (params(Pose::Stand, 0.0), params(Pose::Prone, 0.0));
    assert_eq!(blend(&a, &b, 0.0), a);
    assert_eq!(blend(&a, &b, 1.0), b);
}

#[test]
fn going_prone_never_sinks_into_the_ground() {
    use scalia_render::pose::blend;
    for (from, to) in [(Pose::Stand, Pose::Prone), (Pose::Crouch, Pose::Prone), (Pose::Prone, Pose::Run), (Pose::Walk, Pose::CoverAim)] {
        let (a, b) = (params(from, 0.3), params(to, 0.3));
        for k in 0..=20 {
            let pp = blend(&a, &b, k as f32 / 20.0);
            let low = figure().vertices.iter().map(|v| pose_vertex(Vec3::from(v.pos), part_of(v.color), &pp).z).fold(f32::MAX, f32::min);
            assert!(low > -0.09, "{from:?} -> {to:?} at {k}/20: lowest {low}");
        }
    }
}

#[test]
fn prone_soldier_lifts_his_head_rests_on_his_elbows_and_shoulders_the_rifle() {
    let v = posed(Pose::Prone, 0.0);
    let of = |part: Part| v.iter().filter(move |p| p.0 == part).map(|p| p.1).collect::<Vec<Vec3>>();
    let max = |pts: &[Vec3]| pts.iter().fold(Vec3::splat(f32::MIN), |a, b| a.max(*b));
    let min = |pts: &[Vec3]| pts.iter().fold(Vec3::splat(f32::MAX), |a, b| a.min(*b));
    let (head, rifle) = (of(Part::Head), of(Part::Rifle));
    // Head up and looking forward, not face down in the dirt.
    assert!(max(&head).z > 0.38, "head top only {}", max(&head).z);
    assert!(min(&head).z > 0.03, "face in the ground: {}", min(&head).z);
    // Elbows on the ground, forearms raised to the rifle.
    for arm in [Part::LeftArm, Part::RightArm] {
        assert!(min(&of(arm)).z < 0.1, "{arm:?} elbow floats at {}", min(&of(arm)).z);
    }
    for fore in [Part::LeftForearm, Part::RightForearm] {
        assert!(max(&of(fore)).z > 0.2, "{fore:?} lies flat");
    }
    // Rifle out in front of the head, at head height, butt near the shoulder.
    assert!(max(&rifle).y > max(&head).y + 0.2, "rifle tip {} behind head {}", max(&rifle).y, max(&head).y);
    assert!(min(&rifle).y < max(&head).y, "rifle butt not at the shoulder");
}

#[test]
fn aiming_tucks_the_firing_elbow_and_reaches_across_with_the_other() {
    let pp = params(Pose::StandAim, 0.0);
    // The right (firing) elbow is tucked and bent; the left arm reaches nearly straight to the rifle and swings in.
    assert!(pp.elbow[1] > 1.0, "firing elbow not bent: {:?}", pp.elbow);
    let left = hand(&pp, Side::Left);
    // In front of the chest, well in from the left shoulder (x = -0.238).
    assert!(left.x > -0.238 + 0.12, "support hand does not reach across the body: {left}");
    // Right-handed: the stock sits out at the right shoulder and the rifle points straight ahead once the
    // upper body turns (bladed stance).
    let butt = held(Vec3::new(0.238, -0.171, 0.887), &pp);
    assert!(butt.x > 0.17, "stock not at the right shoulder: {butt}");
    assert!((pp.rifle_yaw + pp.twist).abs() < 0.01, "rifle not pointing ahead: yaw {} twist {}", pp.rifle_yaw, pp.twist);
}


/// Height of the hips (the pelvis) in a pose at a phase.
fn hip_height(pose: Pose, phase: f32) -> f32 {
    let pp = params(pose, phase);
    pose_vertex(Vec3::new(0.0, 0.0, scalia_render::pose::HIP_Z), Part::Torso, &pp).z
}

#[test]
fn walking_has_weight_the_hips_dip_twice_per_stride() {
    // Motion guide rule 2: the stance knee flexes, so the body sinks at mid-step and rises as the legs pass.
    let heights: Vec<f32> = (0..32).map(|k| hip_height(Pose::Walk, k as f32 / 32.0)).collect();
    let (lo, hi) = heights.iter().fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
    assert!(hi - lo > 0.015 && hi - lo < 0.08, "bob {}", hi - lo);
    // Twice per stride: count the local minima around the cycle.
    // Smooth out millimetre ripples (where the lowest point switches between heel and toe), then count.
    let sm: Vec<f32> = (0..32).map(|k| (heights[(k + 30) % 32] + heights[(k + 31) % 32] + heights[k] + heights[(k + 1) % 32] + heights[(k + 2) % 32]) / 5.0).collect();
    let minima = (0..32).filter(|&k| sm[k] < sm[(k + 31) % 32] && sm[k] <= sm[(k + 1) % 32]).count();
    assert_eq!(minima, 2, "{heights:?}");
}

#[test]
fn sprinting_leans_further_and_carries_the_rifle_muzzle_down() {
    let (run, sprint) = (params(Pose::Run, 0.0), params(Pose::Sprint, 0.0));
    assert!(sprint.lean > run.lean, "sprint leans more");
    assert!(sprint.rifle < 0.0, "rifle angled down across the body");
    let walk = params(Pose::Walk, 0.0);
    assert!(walk.rifle < 0.0, "walking carries the rifle at low ready");
    // Combat run (BF3 reference): rifle nearly level across the chest, muzzle a little down; low, knees bent.
    assert!((-0.4..0.0).contains(&run.rifle), "combat run holds the rifle nearly level: {}", run.rifle);
    assert!(run.knee.iter().cloned().fold(f32::MAX, f32::min) > 0.3, "combat run keeps the planted knee bent");
}

#[test]
fn pose_changes_ease_in_and_out() {
    use scalia_render::pose::blend;
    let (a, b) = (params(Pose::Stand, 0.0), params(Pose::Crouch, 0.0));
    // A quarter of the way through the time, an eased blend has covered less than a quarter of the distance.
    let q = blend(&a, &b, 0.25);
    assert!((q.lean - a.lean) / (b.lean - a.lean) < 0.2, "{}", q.lean);
    let h = blend(&a, &b, 0.5);
    assert!(((h.lean - a.lean) / (b.lean - a.lean) - 0.5).abs() < 0.01);
}

#[test]
fn both_hands_hold_the_rifle_when_aiming() {
    use scalia_render::pose::{hand, held, Side, RIFLE_GUARD};
    for pose in [Pose::StandAim, Pose::CoverAim] {
        let pp = params(pose, 0.0);
        let (l, r) = (hand(&pp, Side::Left), hand(&pp, Side::Right));
        // The left palm on the handguard, under the barrel.
        let guard = held(RIFLE_GUARD, &pp);
        assert!((l - guard).length() < 0.05, "{pose:?}: left hand {l:?} is {:.3} m from the handguard at {guard:?}", (l - guard).length());
        // Elbows low: below the hands (motion guide rule 4).
        assert!(r.z > 1.0 || pose == Pose::CoverAim, "{pose:?}: rifle too low {r:?}");
    }
}

#[test]
fn soldiers_can_walk_while_aiming() {
    use scalia_render::pose::{hand, held, layer, Side, RIFLE_GUARD};
    // Legs from the walk, everything above the hips from the aim (motion guide rule 1).
    for k in 0..8 {
        let phase = k as f32 / 8.0;
        let pp = layer(&params(Pose::Walk, phase), &params(Pose::StandAim, phase));
        let walk = params(Pose::Walk, phase);
        assert_eq!((pp.hip, pp.knee, pp.drop), (walk.hip, walk.knee, walk.drop), "legs keep walking");
        let l = hand(&pp, Side::Left);
        assert!((l - held(RIFLE_GUARD, &pp)).length() < 0.05, "both hands stay on the rifle");
        let low = figure().vertices.iter().map(|v| pose_vertex(Vec3::from(v.pos), part_of(v.color), &pp).z).fold(f32::MAX, f32::min);
        assert!((-0.04..=0.06).contains(&low), "feet on the ground at phase {phase}: {low}");
    }
}

#[test]
fn every_carry_holds_the_rifle_in_both_hands() {
    use scalia_render::pose::{hand, held, Side, RIFLE_GUARD};
    // The carrying positions from the reference chart: safe hang (stand), low ready (walk, crouch), collapsed
    // low ready (run, sprint), high ready (cover), ready up (aim).
    for (pose, rifle_lo, rifle_hi) in [
        (Pose::Stand, -1.0, -0.7),
        (Pose::Walk, -0.7, -0.3),
        (Pose::Crouch, -0.7, -0.3),
        // Combat run: nearly level (the stride sways it a little); sprinting: muzzle down at about 40 degrees.
        (Pose::Run, -0.35, -0.05),
        (Pose::Sprint, -0.95, -0.5),
        (Pose::Cover, 0.6, 1.1),
        (Pose::StandAim, -0.1, 0.1),
        (Pose::CoverAim, -0.1, 0.1),
    ] {
        for k in 0..4 {
            let pp = params(pose, k as f32 / 4.0);
            assert!((rifle_lo..=rifle_hi).contains(&pp.rifle), "{pose:?}: rifle angle {}", pp.rifle);
            // The left palm on the handguard as drawn, after `grip` (the right holds the pistol grip by
            // construction).
            let pp = grip(&pp, two_handed(pose));
            let miss = (hand(&pp, Side::Left) - held(RIFLE_GUARD, &pp)).length();
            assert!(miss < 0.05, "{pose:?}: left hand {miss:.3} m off the handguard");
        }
    }
}

#[test]
fn crouching_is_a_kneel_right_knee_down_left_foot_forward() {
    for pose in [Pose::Crouch, Pose::CoverAim] {
        let v = posed(pose, 0.0);
        let of = |p: Part| v.iter().filter(move |q| q.0 == p).map(|q| q.1).collect::<Vec<Vec3>>();
        let low = |pts: &[Vec3]| pts.iter().map(|p| p.z).fold(f32::MAX, f32::min);
        let fwd = |pts: &[Vec3]| pts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        // The right knee (top of the shin) rests on the ground; the left shin stands upright with its foot ahead.
        assert!(low(&of(Part::RightShin)) < 0.08, "{pose:?}: right knee not on the ground");
        assert!(fwd(&of(Part::LeftShin)) > fwd(&of(Part::RightShin)) + 0.3, "{pose:?}: left foot not forward");
        let left = of(Part::LeftShin);
        let top = left.iter().map(|p| p.z).fold(f32::MIN, f32::max);
        assert!(top > 0.4, "{pose:?}: left shin not upright (top {top})");
    }
}

#[test]
fn the_radio_pose_holds_the_handset_to_the_ear() {
    use scalia_render::pose::{hand, Side};
    let pp = params(Pose::Radio, 0.0);
    let l = hand(&pp, Side::Left);
    assert!(l.z > 1.5 && l.x.abs() < 0.2 && l.y.abs() < 0.15, "left hand at {l:?}");
    assert_eq!(Pose::from_id(9), Pose::Radio);
}

#[test]
fn the_tablet_pose_holds_it_in_both_hands_at_the_chest() {
    use scalia_render::pose::{hand, Side, TABLET_AT};
    let pp = params(Pose::Tablet, 0.0);
    assert_eq!(Pose::from_id(10), Pose::Tablet);
    for side in [Side::Left, Side::Right] {
        let h = hand(&pp, side);
        assert!((0.25..0.45).contains(&h.y) && (1.05..1.3).contains(&h.z), "{side:?} hand at {h:?}");
        assert!((0.08..0.3).contains(&h.x.abs()), "{side:?} hand at {h:?}");
        // The tablet sits between the hands.
        assert!((h.y - TABLET_AT.y).abs() < 0.08 && (h.z - TABLET_AT.z).abs() < 0.08, "{side:?} hand {h:?} vs tablet {TABLET_AT:?}");
    }
    assert!(pp.head < -0.2, "looking down at the tablet: {}", pp.head);
}

#[test]
fn the_tablet_pose_slings_the_rifle_across_the_back() {
    let v = posed(Pose::Tablet, 0.0);
    let rifle: Vec<Vec3> = v.iter().filter(|p| p.0 == Part::Rifle).map(|p| p.1).collect();
    assert!(!rifle.is_empty());
    let max_y = rifle.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(max_y < -0.05, "rifle on the back, behind the body: front edge at y = {max_y}");
    let low = rifle.iter().map(|p| p.z).fold(f32::MAX, f32::min);
    assert!(low > 0.3, "slung rifle clear of the ground: {low}");
}

/// Fraction of arm vertices inside the torso's core (an ellipse per 5 cm band of height, from the torso at rest).
fn arm_clip(pp: &scalia_render::pose::PoseParams) -> f32 {
    let f = figure();
    let torso: Vec<Vec3> = f.vertices.iter().filter(|v| part_of(v.color) == Part::Torso).map(|v| Vec3::from(v.pos)).collect();
    let mut bands = Vec::new();
    for b in 0..14 {
        let z0 = 0.85 + b as f32 * 0.05;
        let pts: Vec<&Vec3> = torso.iter().filter(|p| p.z >= z0 - 0.03 && p.z < z0 + 0.08).collect();
        if pts.len() < 8 { continue; }
        let mut xs: Vec<f32> = pts.iter().map(|p| p.x.abs()).collect();
        let mut ys: Vec<f32> = pts.iter().map(|p| p.y).collect();
        xs.sort_by(f32::total_cmp);
        ys.sort_by(f32::total_cmp);
        let q = |v: &Vec<f32>, k: f32| v[((v.len() - 1) as f32 * k) as usize];
        bands.push((z0, q(&xs, 0.85), q(&ys, 0.08), q(&ys, 0.92)));
    }
    let (mut inside, mut n) = (0, 0);
    for v in &f.vertices {
        let part = part_of(v.color);
        if !matches!(part, Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm) { continue; }
        let p = pose_vertex(Vec3::from(v.pos), part, pp);
        n += 1;
        if let Some(b) = bands.iter().find(|b| p.z >= b.0 && p.z < b.0 + 0.05) {
            let cy = (b.2 + b.3) / 2.0;
            let (ax, ay) = (b.1 * 0.9, (b.3 - b.2) / 2.0 * 0.9);
            if (p.x / ax).powi(2) + ((p.y - cy) / ay).powi(2) < 1.0 { inside += 1; }
        }
    }
    inside as f32 / n as f32
}

#[test]
fn arms_clear_the_torso_in_every_carry() {
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Cover, Pose::StandAim] {
        for k in 0..4 {
            let pp = params(pose, k as f32 / 4.0);
            // A little overlap is unavoidable with rigid parts: at the shoulder joint, and the forearms resting
            // against the belly where the rifle hangs at ease.
            let c = arm_clip(&pp);
            assert!(c < 0.10, "{pose:?} phase {k}/4: {:.1}% of the arms inside the torso", c * 100.0);
        }
    }
}

#[test]
fn the_rifle_sways_with_the_stride() {
    use scalia_render::pose::{hand, Side};
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint] {
        let hs: Vec<Vec3> = (0..8).map(|k| hand(&params(pose, k as f32 / 8.0), Side::Right)).collect();
        let rifles: Vec<f32> = (0..8).map(|k| params(pose, k as f32 / 8.0).rifle).collect();
        let spread = hs.iter().map(|h| (*h - hs[0]).length()).fold(0.0f32, f32::max);
        let pitch = rifles.iter().fold(f32::MIN, |a, &b| a.max(b)) - rifles.iter().fold(f32::MAX, |a, &b| a.min(b));
        assert!(spread > 0.02 || pitch > 0.03, "{pose:?}: the rifle stays rigid (hand moves {spread:.3} m, pitch {pitch:.3})");
        assert!(spread < 0.12, "{pose:?}: sways too much ({spread:.3} m)");
    }
}

#[test]
fn the_body_bobs_and_twists_against_the_legs_when_moving() {
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint] {
        let pp: Vec<_> = (0..8).map(|k| params(pose, k as f32 / 8.0)).collect();
        let bob = pp.iter().map(|p| p.bob).fold(f32::MIN, f32::max) - pp.iter().map(|p| p.bob).fold(f32::MAX, f32::min);
        let twist = pp.iter().map(|p| p.twist.abs()).fold(0.0f32, f32::max);
        assert!(bob > 0.01 && bob < 0.08, "{pose:?}: bob {bob}");
        assert!(twist > 0.03 && twist < 0.35, "{pose:?}: twist {twist}");
    }
    let still = params(Pose::Stand, 0.0);
    assert_eq!((still.bob, still.twist), (0.0, 0.0), "no stride, no bob");
}

#[test]
fn standing_soldiers_breathe_shift_and_look_around() {
    use scalia_render::pose::idle;
    let base = params(Pose::Stand, 0.0);
    let at = |t: f32, seed: f32| idle(&base, Pose::Stand, t, seed);
    // Over half a minute the stance keeps changing a little: breathing, weight, glances.
    let samples: Vec<_> = (0..60).map(|k| at(k as f32 * 0.5, 0.3)).collect();
    let range = |f: &dyn Fn(&scalia_render::pose::PoseParams) -> f32| {
        samples.iter().map(f).fold(f32::MIN, f32::max) - samples.iter().map(f).fold(f32::MAX, f32::min)
    };
    assert!(range(&|p| p.shoulder[0]) > 0.02, "breathing moves the shoulders");
    assert!(range(&|p| p.look) > 0.3, "looks around");
    assert!(range(&|p| p.twist) > 0.03, "shifts weight");
    for p in &samples {
        assert!(p.look.abs() < 0.8 && p.twist.abs() < 0.12 && (p.shoulder[0] - base.shoulder[0]).abs() < 0.12, "{p:?}");
    }
    // Different soldiers are out of step with each other.
    assert_ne!(at(5.0, 0.1).look, at(5.0, 0.7).look);
    // Aiming soldiers keep their eyes on the sights.
    let aim = params(Pose::StandAim, 0.0);
    for k in 0..30 {
        let p = idle(&aim, Pose::StandAim, k as f32, 0.5);
        assert!((p.look - aim.look).abs() < 0.08);
        assert_eq!(p.rifle, aim.rifle, "the sight picture stays steady");
    }
}


fn guard_gap(p: &PoseParams) -> f32 {
    hand(p, Side::Left).distance(held(RIFLE_GUARD, p))
}

#[test]
fn grip_keeps_the_left_hand_on_the_handguard_while_idling_and_blending() {
    let stand = params(Pose::Stand, 0.25);
    let worst_idle = (0..3000).map(|i| guard_gap(&grip(&idle(&stand, Pose::Stand, i as f32 * 0.02, 0.37), 1.0))).fold(0.0, f32::max);
    assert!(worst_idle < 0.01, "idle gap {worst_idle}");
    let (a, b) = (params(Pose::Stand, 0.25), params(Pose::Cover, 0.25));
    let worst_blend = (0..=50).map(|i| guard_gap(&grip(&blend(&a, &b, i as f32 / 50.0), 1.0))).fold(0.0, f32::max);
    assert!(worst_blend < 0.05, "blend gap {worst_blend}");
}

#[test]
fn grip_leaves_a_hand_that_is_doing_something_else() {
    for pose in [Pose::Radio, Pose::Tablet] {
        let p = params(pose, 0.25);
        assert_eq!(two_handed(pose), 0.0, "{pose:?}");
        assert_eq!(grip(&p, two_handed(pose)), p, "{pose:?}");
    }
}

#[test]
fn grip_moves_the_arm_and_rifle_no_faster_than_a_blend_allows() {
    let (n, dt) = (70, 0.35 / 70.0);
    for a in 0..=10u32 {
        for b in 0..=10u32 {
            if a == b {
                continue;
            }
            let (ea, eb) = (Pose::from_id(a), Pose::from_id(b));
            let (pa, pb) = (params(ea, 0.25), params(eb, 0.25));
            let v = |w: f32| {
                let q = grip(&blend(&pa, &pb, w), grip_weight(ea, eb, w));
                [q.shoulder[0], q.elbow[0], q.reach[0], q.rifle]
            };
            let mut prev = v(0.0);
            for i in 1..=n {
                let c = v(i as f32 / n as f32);
                for k in 0..4 {
                    let s = (c[k] - prev[k]).abs() / dt;
                    assert!(s <= 15.0, "{ea:?}->{eb:?} joint {k}: {s} rad/s");
                }
                prev = c;
            }
        }
    }
}

#[test]
fn crouched_walk_stays_low_and_holds_the_rifle() {
    assert_eq!(Pose::from_id(11), Pose::CrouchWalk);
    let stand = hip_height(Pose::Stand, 0.0);
    for k in 0..8 {
        let ph = k as f32 / 8.0;
        let h = hip_height(Pose::CrouchWalk, ph);
        assert!(h < stand - 0.2, "phase {ph}: hips at {h}, standing {stand}");
        let pp = grip(&params(Pose::CrouchWalk, ph), two_handed(Pose::CrouchWalk));
        let miss = (hand(&pp, Side::Left) - held(RIFLE_GUARD, &pp)).length();
        assert!(miss < 0.03, "phase {ph}: left hand {miss:.3} m off the handguard");
    }
}

#[test]
fn layer_and_direction_share_one_instance_value() {
    use scalia_render::pose::{pack_layer, unpack_layer};
    for upper in [None, Some(Pose::StandAim), Some(Pose::CrouchWalk)] {
        for dir in [0.0f32, 1.5707964, -1.5707964, 3.1, -3.1, 0.3] {
            let (whole, d) = unpack_layer(pack_layer(upper, dir));
            assert_eq!(whole, upper.map_or(0.0, |p| p as u32 as f32 + 1.0));
            assert!((d - dir).abs() < 1e-3, "{upper:?} {dir} -> {d}");
        }
    }
    // A plain whole number (what instances without a direction carry) means straight ahead.
    assert_eq!(unpack_layer(7.0), (7.0, 0.0));
}

/// Where the head is in a pose (its rest point near the eyes, posed).
fn head_at(pose: Pose) -> Vec3 {
    let pp = params(pose, 0.0);
    pose_vertex(Vec3::new(0.0, 0.05, 1.62), Part::Head, &pp)
}

#[test]
fn leaning_out_moves_the_head_and_rifle_past_the_corner_and_keeps_the_aim() {
    assert_eq!((Pose::from_id(12), Pose::from_id(13), Pose::from_id(14)), (Pose::LeanLeft, Pose::LeanRight, Pose::CrouchAim));
    let aim = head_at(Pose::StandAim);
    let (l, r) = (head_at(Pose::LeanLeft), head_at(Pose::LeanRight));
    assert!(l.x < aim.x - 0.15, "lean left moves the head out to the left: {l} vs {aim}");
    assert!(r.x > aim.x + 0.15, "lean right moves the head out to the right: {r} vs {aim}");
    for pose in [Pose::LeanLeft, Pose::LeanRight, Pose::CrouchAim] {
        let pp = grip(&params(pose, 0.0), two_handed(pose));
        let miss = (hand(&pp, Side::Left) - held(RIFLE_GUARD, &pp)).length();
        assert!(miss < 0.03, "{pose:?}: left hand {miss:.3} m off the handguard");
        assert!((pp.rifle_yaw + pp.twist).abs() < 0.01, "{pose:?}: rifle not pointing ahead");
    }
}

#[test]
fn crouched_aim_pops_up_over_a_low_wall() {
    // Higher than kneeling, lower than standing: eyes over a 1 m wall.
    let (kneel, crouch_aim, stand) = (hip_height(Pose::CoverAim, 0.0), hip_height(Pose::CrouchAim, 0.0), hip_height(Pose::Stand, 0.0));
    assert!(crouch_aim > kneel + 0.1 && crouch_aim < stand - 0.15, "hips {crouch_aim} (kneel {kneel}, stand {stand})");
    assert!(head_at(Pose::CrouchAim).z > 1.15, "eyes clear a 1 m wall: {}", head_at(Pose::CrouchAim).z);
}

#[test]
fn the_stock_sits_in_the_right_shoulder_pocket() {
    // The rifle's rearmost point (rest pose) and the shoulder pocket between chest and deltoid (figure frame,
    // before twist and lean), as examples/carry_ik places them. High ready (cover) carries the stock low under
    // the arm on purpose, so it is not in this list.
    let butt = Vec3::new(0.238, -0.171, 0.887);
    let pocket = Vec3::new(0.17, 0.12, 1.36);
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::StandAim, Pose::CoverAim, Pose::CrouchAim, Pose::LeanLeft, Pose::LeanRight] {
        for k in 0..8 {
            let pp = params(pose, k as f32 / 8.0);
            let off = held(butt, &pp).distance(pocket);
            assert!(off < 0.04, "{pose:?} phase {k}/8: stock {off:.3} m from the shoulder pocket");
        }
    }
}

#[test]
fn close_quarters_and_action_poses_have_ids() {
    let ids: Vec<Pose> = (15..=20).map(Pose::from_id).collect();
    assert_eq!(ids, vec![Pose::Stack, Pose::Sweep, Pose::Throw, Pose::Reload, Pose::Signal, Pose::Drone]);
}

#[test]
fn stacked_at_a_door_is_compact_and_ready() {
    let (stack, stand) = (hip_height(Pose::Stack, 0.0), hip_height(Pose::Stand, 0.0));
    assert!(stack < stand - 0.05 && stack > stand - 0.25, "half crouch: hips {stack} vs {stand}");
    assert!(params(Pose::Stack, 0.0).lean > 0.15, "weight forward, ready to go through");
}

#[test]
fn sweeping_turns_the_aim_left_and_right() {
    let tw: Vec<f32> = (0..16).map(|k| params(Pose::Sweep, k as f32 / 16.0).twist).collect();
    let (lo, hi) = tw.iter().fold((f32::MAX, f32::MIN), |(a, b), &t| (a.min(t), b.max(t)));
    let aim = params(Pose::StandAim, 0.0).twist;
    assert!(lo < aim - 0.35 && hi > aim + 0.35, "sweep range {lo}..{hi} around {aim}");
}

#[test]
fn throwing_winds_up_overhead_and_releases_forward() {
    let p = params(Pose::Throw, 0.35);
    assert_eq!(p.sling, 1.0, "rifle on the back to throw");
    assert!(hand(&p, Side::Right).z > 1.6, "wind-up: hand high {:?}", hand(&p, Side::Right));
    let r = params(Pose::Throw, 0.55);
    assert!(hand(&r, Side::Right).y > 0.3, "release: hand out in front {:?}", hand(&r, Side::Right));
}

#[test]
fn reloading_goes_to_the_pouch_and_back_to_the_handguard() {
    let at = |t: f32| {
        let p = params(Pose::Reload, t);
        let q = reach_left(&p, reload_target(&p, t), 1.0);
        (hand(&q, Side::Left), held(RIFLE_GUARD, &q), q)
    };
    let (h0, g0, _) = at(0.0);
    assert!(h0.distance(g0) < 0.03, "starts on the handguard");
    let (h1, _, q1) = at(0.47);
    assert!(h1.distance(reload_target(&q1, 0.47)) < 0.04 && h1.y < 0.3, "at the pouch: {h1}");
    let (h2, g2, _) = at(1.0);
    assert!(h2.distance(g2) < 0.03, "ends on the handguard");
}

#[test]
fn hand_signal_raises_the_free_hand() {
    assert!(hand(&params(Pose::Signal, 0.25), Side::Left).z > 1.5);
    assert_eq!(two_handed(Pose::Signal), 0.0);
}

#[test]
fn drone_launch_kneels_and_lifts_it_overhead() {
    let p = params(Pose::Drone, 0.5);
    assert!(hand(&p, Side::Left).z > 1.4 && hand(&p, Side::Right).z > 1.4, "both hands up");
    assert!(hip_height(Pose::Drone, 0.5) < 0.7, "kneeling");
    assert_eq!(p.sling, 1.0);
}

#[test]
fn a_fall_ends_exactly_in_the_pose_he_lies_in() {
    use scalia_render::pose::{params, Pose};
    for (fall, lie) in [(Pose::FallBack, Pose::Wounded), (Pose::FallForward, Pose::Dead)] {
        let (a, b) = (params(fall, 1.0), params(lie, 0.0));
        for (x, y) in [(a.prone, b.prone), (a.hip[0], b.hip[0]), (a.hip[1], b.hip[1]), (a.knee[0], b.knee[0]), (a.knee[1], b.knee[1]), (a.shoulder[0], b.shoulder[0]), (a.shoulder[1], b.shoulder[1]), (a.elbow[0], b.elbow[0]), (a.head, b.head), (a.sling, b.sling)] {
            assert!((x - y).abs() < 1e-4, "{fall:?} ends at {x}, {lie:?} starts at {y}");
        }
        // And starts on his feet.
        assert_eq!(params(fall, 0.0).prone, 0.0);
    }
}
