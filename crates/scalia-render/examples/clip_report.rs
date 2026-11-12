//! Reports, per pose, how many arm and forearm vertices end up inside the torso's core (clipping).
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{params, pose_vertex, Pose};

fn main() {
    let f = figure();
    // The torso's core at rest: per 5 cm band of height, the half-widths that 90% of its vertices stay within.
    let torso: Vec<Vec3> = f.vertices.iter().filter(|v| part_of(v.color) == Part::Torso).map(|v| Vec3::from(v.pos)).collect();
    let mut bands = Vec::new();
    for b in 0..20 {
        let z0 = 0.85 + b as f32 * 0.05;
        let mut xs: Vec<f32> = torso.iter().filter(|p| p.z >= z0 && p.z < z0 + 0.05).map(|p| p.x.abs()).collect();
        let mut ys: Vec<f32> = torso.iter().filter(|p| p.z >= z0 && p.z < z0 + 0.05).map(|p| p.y).collect();
        if xs.len() < 8 { continue; }
        xs.sort_by(f32::total_cmp); ys.sort_by(f32::total_cmp);
        let q = |v: &Vec<f32>, k: f32| v[((v.len() - 1) as f32 * k) as usize];
        bands.push((z0, q(&xs, 0.8), q(&ys, 0.1), q(&ys, 0.9)));
        println!("band z {z0:.2}: |x| < {:.3}, y {:.3}..{:.3}", q(&xs, 0.8), q(&ys, 0.1), q(&ys, 0.9));
    }
    for id in 0..=10u32 {
        let pose = Pose::from_id(id);
        let mut worst = 0.0f32;
        for k in 0..8 {
            let pp = params(pose, k as f32 / 8.0);
            let (mut inside, mut n) = (0, 0);
            for v in &f.vertices {
                let part = part_of(v.color);
                if !matches!(part, Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm) { continue; }
                let p = pose_vertex(Vec3::from(v.pos), part, &pp);
                n += 1;
                // Torso is not moved by lean here; compare in the leaned frame by undoing it is skipped: lean is small.
                if let Some(b) = bands.iter().find(|b| p.z >= b.0 && p.z < b.0 + 0.05) {
                    if p.x.abs() < b.1 * 0.85 && p.y > b.2 * 0.85 && p.y < b.3 * 0.85 { inside += 1; }
                }
            }
            worst = worst.max(inside as f32 / n as f32);
        }
        println!("{pose:?}: worst {:.1}% of arm vertices inside the torso", worst * 100.0);
    }
}
