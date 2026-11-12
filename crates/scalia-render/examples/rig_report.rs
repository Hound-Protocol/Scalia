//! Where the converted soldier's joints and rifle really are in the rest pose, against the rig's constants.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
fn main() {
    let f = figure();
    let pts = |part: Part| -> Vec<Vec3> { f.vertices.iter().filter(|v| part_of(v.color) == part).map(|v| Vec3::from(v.pos)).collect() };
    for part in [Part::RightArm, Part::RightForearm, Part::LeftArm, Part::LeftForearm, Part::Rifle, Part::Torso] {
        let p = pts(part);
        if p.is_empty() { println!("{part:?}: none"); continue; }
        let (mut lo, mut hi) = (Vec3::splat(9.0), Vec3::splat(-9.0));
        for v in &p { lo = lo.min(*v); hi = hi.max(*v); }
        println!("{part:?}: {} verts, min {lo:.3}, max {hi:.3}", p.len());
        if matches!(part, Part::RightForearm | Part::LeftForearm) {
            // the hand: lowest 12% of the forearm's vertices
            let mut s = p.clone(); s.sort_by(|a, b| a.z.total_cmp(&b.z));
            let k = (s.len() as f32 * 0.12) as usize + 1;
            let c = s[..k].iter().copied().sum::<Vec3>() / k as f32;
            println!("   hand centre {c:.3}");
        }
    }
    // Rifle profile: lowest and highest point per 4 cm along its length (y).
    let r = pts(Part::Rifle);
    for b in 0..22 {
        let y0 = -0.18 + b as f32 * 0.04;
        let s: Vec<&Vec3> = r.iter().filter(|v| v.y >= y0 && v.y < y0 + 0.04).collect();
        if s.is_empty() { continue; }
        let lo = s.iter().map(|v| v.z).fold(9.0, f32::min);
        let hi = s.iter().map(|v| v.z).fold(-9.0, f32::max);
        let x = s.iter().map(|v| v.x).sum::<f32>() / s.len() as f32;
        println!("   y {y0:+.2}: z {lo:.3}..{hi:.3}  x~{x:.3}  ({})", s.len());
    }
}
