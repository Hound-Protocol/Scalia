use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params, pose_vertex, Pose};

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
        if matches!(part, Part::LeftThigh | Part::RightThigh | Part::LeftShin | Part::RightShin | Part::Torso | Part::Head) {
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
    let a = posed(Pose::Walk, 0.25);
    let b = posed(Pose::Walk, 0.75);
    let foot = |v: &[(Part, Vec3)]| v.iter().filter(|p| p.0 == Part::LeftShin).map(|p| p.1.y).fold(f32::MIN, f32::max);
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
    assert!(pp.reach[0] > 0.3, "support arm does not reach across: {:?}", pp.reach);
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
    assert!(run.rifle < -0.7, "running holds it in collapsed low ready, muzzle down");
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
    use scalia_render::pose::{hand, Side};
    for pose in [Pose::StandAim, Pose::CoverAim] {
        let pp = params(pose, 0.0);
        let (l, r) = (hand(&pp, Side::Left), hand(&pp, Side::Right));
        // The left hand sits on the rifle just in front of the magazine: 0.21 m ahead of the right hand along it.
        let along = Vec3::new(0.0, pp.rifle.cos(), pp.rifle.sin());
        let grip = r + along * 0.21;
        assert!((l - grip).length() < 0.06, "{pose:?}: left hand {l:?} is {:.3} m from the rifle at {grip:?}", (l - grip).length());
        // Elbows low: below the hands (motion guide rule 4).
        assert!(r.z > 1.0 || pose == Pose::CoverAim, "{pose:?}: rifle too low {r:?}");
    }
}

#[test]
fn soldiers_can_walk_while_aiming() {
    use scalia_render::pose::{hand, layer, Side};
    // Legs from the walk, everything above the hips from the aim (motion guide rule 1).
    for k in 0..8 {
        let phase = k as f32 / 8.0;
        let pp = layer(&params(Pose::Walk, phase), &params(Pose::StandAim, phase));
        let walk = params(Pose::Walk, phase);
        assert_eq!((pp.hip, pp.knee, pp.drop), (walk.hip, walk.knee, walk.drop), "legs keep walking");
        let (l, r) = (hand(&pp, Side::Left), hand(&pp, Side::Right));
        assert!((l - (r + Vec3::new(0.0, 0.21, 0.0))).length() < 0.06, "both hands stay on the rifle");
        let low = figure().vertices.iter().map(|v| pose_vertex(Vec3::from(v.pos), part_of(v.color), &pp).z).fold(f32::MAX, f32::min);
        assert!((-0.04..=0.06).contains(&low), "feet on the ground at phase {phase}: {low}");
    }
}

#[test]
fn every_carry_holds_the_rifle_in_both_hands() {
    use scalia_render::pose::{hand, Side};
    // The carrying positions from the reference chart: safe hang (stand), low ready (walk, crouch), collapsed
    // low ready (run, sprint), high ready (cover), ready up (aim).
    for (pose, rifle_lo, rifle_hi) in [
        (Pose::Stand, -1.4, -1.0),
        (Pose::Walk, -0.7, -0.3),
        (Pose::Crouch, -0.7, -0.3),
        (Pose::Run, -1.2, -0.8),
        (Pose::Sprint, -1.2, -0.8),
        (Pose::Cover, 0.6, 1.1),
        (Pose::StandAim, -0.1, 0.1),
        (Pose::CoverAim, -0.1, 0.1),
    ] {
        for k in 0..4 {
            let pp = params(pose, k as f32 / 4.0);
            assert!((rifle_lo..=rifle_hi).contains(&pp.rifle), "{pose:?}: rifle angle {}", pp.rifle);
            let (l, r) = (hand(&pp, Side::Left), hand(&pp, Side::Right));
            // The left hand lies on the rifle: near the line from the grip forward along the barrel.
            let dir = Vec3::new(0.0, pp.rifle.cos(), pp.rifle.sin());
            let t = (l - r).dot(dir).clamp(0.1, 0.35);
            let miss = (l - (r + dir * t)).length();
            assert!(miss < 0.05, "{pose:?}: left hand {miss:.3} m off the rifle");
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
        assert!(twist > 0.03 && twist < 0.2, "{pose:?}: twist {twist}");
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
        assert!(p.look.abs() < 0.08);
        assert_eq!(p.rifle, aim.rifle, "the sight picture stays steady");
    }
}
