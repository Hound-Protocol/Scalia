//! Dumps posed figures for scripts/stance_sheet.py: `stance_dump <out> <pose id>[+<upper id>][@<phase>[~<dir>]]...`
//! (`dir`: travel direction in radians from straight ahead towards the left, for the gaits and the crawl). Each pose is
//! posed as common.wgsl poses it for drawing (upper-body layer, then the two-handed grip). Per pose, per vertex:
//! x, y, z, colour code (f32 little-endian), figure frame (facing +y).
//! Also `<spec>><spec>%<w>`: a blend from the first to the second, `w` of the way (pose.rs `blend`); and a trailing
//! `^<rad>`: the hips and legs trail the chest by that much (the rifle-first lag, as the shader adds it).
use glam::Vec3;
use scalia_render::mesh::figure;
use scalia_render::pose::{blend, grip, layer, params, params_dir, pose_mesh_vertex, two_handed, Pose, PoseParams};

/// One `<pose>[+<upper>][@<phase>[~<dir>]]` spec posed as the shader draws it.
fn one(spec: &str) -> PoseParams {
    let (spec, when) = spec.split_once('@').map_or((spec, "0"), |(a, b)| (a, b));
    let (phase, dir) = when.split_once('~').map_or_else(|| (when.parse().unwrap(), 0.0), |(a, b)| (a.parse().unwrap(), b.parse().unwrap()));
    let mut it = spec.split('+').map(|s| Pose::from_id(s.parse().unwrap()));
    let legs = it.next().unwrap();
    let upper = it.next();
    let mut p = params_dir(legs, phase, dir);
    if let Some(u) = upper {
        p = layer(&p, &params(u, phase));
    }
    let p = grip(&p, two_handed(upper.unwrap_or(legs)));
    // The drop's left hand plants on the ground, as the shader does (pose.rs `drop_hand`).
    if legs == Pose::Drop { scalia_render::pose::drop_hand(&p, phase) } else { p }
}

/// A full spec: a blend (`a>b%w`) or one pose, then an optional leg lag (`^rad`).
fn spec_params(spec: &str) -> PoseParams {
    let (spec, lag) = spec.split_once('^').map_or((spec, 0.0), |(a, b)| (a, b.parse().unwrap()));
    let mut p = match spec.split_once('>') {
        Some((a, rest)) => {
            let (b, w) = rest.split_once('%').unwrap();
            blend(&one(a), &one(b), w.parse().unwrap())
        }
        None => one(spec),
    };
    if p.prone == 0.0 {
        p.yaw += lag;
    }
    p
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let fig = figure();
    let mut out = Vec::new();
    for spec in &a[2..] {
        let p = spec_params(spec);
        for v in &fig.vertices {
            let q = pose_mesh_vertex(Vec3::from(v.pos), v.color, &p);
            for f in [q.x, q.y, q.z, v.color] {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
    }
    std::fs::write(&a[1], out).unwrap();
}
