//! Arms clear of the body (Patch: Animations 2): the support arm never sinks into the torso in an upright pose
//! (`pose::clear_arms` swings it out), and the rifle arm, tied to the stock in the shoulder, is no worse than its
//! braced carry (forearm against the vest).
use scalia_render::contact::contact_stats;
use scalia_render::pose::Pose;

#[test]
fn the_support_arm_stays_out_of_the_torso() {
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::CrouchWalk, Pose::StandAim, Pose::CrouchAim, Pose::Reload] {
        for dir in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
            let s = contact_stats(pose, dir, 16);
            assert!(s.arm_side[0] < 0.015, "{pose:?} dir {dir:.2}: the support arm sinks {:.3} m into the torso", s.arm_side[0]);
            assert!(s.arm_side[1] < 0.045, "{pose:?} dir {dir:.2}: the rifle arm sinks {:.3} m into the torso", s.arm_side[1]);
        }
    }
}

/// Worst edge stretch (posed length over rest length) over a pose's cycle: of the arm triangles (`arm`) or of the
/// torso triangles blended toward an arm (the armpit band).
fn shoulder_stretch(pose: scalia_render::pose::Pose, arm_side: bool) -> f32 {
    use glam::Vec3;
    use scalia_render::mesh::{figure, part_of, Part};
    use scalia_render::pose::{params, pose_mesh_vertex, seam_of};
    let fig = figure();
    let arm = |p: Part| matches!(p, Part::LeftArm | Part::RightArm);
    let pick = |c: f32| if arm_side { arm(part_of(c)) } else { part_of(c) == Part::Torso && seam_of(c).is_some_and(|(o, _)| arm(o)) };
    let mut worst = 0.0f32;
    for k in 0..16 {
        let pp = params(pose, k as f32 / 16.0);
        for t in fig.vertices.chunks(3).filter(|t| t.iter().any(|v| pick(v.color))) {
            for i in 0..3 {
                let (a, b) = (&t[i], &t[(i + 1) % 3]);
                let r = Vec3::from(a.pos).distance(Vec3::from(b.pos));
                if r < 0.01 {
                    continue;
                }
                let s = pose_mesh_vertex(Vec3::from(a.pos), a.color, &pp).distance(pose_mesh_vertex(Vec3::from(b.pos), b.color, &pp));
                worst = worst.max(s / r);
            }
        }
    }
    worst
}

/// The upper arm keeps its shape (Patch: Animations 5): no arm triangle is drawn out into a wedge, even with the arm
/// swung overhead in the crawl (it was x5 there); the armpit band of torso beside it stays within 5% of before
/// (the crawl's body roll, Patch: Animations 5, and its lifted reach, AN-04, open it a little more).
#[test]
fn upper_arms_keep_their_shape() {
    use scalia_render::pose::Pose;
    let mut over = Vec::new();
    for (pose, band) in [(Pose::Walk, 2.3), (Pose::Run, 2.3), (Pose::StandAim, 2.7), (Pose::Prone, 3.8), (Pose::Crawl, 5.6)] {
        let (a, t) = (shoulder_stretch(pose, true), shoulder_stretch(pose, false));
        if a > 2.0 {
            over.push(format!("{pose:?}: upper arm stretched x{a:.2} > x2.0"));
        }
        if t > band {
            over.push(format!("{pose:?}: armpit band stretched x{t:.2} > x{band}"));
        }
    }
    assert!(over.is_empty(), "{over:?}");
}
