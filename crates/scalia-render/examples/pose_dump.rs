//! `pose_dump <pose id> <out>`: a pose's posed figure (x, y, z, part per vertex; figure frame facing +y), for side/top/back contact sheets.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{params, pose_vertex, Pose};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = params(Pose::from_id(a[1].parse().unwrap()), 0.25);
    let fig = figure();
    let mut out = Vec::new();
    for v in &fig.vertices {
        let part = part_of(v.color);
        let q = pose_vertex(Vec3::from(v.pos), part, &p);
        for f in [q.x, q.y, q.z, part as u32 as f32] { out.extend_from_slice(&f.to_le_bytes()); }
    }
    std::fs::write(&a[2], out).unwrap();
}
