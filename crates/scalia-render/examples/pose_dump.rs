//! `pose_dump <pose id> <out> [phase] [grip]`: a pose's posed figure (x, y, z, part per vertex; figure frame facing
//! +y), for side/top/back contact sheets. With `grip`, the left hand is put on the handguard as the shader does.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{grip, params, pose_vertex, two_handed, Pose};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let phase: f32 = a.get(3).map_or(0.25, |s| s.parse().unwrap());
    let mut p = params(pose, phase);
    if a.get(4).is_some() { p = grip(&p, two_handed(pose)); }
    let fig = figure();
    let mut out = Vec::new();
    for v in &fig.vertices {
        let part = part_of(v.color);
        let q = pose_vertex(Vec3::from(v.pos), part, &p);
        for f in [q.x, q.y, q.z, part as u32 as f32] { out.extend_from_slice(&f.to_le_bytes()); }
    }
    std::fs::write(&a[2], out).unwrap();
}
