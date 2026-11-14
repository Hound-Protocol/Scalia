//! Dumps posed figures for scripts/stance_sheet.py: `stance_dump <out> <pose id>[+<upper id>]...`. Each pose is
//! posed as common.wgsl poses it for drawing (upper-body layer, then the two-handed grip). Per pose, per vertex:
//! x, y, z, colour code (f32 little-endian), figure frame (facing +y).
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{grip, layer, params, pose_vertex, two_handed, Pose};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let fig = figure();
    let mut out = Vec::new();
    for spec in &a[2..] {
        let mut it = spec.split('+').map(|s| Pose::from_id(s.parse().unwrap()));
        let legs = it.next().unwrap();
        let upper = it.next();
        let mut p = params(legs, 0.0);
        if let Some(u) = upper {
            p = layer(&p, &params(u, 0.0));
        }
        let p = grip(&p, two_handed(upper.unwrap_or(legs)));
        for v in &fig.vertices {
            let q = pose_vertex(Vec3::from(v.pos), part_of(v.color), &p);
            for f in [q.x, q.y, q.z, v.color] {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
    }
    std::fs::write(&a[1], out).unwrap();
}
