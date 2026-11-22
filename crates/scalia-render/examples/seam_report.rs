//! Worst gap per cut between parts over the common poses (seam debug aid): `cargo run -q -p scalia-render --example seam_report`.
//! The soldier is one mesh cut into parts; a vertex on a cut belongs to two parts and must land in one place.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params, pose_mesh_vertex, Pose};
fn main() {
    let fig = figure();
    let mut seen: Vec<(Vec3, Vec<f32>)> = Vec::new();
    for v in &fig.vertices {
        let at = Vec3::from(v.pos);
        match seen.iter_mut().find(|(p, _)| p.distance(at) < 1e-4) {
            Some((_, cs)) => if !cs.iter().any(|c| part_of(*c) == part_of(v.color)) { cs.push(v.color) },
            None => seen.push((at, vec![v.color])),
        }
    }
    seen.retain(|(_, ps)| ps.len() > 1);
    let mut worst: std::collections::BTreeMap<String, (f32, String)> = Default::default();
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint, Pose::CrouchWalk, Pose::Crouch, Pose::Prone, Pose::Crawl, Pose::StandAim, Pose::CrouchAim, Pose::Drop] {
        for k in 0..8 {
            let pp = params(pose, k as f32 / 8.0);
            for (at, ps) in &seen {
                let posed: Vec<Vec3> = ps.iter().map(|&c| pose_mesh_vertex(*at, c, &pp)).collect();
                let ps: Vec<Part> = ps.iter().map(|&c| part_of(c)).collect();
                let gap = posed.iter().flat_map(|a| posed.iter().map(move |b| a.distance(*b))).fold(0.0f32, f32::max);
                let e = worst.entry(format!("{ps:?}")).or_insert((0.0, String::new()));
                if gap > e.0 { *e = (gap, format!("{pose:?}@{}", k as f32 / 8.0)); }
            }
        }
    }
    for (k, (g, at)) in worst { println!("{k:40} {:.3} m  ({at})", g); }
}
