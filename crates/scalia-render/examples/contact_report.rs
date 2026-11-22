//! Ground contact and self-clipping over the common poses: `cargo run -q -p scalia-render --example contact_report`.
//! Per pose, over one cycle: the lowest point of each part group (negative = under the ground) and how deep the
//! arms sink into the torso. The torso is measured as stacked ellipses fitted to its rest shape, so the depth is an
//! estimate; the order of the worst poses is what matters.
use scalia_render::contact::contact_stats;
use scalia_render::pose::Pose;

fn main() {
    let poses = [
        Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::CrouchWalk, Pose::StandAim, Pose::CrouchAim,
        Pose::Prone, Pose::ProneAim, Pose::Crawl, Pose::Reload, Pose::Drop,
    ];
    println!("{:10} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8}", "pose", "boot", "knee", "elbow", "belly", "head", "rifle", "arm-in");
    for pose in poses {
        let s = contact_stats(pose, 0.0, 16);
        println!(
            "{:10} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:8.3}  (arm worst at {:.2})",
            format!("{pose:?}"), s.boot, s.knee, s.elbow, s.belly, s.head, s.rifle, s.arm_in, s.arm_at
        );
    }
}
