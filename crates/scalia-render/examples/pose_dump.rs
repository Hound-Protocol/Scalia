//! `pose_dump <pose id> <out> [phase] [grip]`: a pose's posed figure (x, y, z, part per vertex; figure frame facing
//! +y), for side/top/back contact sheets. With `grip`, the left hand is put on the handguard as the shader does.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{grip, layer, params, params_dir, pose_vertex, two_handed, Pose};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let phase: f32 = a.get(3).map_or(0.25, |s| s.parse().unwrap());
    // Optional: walking direction in degrees (5th) and an upper-body pose id layered on top (6th).
    let dir: f32 = a.get(5).map_or(0.0, |s| s.parse::<f32>().unwrap().to_radians());
    let mut p = params_dir(pose, phase, dir);
    let mut hold = two_handed(pose);
    if let Some(u) = a.get(6) { let up = Pose::from_id(u.parse().unwrap()); p = layer(&p, &params(up, phase)); hold = two_handed(up); }
    if a.get(4).is_some() { p = grip(&p, hold); }
    let fig = figure();
    let mut out = Vec::new();
    for v in &fig.vertices {
        let part = part_of(v.color);
        let q = pose_vertex(Vec3::from(v.pos), part, &p);
        for f in [q.x, q.y, q.z, part as u32 as f32] { out.extend_from_slice(&f.to_le_bytes()); }
    }
    std::fs::write(&a[2], out).unwrap();
}
