//! Solves pose parameters frame by frame so our rig's limbs point where a reference clip's do (AN-02-S12):
//! `cargo run -p scalia-render --example fit_clip -- <targets.txt> <pose id> <groups>`.
//! Targets come from `scripts/clip_ref.py` (unit segment directions in the figure frame). Groups, comma
//! separated: `legs` (hip, knee, side), `torso` (lean, twist, tilt), `arms` (shoulder, elbow, reach, abduct), `arms2` (shoulder, elbow only),
//! `head` (nod, turn), `bob` (hip height), `grip` (keep the rifle within the left hand's reach). Each sample starts from `params(pose, phase)` and from the
//! previous sample's answer, so the loop stays smooth. Prints one row per sample, in the clip table order
//! `CLIP_FIELDS`, then the fit error per segment.
use glam::Vec3;
use scalia_render::mesh::Part;
use scalia_render::pose::{grip, hand, held, params_dir, pose_vertex, Side, RIFLE_GUARD, Pose, PoseParams, ANKLE_Z, ELBOW_X, ELBOW_Z, HIP_X, HIP_Z, KNEE_Z, NECK_Z, PALM, SHOULDER_X, SHOULDER_Z};

const LEG: f32 = HIP_Z - ANKLE_Z;

fn get(p: &PoseParams, k: usize) -> f32 {
    match k {
        0 => p.hip[0], 1 => p.hip[1], 2 => p.knee[0], 3 => p.knee[1], 4 => p.side[0], 5 => p.side[1],
        6 => p.lean, 7 => p.twist, 8 => p.tilt, 9 => p.head, 10 => p.look,
        11 => p.shoulder[0], 12 => p.shoulder[1], 13 => p.elbow[0], 14 => p.elbow[1],
        15 => p.reach[0], 16 => p.reach[1], 17 => p.abduct[0], 18 => p.abduct[1], _ => p.bob,
    }
}
fn set(p: &mut PoseParams, k: usize, v: f32) {
    match k {
        0 => p.hip[0] = v, 1 => p.hip[1] = v, 2 => p.knee[0] = v, 3 => p.knee[1] = v, 4 => p.side[0] = v, 5 => p.side[1] = v,
        6 => p.lean = v, 7 => p.twist = v, 8 => p.tilt = v, 9 => p.head = v, 10 => p.look = v,
        11 => p.shoulder[0] = v, 12 => p.shoulder[1] = v, 13 => p.elbow[0] = v, 14 => p.elbow[1] = v,
        15 => p.reach[0] = v, 16 => p.reach[1] = v, 17 => p.abduct[0] = v, 18 => p.abduct[1] = v, _ => p.bob = v,
    }
}
const BOUNDS: [(f32, f32); 19] = [(-1.2, 2.2), (-1.2, 2.2), (0.0, 2.6), (0.0, 2.6), (-1.7, 1.7), (-1.7, 1.7),
    (-0.8, 1.2), (-1.0, 1.0), (-0.6, 0.6), (-1.0, 1.4), (-1.0, 1.0),
    (-1.0, 3.3), (-1.0, 3.3), (0.0, 2.6), (0.0, 2.6), (-0.6, 1.6), (-0.6, 1.6), (-0.6, 1.6), (-0.6, 1.6)];

fn dir(a: Vec3, b: Vec3) -> Vec3 { (b - a).normalize_or_zero() }
/// Our 11 segment directions, in the order `scripts/clip_ref.py` writes them (feet are not fitted).
fn segments(p: &PoseParams) -> [Vec3; 11] {
    let v = |x: f32, y: f32, z: f32, part| pose_vertex(Vec3::new(x, y, z), part, p);
    let leg = |px: f32, thigh, shin| {
        let (h, k, a) = (v(px, 0.0, HIP_Z, thigh), v(px, 0.0, KNEE_Z, shin), v(px, 0.0, ANKLE_Z, shin));
        (dir(h, k), dir(k, a))
    };
    let arm = |s: f32, upper, fore| {
        let (sh, el) = (v(s * SHOULDER_X, 0.0, SHOULDER_Z, upper), v(s * ELBOW_X, 0.0, ELBOW_Z, upper));
        (dir(sh, el), dir(v(s * ELBOW_X, 0.0, ELBOW_Z, fore), v(s * PALM.x, 0.0, PALM.z, fore)))
    };
    let (tl, sl) = leg(-HIP_X, Part::LeftThigh, Part::LeftShin);
    let (tr, sr) = leg(HIP_X, Part::RightThigh, Part::RightShin);
    let (ul, fl) = arm(-1.0, Part::LeftArm, Part::LeftForearm);
    let (ur, fr) = arm(1.0, Part::RightArm, Part::RightForearm);
    let spine = dir(v(0.0, 0.0, HIP_Z, Part::Torso), v(0.0, 0.0, NECK_Z, Part::Torso));
    let line = dir(v(-SHOULDER_X, 0.0, SHOULDER_Z, Part::Torso), v(SHOULDER_X, 0.0, SHOULDER_Z, Part::Torso));
    let head = dir(v(0.0, 0.0, NECK_Z, Part::Head), v(0.0, 0.0, 1.76, Part::Head));
    [tl, sl, tr, sr, spine, line, ul, fl, ur, fr, head]
}

/// Nelder-Mead over the chosen fields, bounded by a penalty.
fn solve(p: &mut PoseParams, keys: &[usize], cost: &dyn Fn(&PoseParams) -> f32) {
    let n = keys.len();
    let eval = |x: &[f32]| {
        let mut q = *p;
        let mut pen = 0.0;
        for (i, &k) in keys.iter().enumerate() {
            let (lo, hi) = BOUNDS[k];
            pen += (lo - x[i]).max(0.0).powi(2) + (x[i] - hi).max(0.0).powi(2);
            set(&mut q, k, x[i].clamp(lo, hi));
        }
        cost(&q) + 10.0 * pen
    };
    let x0: Vec<f32> = keys.iter().map(|&k| get(p, k)).collect();
    let mut pts: Vec<Vec<f32>> = vec![x0.clone()];
    for i in 0..n { let mut x = x0.clone(); x[i] += 0.15; pts.push(x); }
    let mut f: Vec<f32> = pts.iter().map(|x| eval(x)).collect();
    for _ in 0..400 * n {
        let mut o: Vec<usize> = (0..=n).collect();
        o.sort_by(|&a, &b| f[a].total_cmp(&f[b]));
        pts = o.iter().map(|&i| pts[i].clone()).collect();
        f = o.iter().map(|&i| f[i]).collect();
        if f[n] - f[0] < 1e-9 { break; }
        let c: Vec<f32> = (0..n).map(|j| pts[..n].iter().map(|x| x[j]).sum::<f32>() / n as f32).collect();
        let at = |t: f32| -> Vec<f32> { (0..n).map(|j| c[j] + t * (pts[n][j] - c[j])).collect() };
        let r = at(-1.0); let fr = eval(&r);
        if fr < f[0] {
            let e = at(-2.0); let fe = eval(&e);
            if fe < fr { pts[n] = e; f[n] = fe } else { pts[n] = r; f[n] = fr }
        } else if fr < f[n - 1] { pts[n] = r; f[n] = fr }
        else {
            let k = at(0.5); let fk = eval(&k);
            if fk < f[n] { pts[n] = k; f[n] = fk } else {
                for i in 1..=n { pts[i] = (0..n).map(|j| pts[0][j] + 0.5 * (pts[i][j] - pts[0][j])).collect(); f[i] = eval(&pts[i]); }
            }
        }
    }
    let b = (0..=n).min_by(|&a, &b| f[a].total_cmp(&f[b])).unwrap();
    for (i, &k) in keys.iter().enumerate() { set(p, k, pts[b][i].clamp(BOUNDS[k].0, BOUNDS[k].1)); }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&a[1]).expect("targets");
    let pose = Pose::from_id(a[2].parse().unwrap());
    let groups: Vec<&str> = a[3].split(',').collect();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    let once = head[1] == "once";
    let rows: Vec<Vec<f32>> = lines.map(|l| l.split_whitespace().map(|x| x.parse().unwrap()).collect()).collect();
    let n = rows.len();
    let mean_h = rows.iter().map(|r| r[39]).sum::<f32>() / n as f32;
    let hold = groups.contains(&"grip");
    let mut prev: Option<PoseParams> = None;
    let mut err = [[0f32; 2]; 11];
    let mut out = Vec::new();
    for (k, r) in rows.iter().enumerate() {
        let phase = if once { k as f32 / (n - 1) as f32 } else { k as f32 / n as f32 };
        let base = params_dir(pose, phase, 0.0);
        let mut p = base;
        if let Some(q) = prev { for f in 0..19 { set(&mut p, f, get(&q, f)); } }
        let t: Vec<Vec3> = (0..11).map(|s| Vec3::new(r[3 * s], r[3 * s + 1], r[3 * s + 2])).collect();
        let anchor = p;
        let fit = |p: &mut PoseParams, keys: &[usize], segs: &[(usize, f32)], seeds: &[&[f32]]| {
            let c = |q: &PoseParams| {
                let s = segments(q);
                let mut e: f32 = segs.iter().map(|&(i, w)| w * (1.0 - s[i].dot(t[i]))).sum();
                if prev.is_some() { e += keys.iter().map(|&k| 0.002 * (get(q, k) - get(&anchor, k)).powi(2)).sum::<f32>(); }
                // Knees and ankles stay above the ground (lying down, a pitched hip drives them into it).
                for (px, shin) in [(-HIP_X, Part::LeftShin), (HIP_X, Part::RightShin)] {
                    for (z, floor) in [(KNEE_Z, 0.06), (ANKLE_Z, 0.04)] {
                        let h = pose_vertex(Vec3::new(px, 0.0, z), shin, q).z;
                        e += 20.0 * (floor - h).max(0.0).powi(2);
                    }
                    // Lying down, the drawn-up knee stays low (a low crawl, as tests/pose.rs checks).
                    if q.prone > 0.5 {
                        for z in [KNEE_Z, ANKLE_Z] {
                            let h = pose_vertex(Vec3::new(px, 0.0, z), shin, q).z;
                            e += 40.0 * (h - 0.19).max(0.0).powi(2);
                        }
                    }
                }
                // `grip`: the rifle must stay where the left hand can hold the handguard (two-handed poses).
                if hold && keys.contains(&12) {
                    let g = grip(q, 1.0);
                    e += 50.0 * hand(&g, Side::Left).distance(held(RIFLE_GUARD, &g)).max(0.0).powi(2);
                }
                e
            };
            let mut best = *p;
            solve(&mut best, keys, &c);
            for seed in seeds {
                let mut q = *p;
                for (i, &k) in keys.iter().enumerate() { set(&mut q, k, seed[i]); }
                solve(&mut q, keys, &c);
                if c(&q) < c(&best) { best = q; }
            }
            *p = best;
        };
        if groups.contains(&"torso") { fit(&mut p, &[6, 7, 8], &[(4, 3.0), (5, 1.0)], &[]); }
        if groups.contains(&"legs") {
            let seeds: [&[f32]; 4] = [&[0.2, 0.3, 0.0], &[0.2, 1.4, -0.9], &[0.2, 1.4, 0.9], &[0.6, 1.0, 0.0]];
            fit(&mut p, &[0, 2, 4], &[(0, 2.0), (1, 1.0)], &seeds);
            fit(&mut p, &[1, 3, 5], &[(2, 2.0), (3, 1.0)], &seeds);
        }
        if groups.contains(&"arms") {
            fit(&mut p, &[11, 13, 15, 17], &[(6, 2.0), (7, 1.0)], &[]);
            fit(&mut p, &[12, 14, 16, 18], &[(8, 2.0), (9, 1.0)], &[]);
        }
        if groups.contains(&"arms2") {
            // Shoulder swing and elbow bend only; reach and raise stay as the pose sets them (rifle hold).
            fit(&mut p, &[11, 13], &[(6, 2.0), (7, 1.0)], &[]);
            fit(&mut p, &[12, 14], &[(8, 2.0), (9, 1.0)], &[]);
        }
        if groups.contains(&"head") { fit(&mut p, &[9, 10], &[(10, 1.0)], &[]); }
        if groups.contains(&"bob") { p.bob = (r[39] - mean_h) * LEG; }
        let s = segments(&p);
        for i in 0..11 { let d = s[i].dot(t[i]).clamp(-1.0, 1.0).acos().to_degrees(); err[i][0] += d / n as f32; err[i][1] = err[i][1].max(d); }
        out.push((0..20).map(|f| get(&p, f)).collect::<Vec<f32>>());
        prev = Some(p);
    }
    for r in &out { println!("{}", r.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(" ")); }
    let names = ["thigh L", "shin L", "thigh R", "shin R", "spine", "shoulders", "upper arm L", "forearm L", "upper arm R", "forearm R", "head"];
    for i in 0..11 { eprintln!("{:12} mean {:5.1} max {:5.1} deg", names[i], err[i][0], err[i][1]); }
}
