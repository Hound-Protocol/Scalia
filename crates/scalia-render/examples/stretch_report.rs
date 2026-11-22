//! Worst triangle stretch (posed edge / rest edge) on the arm parts per pose: `cargo run -q -p scalia-render --example stretch_report`.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params, pose_mesh_vertex, Pose};
fn main() {
    let fig = figure();
    let arm = |c: f32| matches!(part_of(c), Part::LeftArm | Part::RightArm);
    for pose in [Pose::Walk, Pose::Run, Pose::StandAim, Pose::Prone, Pose::Crawl] {
        let mut worst = (0.0f32, 0.0f32, 0.0f32, Vec3::ZERO, [0.0f32;2]);
        for k in 0..16 {
            let ph = k as f32 / 16.0;
            let pp = params(pose, ph);
            for t in fig.vertices.chunks(3).filter(|t| t.iter().any(|v| arm(v.color))) {
                let rest: Vec<Vec3> = t.iter().map(|v| Vec3::from(v.pos)).collect();
                let posed: Vec<Vec3> = t.iter().map(|v| pose_mesh_vertex(Vec3::from(v.pos), v.color, &pp)).collect();
                for i in 0..3 {
                    let j = (i + 1) % 3;
                    let r = rest[i].distance(rest[j]);
                    if r < 0.01 { continue; }
                    let s = posed[i].distance(posed[j]) / r;
                    if s > worst.0 { worst = (s, ph, r, rest[i], pp.shoulder); }
                }
            }
        }
        println!("  at rest {:?} shoulder {:?}", worst.3, worst.4);
        println!("{pose:?}: worst edge stretch x{:.2} at phase {:.2} (rest edge {:.3} m)", worst.0, worst.1, worst.2);
    }
}
