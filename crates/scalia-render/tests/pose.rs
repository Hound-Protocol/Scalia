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
fn standing_is_the_mesh_itself() {
    let pp = params(Pose::Stand, 0.0);
    for v in figure().vertices {
        let p = Vec3::from(v.pos);
        assert!((pose_vertex(p, part_of(v.color), &pp) - p).length() < 1e-5);
    }
}

#[test]
fn feet_stay_on_the_ground_in_every_pose_and_phase() {
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Crouch, Pose::CoverAim, Pose::StandAim] {
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
    assert!(crouch < 1.35, "crouch {crouch}");
    assert!(top(&prone) < 0.5, "prone height {}", top(&prone));
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
    let head_y = |v: &[(Part, Vec3)]| v.iter().filter(|p| p.0 == Part::Torso && p.1.z > 1.3).map(|p| p.1.y).fold(f32::MIN, f32::max);
    assert!(head_y(&posed(Pose::Run, 0.0)) > head_y(&posed(Pose::Stand, 0.0)) + 0.15);
}

#[test]
fn pose_ids_round_trip() {
    for p in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Crouch, Pose::CoverAim, Pose::Prone, Pose::StandAim] {
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
