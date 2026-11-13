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
    // Each gait at its game speeds: (pose, ground speed m/s, share of the cycle a foot is planted).
    for (pose, speed, stance) in [(Pose::Walk, 1.7f32, 0.6f32), (Pose::Walk, 1.5, 0.6), (Pose::Run, 3.0, 0.45), (Pose::Sprint, 4.6, 0.36), (Pose::Sprint, 3.5, 0.36)] {
        let ps: Vec<PoseParams> = (0..=N).map(|k| params(pose, k as f32 / N as f32)).collect();
        // The game sets the cadence from the speed: cycles per second = speed / stride.
        let hz = speed / stride_m(pose);
        let dt = 1.0 / hz / N as f32;
        let f: Vec<[[Vec3; 2]; 2]> = ps.iter().map(|p| feet(p, &pts)).collect();
        // Foot slide: the left boot (heel strike at phase 0) through the middle of its stance, against the ground.
        let c = |k: usize| (f[k][0][0].y + f[k][0][1].y) * 0.5;
        let (k0, k1) = ((0.08 * N as f32) as usize, ((stance - 0.08) * N as f32) as usize);
        let measured = -(c(k1) - c(k0)) / ((k1 - k0) as f32 / N as f32); // metres per cycle
        let (mut sum, mut max, mut n, mut vs) = (0.0f32, 0.0f32, 0, 0.0f32);
        for k in k0..k1 {
            let v = -(c(k + 1) - c(k)) / dt;
            let s = (v - speed).abs();
            sum += s; max = max.max(s); n += 1; vs += v;
        }
        if (measured - stride_m(pose)).abs() > 0.05 * measured {
            println!("  !! {pose:?}: stride_m says {:.2} m but the legs cover {measured:.2} m per cycle", stride_m(pose));
            bad += 1;
        }
        if sum / n.max(1) as f32 > 0.15 { println!("  !! {pose:?} at {speed} m/s: the planted foot slides {:.2} m/s", sum / n.max(1) as f32); bad += 1; }
        let seam = angles(&ps[0]).iter().zip(angles(&ps[N])).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        let a: Vec<Vec<f32>> = ps.iter().map(angles).collect();
        let mut jerk = 0.0f32;
        for k in 1..N - 1 { for j in 0..a[k].len() { jerk = jerk.max(((a[k + 1][j] - 2.0 * a[k][j] + a[k - 1][j]) / (dt * dt)).abs()); } }
        println!("{:<10} {:>6.1}m/s {:>8.2}m/s {:>8.2}m/s {:>9.3}r {:>9.0}   stride {measured:.2} m/cycle, {hz:.2} cycles/s (foot {:.2} m/s)", format!("{pose:?}"), speed, sum / n.max(1) as f32, max, seam, jerk, vs / n.max(1) as f32);
        if seam > 0.01 { println!("  !! loop seam {seam:.3} rad: the cycle jumps where it restarts"); bad += 1; }
    }
    // Walking any direction while facing ahead (strafe, backpedal): the planted foot against the ground, in 2D.
    println!("\nWalk by direction (facing ahead): planted-foot slide (m/s)");
    for deg in [0.0f32, 45.0, 90.0, 135.0, 180.0, -90.0, -45.0] {
        let dir = deg.to_radians();
        let speed = 1.5f32;
        let mv = Vec3::new(-dir.sin(), dir.cos(), 0.0) * speed;
        let ps: Vec<PoseParams> = (0..=N).map(|k| params_dir(Pose::Walk, k as f32 / N as f32, dir)).collect();
        let hz = speed / stride_m_dir(Pose::Walk, dir);
        let dt = 1.0 / hz / N as f32;
        let f: Vec<[[Vec3; 2]; 2]> = ps.iter().map(|p| feet(p, &pts)).collect();
        let c = |k: usize| (f[k][0][0] + f[k][0][1]) * 0.5;
        let (k0, k1) = ((0.08 * N as f32) as usize, ((0.6 - 0.08) * N as f32) as usize);
        let (mut sum, mut n) = (0.0f32, 0);
        for k in k0..k1 {
            let v = (c(k + 1) - c(k)) / dt + mv; // foot velocity over the ground
            sum += Vec3::new(v.x, v.y, 0.0).length(); n += 1;
        }
        let avg = sum / n as f32;
        println!("  {deg:>6.0} deg  slide {avg:.2}");
        if avg > 0.2 { println!("  !! walking at {deg} deg: the planted foot slides {avg:.2} m/s"); bad += 1; }
    }

    println!("\nLeft palm to the rifle handguard (m, worst over the cycle, after grip()); the right palm holds the grip by construction");
    for pose in [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::CoverAim, Pose::Cover, Pose::StandAim] {
        // Worst over the cycle, as drawn (after grip()).
        let guard = (0..16).map(|k| { let p = grip(&params(pose, k as f32 / 16.0), two_handed(pose)); hand(&p, Side::Left).distance(held(RIFLE_GUARD, &p)) }).fold(0.0, f32::max);
        println!("{:<10} guard {:>6.3}", format!("{pose:?}"), guard);
        if guard > 0.03 { println!("  !! {pose:?}: the left hand is off the handguard"); bad += 1; }
    }
    println!("\nLeft palm to the handguard through idle motion and blends, after grip()");
    let holds = [Pose::Stand, Pose::Walk, Pose::Run, Pose::Sprint, Pose::Crouch, Pose::CoverAim, Pose::Cover, Pose::StandAim];
    let gap = |p: &PoseParams| { let q = grip(p, 1.0); hand(&q, Side::Left).distance(held(RIFLE_GUARD, &q)) };
    for &pose in &holds {
        let m = (0..3000).map(|i| gap(&idle(&params(pose, 0.25), pose, i as f32 * 0.02, 0.37))).fold(0.0, f32::max);
        if m > 0.03 { println!("  !! {pose:?} idle: the left hand leaves the handguard by {m:.3} m"); bad += 1; }
    }
    let mut worst = (0.0f32, Pose::Stand, Pose::Stand);
    for &a in &holds { for &b in &holds { if a == b { continue; }
        let (pa, pb) = (params(a, 0.25), params(b, 0.25));
        for i in 0..=50 { let d = gap(&blend(&pa, &pb, i as f32 / 50.0)); if d > worst.0 { worst = (d, a, b); } }
    } }
    println!("worst blend gap {:.3} m ({:?} -> {:?})", worst.0, worst.1, worst.2);
    if worst.0 > 0.05 { println!("  !! the left hand leaves the handguard during a blend"); bad += 1; }
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
    // A person can whip a limb round at about 15 rad/s; faster than that a pose change reads as a snap.
    if worst.0 > 15.0 { println!("  !! a pose change moves a joint faster than 15 rad/s"); bad += 1; }
    if bad > 0 { std::process::exit(1); }
}
