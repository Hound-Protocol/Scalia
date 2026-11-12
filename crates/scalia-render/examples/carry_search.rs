//! Searches each rifle carry's arm angles (shoulder, elbow, inward reach; rifle angle fixed) so the arms clear the
//! torso, the left hand stays on the handguard and the right hand stays near where the carry had it.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{carry, hand, pose_vertex, Carry, PoseParams, Side, GRIP_Z, SHOULDER_X};

struct Core { bands: Vec<(f32, f32, f32, f32)> }
impl Core {
    fn new(torso: &[Vec3]) -> Core {
        let mut bands = Vec::new();
        for b in 0..14 {
            let z0 = 0.85 + b as f32 * 0.05;
            let pts: Vec<&Vec3> = torso.iter().filter(|p| p.z >= z0 - 0.03 && p.z < z0 + 0.08).collect();
            if pts.len() < 8 { continue; }
            let mut xs: Vec<f32> = pts.iter().map(|p| p.x.abs()).collect();
            let mut ys: Vec<f32> = pts.iter().map(|p| p.y).collect();
            xs.sort_by(f32::total_cmp); ys.sort_by(f32::total_cmp);
            let q = |v: &Vec<f32>, k: f32| v[((v.len() - 1) as f32 * k) as usize];
            bands.push((z0, q(&xs, 0.85), q(&ys, 0.08), q(&ys, 0.92)));
        }
        Core { bands }
    }
    fn inside(&self, p: Vec3) -> bool {
        self.bands.iter().find(|b| p.z >= b.0 && p.z < b.0 + 0.05).is_some_and(|b| {
            let cy = (b.2 + b.3) / 2.0;
            let (ax, ay) = (b.1 * 0.9, (b.3 - b.2) / 2.0 * 0.9);
            (p.x / ax).powi(2) + ((p.y - cy) / ay).powi(2) < 1.0
        })
    }
}

fn handguard(p: &PoseParams) -> Vec3 {
    let h = hand(p, Side::Right);
    let v = Vec3::new(SHOULDER_X, 0.36, 0.80);
    let (s, c) = p.rifle.sin_cos();
    let (dy, dz) = (v.y, v.z - GRIP_Z);
    Vec3::new(v.x, dy * c - dz * s, GRIP_Z + dy * s + dz * c) + Vec3::new(h.x - SHOULDER_X, h.y, h.z - GRIP_Z)
}

fn main() {
    let f = figure();
    let torso: Vec<Vec3> = f.vertices.iter().filter(|v| part_of(v.color) == Part::Torso).map(|v| Vec3::from(v.pos)).collect();
    let core = Core::new(&torso);
    let limbs: Vec<(Part, Vec3)> = f.vertices.iter().map(|v| (part_of(v.color), Vec3::from(v.pos)))
        .filter(|(p, _)| matches!(p, Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm | Part::Rifle)).collect();
    for c in [Carry::SafeHang, Carry::LowReady, Carry::Collapsed, Carry::HighReady, Carry::ReadyUp] {
        let mut p0 = PoseParams::default();
        carry(&mut p0, c);
        let rh0 = hand(&p0, Side::Right);
        let score = |p: &PoseParams| {
            let (mut arm_in, mut arm_n, mut rif_in, mut rif_n) = (0, 0, 0, 0);
            for (part, v) in &limbs {
                let q = pose_vertex(*v, *part, p);
                if *part == Part::Rifle { rif_n += 1; if core.inside(q) { rif_in += 1; } } else { arm_n += 1; if core.inside(q) { arm_in += 1; } }
            }
            let clip = arm_in as f32 / arm_n as f32;
            let rclip = rif_in as f32 / rif_n as f32;
            // The left hand on the rifle, as the tests define it: near the barrel line 0.1-0.35 m ahead of the
            // grip (exactly 0.21 m ahead when aiming).
            let (l, r) = (hand(p, Side::Left), hand(p, Side::Right));
            let dir = Vec3::new(0.0, p.rifle.cos(), p.rifle.sin());
            let t = if matches!(c, Carry::ReadyUp) { 0.21 } else { (l - r).dot(dir).clamp(0.1, 0.35) };
            let d1 = (l - (r + dir * t)).length();
            let _ = handguard;
            let d2 = (hand(p, Side::Right) - rh0).length();
            (10.0 * clip + 6.0 * rclip + 4.0 * d1 + 60.0 * (d1 - 0.03).max(0.0) + 3.0 * d2, clip, rclip, d1, d2)
        };
        let mut best = p0;
        let mut bs = score(&best);
        println!("{c:?} before: score {:.3} clip {:.1}% rifle {:.1}% left-hand off {:.3} m", bs.0, bs.1 * 100.0, bs.2 * 100.0, bs.3);
        let mut seed: u64 = 0x9e3779b97f4a7c15;
        let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed % 10_000) as f32 / 10_000.0 - 0.5 };
        for it in 0..6000 {
            let step = if it < 3000 { 0.25 } else { 0.08 };
            let mut q = best;
            q.shoulder[0] += rnd() * step; q.shoulder[1] += rnd() * step;
            q.elbow[0] += rnd() * step; q.elbow[1] += rnd() * step;
            q.reach[0] += rnd() * step; q.reach[1] += rnd() * step;
            q.elbow[0] = q.elbow[0].clamp(-0.1, 2.4); q.elbow[1] = q.elbow[1].clamp(-0.1, 2.4);
            q.reach[0] = q.reach[0].clamp(-0.2, 1.3); q.reach[1] = q.reach[1].clamp(-0.2, 1.3);
            let s = score(&q);
            if s.0 < bs.0 { best = q; bs = s; }
        }
        println!("{c:?} after:  score {:.3} clip {:.1}% rifle {:.1}% left-hand off {:.3} m right moved {:.3} m", bs.0, bs.1 * 100.0, bs.2 * 100.0, bs.3, bs.4);
        println!("  ([{:.2}, {:.2}], [{:.2}, {:.2}], [{:.2}, {:.2}], {:.2})", best.shoulder[0], best.shoulder[1], best.elbow[0], best.elbow[1], best.reach[0], best.reach[1], best.rifle);
    }
}
