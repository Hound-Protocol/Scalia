//! Dumps the full-detail soldier posed as given (pose id, phase), as f32 x, y, z, code per vertex, for previews.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{params, pose_vertex, Pose};
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let phase: f32 = a.get(2).map_or(0.25, |s| s.parse().unwrap());
    let pp = params(pose, phase);
    let mut out = std::io::stdout().lock();
    for v in figure().vertices {
        let q = pose_vertex(Vec3::from(v.pos), part_of(v.color), &pp);
        for f in [q.x, q.y, q.z, v.color] {
            out.write_all(&f.to_le_bytes()).unwrap();
        }
    }
}
