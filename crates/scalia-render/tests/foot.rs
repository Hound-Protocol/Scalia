//! Foot roll (Patch: Animations 2): the boots strike on the heel, roll flat, push off from the toe and hang in the
//! swing, without sinking into the ground.
use scalia_render::contact::Body;
use scalia_render::pose::{params_dir, Pose};

#[test]
fn straight_gaits_strike_toe_up_and_push_off_heel_up_without_sinking() {
    let body = Body::new();
    for (pose, up, down) in [(Pose::Walk, 0.12, -0.45), (Pose::Run, 0.08, -0.5), (Pose::Sprint, 0.02, -0.45)] {
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
