//! Lowest point of every part of a posed figure (`low_parts <pose id> <phase> [dir]`): what touches the ground.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of};
use scalia_render::pose::{params_dir, pose_mesh_vertex, Pose};
use std::collections::BTreeMap;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pose = Pose::from_id(a[1].parse().unwrap());
    let p = params_dir(pose, a[2].parse().unwrap(), a.get(3).map_or(0.0, |s| s.parse().unwrap()));
    let mut low: BTreeMap<String, (f32, Vec3)> = BTreeMap::new();
    for v in &figure().vertices {
        let q = pose_mesh_vertex(Vec3::from(v.pos), v.color, &p);
        let e = low.entry(format!("{:?}", part_of(v.color))).or_insert((f32::MAX, Vec3::ZERO));
        if q.z < e.0 { *e = (q.z, Vec3::from(v.pos)); }
    }
    for (k, (z, rest)) in low { println!("{k:14} {z:+.3}  (rest point {:.2},{:.2},{:.2})", rest.x, rest.y, rest.z); }
}
