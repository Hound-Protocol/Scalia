//! Dumps a gait cycle's posed figure for scripts/filmstrip.py: `filmstrip_dump <pose id> <frames> <out>`.
//! Per frame, per vertex: x, y, z, colour code (f32 little-endian), figure frame (facing +y).
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{params, pose_vertex, Pose};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = match a[1].as_str() { "walk" => Pose::Walk, "run" => Pose::Run, "sprint" => Pose::Sprint, "stand" => Pose::Stand, "crouch" => Pose::Crouch, "aim" => Pose::StandAim, s => panic!("pose {s}") };
    let frames: usize = a[2].parse().unwrap();
    let fig = figure();
    let mut out = Vec::new();
    for k in 0..frames {
        let p = params(pose, k as f32 / frames as f32);
        for v in &fig.vertices {
            let q = pose_vertex(Vec3::from(v.pos), part_of(v.color), &p);
            for f in [q.x, q.y, q.z, v.color] { out.extend_from_slice(&f.to_le_bytes()); }
        }
    }
    std::fs::write(&a[3], out).unwrap();
}
