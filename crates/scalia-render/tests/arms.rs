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
