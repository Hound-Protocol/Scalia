//! Solves each rifle carry's arm angles (shoulder swing, elbow bend, inward turn, outward raise; per arm) for real
//! grip targets: the right hand on the pistol grip, the left on the handguard just under the barrel, each elbow out
//! to the side and below the hands, and the arms clear of the torso. Prints the carry table for pose.rs.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{elbow, hand, held, pose_vertex, PoseParams, Side, RIFLE_GUARD};

struct Target { name: &'static str, grip: Vec3, pitch: f32, along: f32, elbow_r: Vec3, elbow_l: Vec3 }

fn main() {
    let f = figure();
    let torso: Vec<Vec3> = f.vertices.iter().filter(|v| part_of(v.color) == Part::Torso).map(|v| Vec3::from(v.pos)).collect();
    // Torso core per band of height, as in the tests.
    let mut bands = Vec::new();
    for b in 0..14 {
        let z0 = 0.85 + b as f32 * 0.05;
        let pts: Vec<&Vec3> = torso.iter().filter(|p| p.z >= z0 - 0.03 && p.z < z0 + 0.08).collect();
        if pts.len() < 8 { continue; }
        let mut xs: Vec<f32> = pts.iter().map(|p| p.x.abs()).collect();
        let mut ys: Vec<f32> = pts.iter().map(|p| p.y).collect();
        xs.sort_by(f32::total_cmp);
        ys.sort_by(f32::total_cmp);
        let q = |v: &Vec<f32>, k: f32| v[((v.len() - 1) as f32 * k) as usize];
        bands.push((z0, q(&xs, 0.85), q(&ys, 0.08), q(&ys, 0.92)));
    }
    let inside = |p: Vec3| bands.iter().find(|b| p.z >= b.0 && p.z < b.0 + 0.05).is_some_and(|b| {
        let cy = (b.2 + b.3) / 2.0;
        (p.x / (b.1 * 0.9)).powi(2) + ((p.y - cy) / ((b.3 - b.2) / 2.0 * 0.9)).powi(2) < 1.0
    });
    let arms: Vec<(Part, Vec3)> = f.vertices.iter().map(|v| (part_of(v.color), Vec3::from(v.pos)))
        .filter(|(p, _)| matches!(p, Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm)).step_by(2).collect();
    // The stock's butt in the rest pose (the rifle's rearmost points).
    let butt_pts: Vec<Vec3> = f.vertices.iter().filter(|v| part_of(v.color) == Part::Rifle && v.pos[1] < -0.15).map(|v| Vec3::from(v.pos)).collect();
    let butt = butt_pts.iter().fold(Vec3::ZERO, |a, b| a + *b) / butt_pts.len().max(1) as f32;
    println!("butt (rest) {butt:.3}");
    // A right-handed hold: the grip and the whole rifle on the right side of the chest (x about 0.15, the
    // rifle cannot turn, only pitch), the stock against the chest or in the shoulder pocket, and the left
    // (support) arm reaching across to the handguard.
    let targets = [
        Target { name: "SafeHang", grip: Vec3::new(0.13, 0.24, 1.02), pitch: -1.2, along: 0.14, elbow_r: Vec3::new(0.27, 0.0, 1.08), elbow_l: Vec3::new(-0.2, 0.10, 1.06) },
        Target { name: "LowReady", grip: Vec3::new(0.15, 0.31, 1.16), pitch: -0.5, along: 0.16, elbow_r: Vec3::new(0.29, 0.04, 1.12), elbow_l: Vec3::new(-0.10, 0.28, 1.06) },
        Target { name: "Collapsed", grip: Vec3::new(0.15, 0.29, 1.16), pitch: -0.75, along: 0.15, elbow_r: Vec3::new(0.29, 0.03, 1.12), elbow_l: Vec3::new(-0.11, 0.25, 1.06) },
        Target { name: "HighReady", grip: Vec3::new(0.15, 0.35, 1.14), pitch: 0.85, along: 0.21, elbow_r: Vec3::new(0.30, 0.08, 1.06), elbow_l: Vec3::new(-0.06, 0.30, 1.02) },
        Target { name: "Rush", grip: Vec3::new(0.14, 0.24, 1.22), pitch: -0.2, along: 0.16, elbow_r: Vec3::new(0.29, 0.04, 1.12), elbow_l: Vec3::new(-0.10, 0.28, 1.08) },
        Target { name: "ReadyUp", grip: Vec3::new(0.15, 0.33, 1.29), pitch: 0.0, along: 0.21, elbow_r: Vec3::new(0.32, 0.12, 1.16), elbow_l: Vec3::new(-0.02, 0.34, 1.14) },
    ];
    for t in &targets {
        let score = |p: &PoseParams| {
            let (rh, lh) = (hand(p, Side::Right), hand(p, Side::Left));
            let (re, le) = (elbow(p, Side::Right), elbow(p, Side::Left));
            let clip = arms.iter().filter(|(part, v)| inside(pose_vertex(*v, *part, p))).count() as f32 / arms.len() as f32;
            let b = held(butt, p);
            let butt_in = if inside(b) { 20.0 } else { 0.0 };
            // Stock out on the right: in the shoulder pocket for the aim, by the right chest for carries.
            let butt_right = (0.17 - b.x).max(0.0);
            butt_in + 20.0 * butt_right + 30.0 * (rh - t.grip).length() + 80.0 * (lh - held(RIFLE_GUARD, p)).length()
                + 3.0 * (re - t.elbow_r).length() + 3.0 * (le - t.elbow_l).length() + 30.0 * clip + 0.5 * p.rifle_yaw.abs() + 4.0 * ((0.45 - p.elbow[0]).max(0.0) + (0.45 - p.elbow[1]).max(0.0))
        };
        let mut best = PoseParams { rifle: t.pitch, rifle_yaw: 0.3, shoulder: [0.8, 0.8], elbow: [1.2, 1.2], ..Default::default() };
        let mut bs = score(&best);
        let mut seed: u64 = 0x2545f4914f6cdd1d;
        let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed % 100_000) as f32 / 100_000.0 - 0.5 };
        for it in 0..160_000 {
            let step = if it < 50_000 { 0.4 } else if it < 120_000 { 0.1 } else { 0.02 };
            let mut q = best;
            for i in 0..2 {
                q.shoulder[i] = (q.shoulder[i] + rnd() * step).clamp(-0.5, 2.6);
                q.elbow[i] = (q.elbow[i] + rnd() * step).clamp(0.0, 2.5);
                q.reach[i] = (q.reach[i] + rnd() * step).clamp(-0.6, 1.4);
                q.abduct[i] = (q.abduct[i] + rnd() * step).clamp(-0.2, 1.2);
            }
            q.rifle_yaw = (q.rifle_yaw + rnd() * step).clamp(0.0, 0.75);
            let s = score(&q);
            if s < bs { best = q; bs = s; }
        }
        let (rh, lh) = (hand(&best, Side::Right), hand(&best, Side::Left));
        let b = held(butt, &best);
        let clip = arms.iter().filter(|(part, v)| inside(pose_vertex(*v, *part, &best))).count();
        println!("{}: score {bs:.3}, grip {rh:.2} (off {:.3}), butt {b:.2} (in torso {}), muzzle {:.2}, left hand off guard {:.3}, arm pts in torso {clip}",
            t.name, (rh - t.grip).length(), inside(b), held(glam::Vec3::new(0.238, 0.654, 0.86), &best), (lh - held(RIFLE_GUARD, &best)).length());
        println!("        Carry::{} => ([{:.2}, {:.2}], [{:.2}, {:.2}], [{:.2}, {:.2}], [{:.2}, {:.2}], {:.2}, {:.2}),", t.name,
            best.shoulder[0], best.shoulder[1], best.elbow[0], best.elbow[1], best.reach[0], best.reach[1], best.abduct[0], best.abduct[1], t.pitch, best.rifle_yaw);
    }
}
