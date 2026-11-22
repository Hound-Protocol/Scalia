//! Left boot through a cycle per gait: pitch (deg, + toe up), heel and toe heights (m):
//! `cargo run -q -p scalia-render --example foot_report`.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params_dir, pose_mesh_vertex, Pose};
fn main() {
    let boot: Vec<_> = figure().vertices.into_iter().filter(|v| part_of(v.color) == Part::LeftFoot).collect();
    let (heel, toe) = boot.iter().fold((Vec3::splat(9.0), Vec3::splat(-9.0)), |(h, t), v| {
        let p = Vec3::from(v.pos);
        (if p.y < h.y { p } else { h }, if p.y > t.y { p } else { t })
    });
    println!("heel {heel:.3} toe {toe:.3}");
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint, Pose::CrouchWalk] {
        let mut line = format!("{:10}", format!("{pose:?}"));
        for k in 0..16 {
            let p = params_dir(pose, k as f32 / 16.0, 0.0);
            let lo = boot.iter().map(|v| pose_mesh_vertex(Vec3::from(v.pos), v.color, &p).z).fold(9.0f32, f32::min);
            let h = boot.iter().filter(|v| v.pos[1] < heel.y + 0.04).map(|v| pose_mesh_vertex(Vec3::from(v.pos), v.color, &p)).fold(Vec3::splat(9.0), |a, b| if b.z < a.z { b } else { a });
            let t = boot.iter().filter(|v| v.pos[1] > toe.y - 0.04).map(|v| pose_mesh_vertex(Vec3::from(v.pos), v.color, &p)).fold(Vec3::splat(9.0), |a, b| if b.z < a.z { b } else { a });
            let pitch = ((t.z - h.z) / (t - h).truncate().length()).atan().to_degrees();
            line += &format!(" {pitch:4.0}/{:.2}", lo);
        }
        println!("{line}");
    }
}
