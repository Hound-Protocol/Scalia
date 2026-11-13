//! Animation audit: measures what the eye notices, per gait and per carry, so changes to the animation are judged by
//! numbers and not only by screenshots. Prints a table and exits non-zero if a hard limit is broken.
//!   - foot slide: how fast the planted foot moves over the ground (m/s) against the commanded ground speed
//!   - loop seam: the biggest pose parameter jump where a cycle ends and starts again
//!   - jerk: the sharpest change of a joint's angular speed within a cycle
//!   - grip: palm distance to the rifle grip (right) and handguard (left) in carries that hold it with both hands
//!   - blend: the fastest joint movement when changing between any two poses
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::*;

const N: usize = 48;

fn angles(p: &PoseParams) -> Vec<f32> {
    let mut v = vec![p.lean, p.head, p.rifle, p.twist, p.look];
    v.extend(p.hip); v.extend(p.knee); v.extend(p.shoulder); v.extend(p.elbow); v.extend(p.reach); v.extend(p.abduct);
    v
}

/// Heel and toe points of each leg (the shin vertices nearest the rest-pose heel and toe), posed: [leg][heel, toe].
fn feet(p: &PoseParams, pts: &[[(Vec3, Part); 2]; 2]) -> [[Vec3; 2]; 2] {
    let mut o = [[Vec3::ZERO; 2]; 2];
    for i in 0..2 { for j in 0..2 { o[i][j] = pose_vertex(pts[i][j].0, pts[i][j].1, p); } }
    o
}

fn main() {
    let fig = figure();
    let shins: Vec<(Vec3, Part)> = fig.vertices.iter().map(|v| (Vec3::from(v.pos), part_of(v.color))).filter(|(_, p)| matches!(p, Part::LeftFoot | Part::RightFoot)).collect();
    let near = |part: Part, target: Vec3| *shins.iter().filter(|(_, p)| *p == part).min_by(|a, b| a.0.distance(target).total_cmp(&b.0.distance(target))).unwrap();
    let mut pts = [[(Vec3::ZERO, Part::Body); 2]; 2];
    for (i, part, x) in [(0, Part::LeftFoot, -0.1f32), (1, Part::RightFoot, 0.1)] {
        pts[i][0] = near(part, Vec3::new(x, -0.09, 0.03));
        pts[i][1] = near(part, Vec3::new(x, 0.14, 0.03));
    }
    let mut bad = 0;
    println!("{:<10} {:>8} {:>10} {:>10} {:>10} {:>10}", "gait", "speed", "slide avg", "slide max", "seam", "jerk");
    for (pose, hz, speed) in [(Pose::Walk, 0.95f32, 1.7f32), (Pose::Walk, 0.9, 1.4), (Pose::Run, 1.2, 3.0), (Pose::Sprint, 1.5, 4.6), (Pose::Sprint, 1.45, 3.5)] {
        let ps: Vec<PoseParams> = (0..=N).map(|k| params(pose, k as f32 / N as f32)).collect();
        let dt = 1.0 / hz / N as f32;
        // Foot slide: while a foot is the planted one (lowest of the two), it should move backwards at the ground speed.
        let f: Vec<[[Vec3; 2]; 2]> = ps.iter().map(|p| feet(p, &pts)).collect();
        let (mut sum, mut max, mut n, mut vs) = (0.0f32, 0.0f32, 0, 0.0f32);
        for k in 0..N {
            // The planted leg is the one whose lowest point is lower; its boot centre should travel back at ground speed.
            let low = |fr: &[[Vec3; 2]; 2], i: usize| fr[i][0].z.min(fr[i][1].z);
            let i = if low(&f[k], 0) < low(&f[k], 1) { 0 } else { 1 };
            if low(&f[k + 1], i) < low(&f[k + 1], 1 - i) {
                let c = |fr: &[[Vec3; 2]; 2]| (fr[i][0].y + fr[i][1].y) * 0.5;
                let v = -(c(&f[k + 1]) - c(&f[k])) / dt;
                let s = (v - speed).abs();
                sum += s; max = max.max(s); n += 1; vs += v;
            }
        }
        let seam = angles(&ps[0]).iter().zip(angles(&ps[N])).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        let a: Vec<Vec<f32>> = ps.iter().map(angles).collect();
        let mut jerk = 0.0f32;
        for k in 1..N - 1 { for j in 0..a[k].len() { jerk = jerk.max(((a[k + 1][j] - 2.0 * a[k][j] + a[k - 1][j]) / (dt * dt)).abs()); } }
        println!("{:<10} {:>6.1}m/s {:>8.2}m/s {:>8.2}m/s {:>9.3}r {:>9.0}   stride {:.2} m/cycle: no slide at {:.2} cycles/s", format!("{pose:?}"), speed, sum / n.max(1) as f32, max, seam, jerk, vs / n.max(1) as f32 / hz, speed / (vs / n.max(1) as f32 / hz));
        if seam > 0.01 { println!("  !! loop seam {seam:.3} rad: the cycle jumps where it restarts"); bad += 1; }
    }
    println!("\nLeft palm to the rifle handguard (m); the right palm holds the grip by construction");
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::CoverAim, Pose::Cover, Pose::StandAim] {
        let p = params(pose, 0.25);
        let guard = hand(&p, Side::Left).distance(held(RIFLE_GUARD, &p));
        println!("{:<10} guard {:>6.3}", format!("{pose:?}"), guard);
    }
    println!("\nBlend between poses: fastest joint (rad/s) over {BLEND_S}s");
    let ids = 0..=10u32;
    let mut worst = (0.0f32, 0, 0);
    for a in ids.clone() { for b in ids.clone() {
        if a == b { continue; }
        let (pa, pb) = (params(Pose::from_id(a), 0.25), params(Pose::from_id(b), 0.25));
        let mut prev = angles(&blend(&pa, &pb, 0.0));
        for k in 1..=20 {
            let w = k as f32 / 20.0; let cur = angles(&blend(&pa, &pb, w));
            for j in 0..cur.len() { let s = (cur[j] - prev[j]).abs() / (BLEND_S / 20.0); if s > worst.0 { worst = (s, a, b); } }
            prev = cur;
        }
    } }
    println!("worst {:.1} rad/s going {:?} -> {:?}", worst.0, Pose::from_id(worst.1), Pose::from_id(worst.2));
    if bad > 0 { std::process::exit(1); }
}
