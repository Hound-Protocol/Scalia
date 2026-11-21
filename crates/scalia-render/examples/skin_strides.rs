//! Clip travel per cycle against the stride the game sets cadence from (AN-03 debug aid).
use scalia_render::pose::{stride_m_dir, Pose};
use scalia_render::skin::{anims, pose_clips};
fn main() {
    let an = anims();
    for (pose, id) in [(Pose::Walk, 1), (Pose::Run, 2), (Pose::Sprint, 7), (Pose::CrouchWalk, 11), (Pose::Crawl, 32)] {
        let (_, names) = pose_clips(id);
        let n = names.len();
        let row: Vec<String> = names.iter().enumerate().map(|(i, nm)| {
            let c = &an.clips[an.clip(nm).unwrap()];
            let dir = if n == 8 { i as f32 * std::f32::consts::TAU / 8.0 } else { i as f32 * std::f32::consts::PI };
            format!("{nm} {:.2}/{:.2}", c.travel.length(), stride_m_dir(pose, dir))
        }).collect();
        println!("{pose:?}: {}", row.join("  "));
    }
}
