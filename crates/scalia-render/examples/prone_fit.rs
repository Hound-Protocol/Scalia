//! Fits the prone poses to the ground (AN-04): finds the body height, boot roll and arm angles that put each boot, each
//! forearm and the belly just on the ground (not in it, not floating), as drawn (the two-handed grip on top), and
//! prints them for pose.rs: `cargo run -q --release -p scalia-render --example prone_fit [5|36]`.
use glam::Vec3;
use scalia_render::mesh::{figure, part_of, Part};
use scalia_render::pose::{grip, params, pose_mesh_vertex, pose_rigid, two_handed, Pose, PoseParams, PALM};

fn lows(p: &PoseParams, verts: &[(Vec3, f32)]) -> [f32; 5] {
    let mut l = [f32::MAX; 5];
    for (at, c) in verts {
        let k = match part_of(*c) {
            Part::LeftFoot | Part::LeftShin => 0,
            Part::RightFoot | Part::RightShin => 1,
            Part::LeftArm | Part::LeftForearm => 2,
            Part::RightArm | Part::RightForearm => 3,
            Part::Torso | Part::Body | Part::LeftThigh | Part::RightThigh => 4,
            _ => continue,
        };
        l[k] = l[k].min(pose_mesh_vertex(*at, *c, p).z);
    }
    l
}


fn main() {
    let id: u32 = std::env::args().nth(1).map_or(5, |s| s.parse().unwrap());
    let pose = Pose::from_id(id);
    let verts: Vec<(Vec3, f32)> = figure().vertices.iter().map(|v| (Vec3::from(v.pos), v.color)).collect();
    // LEAN_HOLD / RAISE_HOLD: how firmly the chest's lean and the body's height are kept at the pose's own.
    let env = |k: &str, d: f32| std::env::var(k).map_or(d, |v| v.parse().unwrap());
    let (lean_hold, raise_hold, toe_hold) = (env("LEAN_HOLD", 1.0), env("RAISE_HOLD", 0.05), env("TOE_HOLD", 0.05));
    let hand = env("HAND", 0.1);
    let mut base = params(pose, 0.0);
    // RAISE / LEAN: start (and, held, stay) from these instead.
    base.raise = env("RAISE", base.raise);
    let toe = env("TOE", f32::NAN);
    if !toe.is_nan() {
        base.toe = [toe; 2];
    }
    let lean = env("LEAN", base.lean);
    base.rifle += lean - base.lean;
    base.head += lean - base.lean;
    base.lean = lean;
    // SH="l,r" / EL="l,r": start the arms from these.
    let pair = |k: &str| std::env::var(k).ok().map(|v| { let a: Vec<f32> = v.split(',').map(|x| x.parse().unwrap()).collect(); [a[0], a[1]] });
    if let Some(v) = pair("SH") { base.shoulder = v; }
    if let Some(v) = pair("EL") { base.elbow = v; }
    // Unknowns: raise, toe L, toe R, right shoulder, right elbow, left shoulder, left elbow, left abduct.
    let set = |x: &[f32; 9]| {
        let mut q = base;
        q.raise = x[0];
        q.toe = [x[1], x[2]];
        q.shoulder = [x[5], x[3]];
        q.elbow = [x[6], x[4]];
        q.abduct[0] = x[7];
        // The chest's lean, the rifle and head taking it back (pose.rs: aim and eyes stay put).
        q.lean = x[8];
        q.rifle = base.rifle + (x[8] - base.lean);
        q.head = base.head + (x[8] - base.lean);
        grip(&q, two_handed(pose))
    };
    let want = [0.004, 0.004, 0.004, 0.004, 0.0];
    let x0 = [base.raise, base.toe[0], base.toe[1], base.shoulder[1], base.elbow[1], base.shoulder[0], base.elbow[0], base.abduct[0], base.lean];
    let res = |x: &[f32; 9]| -> [f32; 15] {
        let l = lows(&set(x), &verts);
        let mut r = [0.0; 15];
        // Boots may hover (one-sided); arms and the belly sit on the ground.
        for k in 0..5 {
            let d = l[k] - want[k];
            r[k] = if k < 2 { d.min(0.0) } else { d } * 10.0;
        }
        for k in 0..9 {
            // The lean held near the pose's own (the chest's height is a look, not a fit); the rest move freely.
            r[5 + k] = (x[k] - x0[k]) * if k == 8 { lean_hold } else if k == 0 { raise_hold } else if k < 3 { toe_hold } else { 0.05 };
        }
        // At rest the rifle lies across in front, low: the right hand (on its grip) held near HAND= (m).
        if pose == Pose::Prone {
            r[14] = (pose_rigid(PALM, Part::RightForearm, &set(x)).z - hand) * 30.0;
        }
        r
    };
    let mut x = x0;
    let cost = |r: &[f32; 15]| r.iter().map(|v| v * v).sum::<f32>();
    for _ in 0..200 {
        let r = res(&x);
        let h = 1e-3;
        let j: Vec<[f32; 15]> = (0..9).map(|k| { let mut y = x; y[k] += h; let ry = res(&y); std::array::from_fn(|m| (ry[m] - r[m]) / h) }).collect();
        let mut a = [[0.0f32; 9]; 9];
        let mut g = [0.0f32; 9];
        for p in 0..9 { for q in 0..9 { a[p][q] = (0..15).map(|m| j[p][m] * j[q][m]).sum::<f32>() + if p == q { 1e-3 } else { 0.0 }; } g[p] = (0..15).map(|m| j[p][m] * r[m]).sum(); }
        for c in 0..9 { for p in 0..9 { if p != c { let k = a[p][c] / a[c][c]; for q in 0..9 { a[p][q] -= k * a[c][q]; } g[p] -= k * g[c]; } } }
        let d: [f32; 9] = std::array::from_fn(|p| (g[p] / a[p][p]).clamp(-0.1, 0.1));
        let mut t = 1.0;
        let c0 = cost(&r);
        loop { let y: [f32; 9] = std::array::from_fn(|p| x[p] - d[p] * t); if cost(&res(&y)) < c0 { x = y; break; } t *= 0.5; if t < 1e-3 { break; } }
    }
    println!("{pose:?} before lows (boots L R, arms L R, body): {:?}", lows(&set(&x0), &verts).map(|v| (v * 1000.0).round() / 1000.0));
    println!("right palm z before {:.3} after {:.3}", pose_rigid(PALM, Part::RightForearm, &set(&x0)).z, pose_rigid(PALM, Part::RightForearm, &set(&x)).z);
    println!("{pose:?} after  lows: {:?}", lows(&set(&x), &verts).map(|v| (v * 1000.0).round() / 1000.0));
    println!("raise {:.4} toe [{:.3}, {:.3}] shoulder [{:.3}, {:.3}] elbow [{:.3}, {:.3}] abductL {:.3} lean {:.3} (was {:.3})", x[0], x[1], x[2], x[5], x[3], x[6], x[4], x[7], x[8], base.lean);
}
