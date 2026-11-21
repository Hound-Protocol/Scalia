//! Solves pose parameters frame by frame so our rig's limbs point where a reference clip's do (AN-02-S12):
//! `cargo run -p scalia-render --example fit_clip -- <targets.txt> <pose id> <groups>`.
//! Targets come from `scripts/clip_ref.py` (unit segment directions in the figure frame). Groups, comma
//! separated: `legs` (hip, knee, side), `torso` (lean, twist, tilt), `arms` (shoulder, elbow, reach, abduct), `arms2` (shoulder, elbow only),
//! `head` (nod, turn), `bob` (hip height), `height` (gait body lift, see below), `yaw` (pelvis turn; env YAW_SCALE keeps a share of it), `grip` (keep the rifle within the left hand's reach), `plant` (a second pass that holds the boots the reference has on
//! the ground to an even sweep along the travel). Each sample starts from `params(pose, phase)` and from the
//! previous sample's answer, so the loop stays smooth. Prints one row per sample, in the clip table order
//! `CLIP_FIELDS`, then the fit error per segment.
use glam::Vec3;
use scalia_render::mesh::Part;
use scalia_render::pose::{grip, HEEL_Y, TOE_Y, hand, held, params_dir, pose_vertex, Side, RIFLE_GUARD, Pose, PoseParams, ANKLE_Z, ELBOW_X, ELBOW_Z, HIP_X, HIP_Z, KNEE_Z, NECK_Z, PALM, SHOULDER_X, SHOULDER_Z};

const LEG: f32 = HIP_Z - ANKLE_Z;

fn get(p: &PoseParams, k: usize) -> f32 {
    match k {
        0 => p.hip[0], 1 => p.hip[1], 2 => p.knee[0], 3 => p.knee[1], 4 => p.side[0], 5 => p.side[1],
        6 => p.lean, 7 => p.twist, 8 => p.tilt, 9 => p.head, 10 => p.look,
        11 => p.shoulder[0], 12 => p.shoulder[1], 13 => p.elbow[0], 14 => p.elbow[1],
        15 => p.reach[0], 16 => p.reach[1], 17 => p.abduct[0], 18 => p.abduct[1], 19 => p.bob, _ => p.yaw,
    }
}
fn set(p: &mut PoseParams, k: usize, v: f32) {
    match k {
        0 => p.hip[0] = v, 1 => p.hip[1] = v, 2 => p.knee[0] = v, 3 => p.knee[1] = v, 4 => p.side[0] = v, 5 => p.side[1] = v,
        6 => p.lean = v, 7 => p.twist = v, 8 => p.tilt = v, 9 => p.head = v, 10 => p.look = v,
        11 => p.shoulder[0] = v, 12 => p.shoulder[1] = v, 13 => p.elbow[0] = v, 14 => p.elbow[1] = v,
        15 => p.reach[0] = v, 16 => p.reach[1] = v, 17 => p.abduct[0] = v, 18 => p.abduct[1] = v, 19 => p.bob = v, _ => p.yaw = v,
    }
}
const BOUNDS: [(f32, f32); 21] = [(-1.2, 2.2), (-1.2, 2.2), (0.0, 2.6), (0.0, 2.6), (-1.7, 1.7), (-1.7, 1.7),
    (-0.8, 1.2), (-1.0, 1.0), (-0.6, 0.6), (-1.0, 1.4), (-1.0, 1.0),
    (-1.0, 3.3), (-1.0, 3.3), (0.0, 2.6), (0.0, 2.6), (-0.6, 1.6), (-0.6, 1.6), (-0.6, 1.6), (-0.6, 1.6), (-1.0, 1.0), (-1.6, 1.6)];

fn dir(a: Vec3, b: Vec3) -> Vec3 { (b - a).normalize_or_zero() }
/// Our segment directions in the order `scripts/clip_ref.py` writes them, feet left out: the 12th is the
/// pelvis line (the reference's 14th).
fn segments(p: &PoseParams) -> [Vec3; 12] {
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
    let pelvis = dir(v(-HIP_X, 0.0, HIP_Z, Part::LeftThigh), v(HIP_X, 0.0, HIP_Z, Part::RightThigh));
    [tl, sl, tr, sr, spine, line, ul, fl, ur, fr, head, pelvis]
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

fn r_len_ok(rows: &[Vec<f32>]) -> bool { rows.first().is_some_and(|r| r.len() >= 49) }

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
    let mean_h = rows.iter().map(|r| r[42]).sum::<f32>() / n as f32;
    // `yaw` fits the pelvis turn, then keeps YAW_SCALE of it (1 = as the reference) before the legs are fitted.
    // ANKLE_W: weight of the ankle position term in the leg fit (0 turns it off).
    let ankle_w: f32 = std::env::var("ANKLE_W").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
    let yaw_scale: f32 = std::env::var("YAW_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let hold = groups.contains(&"grip");
    // `plant` (gaits): a second pass pins each boot that the reference has on the ground to a straight, even sweep
    // back along the travel, so the planted foot does not skate once the game matches the cadence to the speed.
    let travel = Vec3::new(head.get(3).map_or(0.0, |v| v.parse().unwrap()), head.get(4).map_or(0.0, |v| v.parse().unwrap()), 0.0).normalize_or_zero();
    let passes = if groups.contains(&"plant") { 2 } else { 1 };
    // REG: how much each sample keeps to the previous one (radians squared).
    let reg: f32 = std::env::var("REG").ok().and_then(|v| v.parse().ok()).unwrap_or(0.01);
    let plant_w: f32 = std::env::var("PLANT_W").ok().and_then(|v| v.parse().ok()).unwrap_or(4000.0);
    let mut plant: Vec<[Option<Vec3>; 2]> = vec![[None; 2]; n];
    // Where each boot is down in the reference: its ankle within PLANT_TOL (5 %) of the leg length of its lowest.
    let plant_tol: f32 = std::env::var("PLANT_TOL").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
    let ankle_h = |r: &Vec<f32>, i: usize| r[42] + r[45 + 3 * i];
    let down: Vec<[bool; 2]> = (0..n).map(|k| std::array::from_fn(|i| {
        r_len_ok(&rows) && {
            let lo = rows.iter().map(|r| ankle_h(r, i)).fold(f32::MAX, f32::min);
            ankle_h(&rows[k], i) < lo + plant_tol
        }
    })).collect();
    let mut fitted: Vec<PoseParams> = Vec::new();
    let mut err = [[0f32; 2]; 12];
    let mut out = Vec::new();
    // SEEDS=all restores multi-start on every sample (the prone crawls were fitted that way).
    let smooth_branch = std::env::var("SEEDS").map_or(true, |v| v != "all");
    let mut dropped = 0;
    let mut first: Vec<PoseParams> = Vec::new();
    for pass in 0..passes {
    if pass == 1 { first = fitted.clone(); }
    // The second pass carries on from the end of the first, so the loop closes on the same branch.
    let mut prev: Option<PoseParams> = if smooth_branch { fitted.last().copied() } else { None };
    err = [[0f32; 2]; 12];
    out = Vec::new();
    fitted = Vec::new();
    for (k, r) in rows.iter().enumerate() {
        let phase = if once { k as f32 / (n - 1) as f32 } else { k as f32 / n as f32 };
        let base = params_dir(pose, phase, 0.0);
        let mut p = base;
        if let Some(q) = prev { for f in 0..19 { set(&mut p, f, get(&q, f)); } }
        // Ankle offsets from the hip joints over the leg length (absent in older target files).
        let ank: Option<[Vec3; 2]> = (r.len() >= 49).then(|| [Vec3::new(r[43], r[44], r[45]), Vec3::new(r[46], r[47], r[48])]);
        let t: Vec<Vec3> = (0..12).map(|s| { let j = if s == 11 { 13 } else { s }; Vec3::new(r[3 * j], r[3 * j + 1], r[3 * j + 2]) }).collect();
        let anchor = p;
        let pins = std::cell::Cell::new(plant[k]);
        let fit = |p: &mut PoseParams, keys: &[usize], segs: &[(usize, f32)], seeds: &[&[f32]]| {
            let c = |q: &PoseParams| {
                let s = segments(q);
                let mut e: f32 = segs.iter().map(|&(i, w)| w * (1.0 - s[i].dot(t[i]))).sum();
                // The ankle where the reference has it (relative to the hip), so the legs also reach the same
                // height: matching directions alone lets small angle errors add up in the body's rise and fall.
                if let Some(a) = ank {
                    for (i, (px, thigh, shin)) in [(-HIP_X, Part::LeftThigh, Part::LeftShin), (HIP_X, Part::RightThigh, Part::RightShin)].into_iter().enumerate() {
                        if keys.contains(&i) || keys.contains(&(2 + i)) {
                            let rel = (pose_vertex(Vec3::new(px, 0.0, ANKLE_Z), shin, q) - pose_vertex(Vec3::new(px, 0.0, HIP_Z), thigh, q)) / LEG;
                            e += ankle_w * (rel - a[i]).length_squared();
                        }
                    }
                }
                // Planted boot on its straight sweep (second pass of `plant`).
                for (i, (px, shin)) in [(-HIP_X, Part::LeftShin), (HIP_X, Part::RightShin)].into_iter().enumerate() {
                    if let Some(target) = pins.get()[i] {
                        if keys.contains(&i) || keys.contains(&(2 + i)) {
                            let a = pose_vertex(Vec3::new(px, 0.0, ANKLE_Z), shin, q);
                            e += plant_w * Vec3::new(a.x - target.x, a.y - target.y, 0.0).length_squared();
                        }
                    }
                }
                if prev.is_some() { e += keys.iter().map(|&k| reg * (get(q, k) - get(&anchor, k)).powi(2)).sum::<f32>(); }
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
            // Seeds only where there is nothing to carry on from (the first sample): later samples start from the
            // previous answer, so the loop stays on one branch instead of popping between equally good fits.
            let seeds = if prev.is_some() && smooth_branch { &[][..] } else { seeds };
            for seed in seeds {
                let mut q = *p;
                for (i, &k) in keys.iter().enumerate() { set(&mut q, k, seed[i]); }
                solve(&mut q, keys, &c);
                if c(&q) < c(&best) { best = q; }
            }
            *p = best;
        };
        if groups.contains(&"torso") { fit(&mut p, &[6, 7, 8], &[(4, 3.0), (5, 1.0)], &[]); }
        if groups.contains(&"yaw") {
            fit(&mut p, &[20], &[(11, 1.0)], &[]);
            p.yaw *= yaw_scale;
        }
        if groups.contains(&"legs") {
            let seeds: [&[f32]; 4] = [&[0.2, 0.3, 0.0], &[0.2, 1.4, -0.9], &[0.2, 1.4, 0.9], &[0.6, 1.0, 0.0]];
            let before = p;
            fit(&mut p, &[0, 2, 4], &[(0, 2.0), (1, 1.0)], &seeds);
            fit(&mut p, &[1, 3, 5], &[(2, 2.0), (3, 1.0)], &seeds);
            // A pin the leg can only meet by bending into another shape (any angle more than 0.6 rad from the unpinned
            // first-pass fit, or a jump of 0.5 rad from the last sample) is dropped for that sample: a pop
            // mid-stride is worse than a moment of slide.
            if let (Some(q), true) = (prev, pins.get().iter().any(|t| t.is_some())) {
                for (i, keys) in [(0usize, [0usize, 2, 4]), (1, [1, 3, 5])] {
                    let off = |f: usize| first.get(k).is_some_and(|a| (get(&p, f) - get(a, f)).abs() > 0.6);
                    if pins.get()[i].is_some() && keys.iter().any(|&f| (get(&p, f) - get(&q, f)).abs() > 0.5 || off(f)) {
                        let mut left = pins.get();
                        let pin = left[i].take();
                        pins.set(left);
                        let mut r = p;
                        for &f in &keys { set(&mut r, f, first.get(k).map_or(get(&before, f), |a| get(a, f))); }
                        fit(&mut r, &keys, if i == 0 { &[(0, 2.0), (1, 1.0)] } else { &[(2, 2.0), (3, 1.0)] }, &seeds);
                        for &f in &keys { set(&mut p, f, get(&r, f)); }
                        let _ = pin;
                        dropped += 1;
                    }
                }
            }
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
        if groups.contains(&"bob") { p.bob = (r[42] - mean_h) * LEG; }
        let s = segments(&p);
        for i in 0..12 { let d = s[i].dot(t[i]).clamp(-1.0, 1.0).acos().to_degrees(); err[i][0] += d / n as f32; err[i][1] = err[i][1].max(d); }
        out.push((0..21).map(|f| get(&p, f)).collect::<Vec<f32>>());
        prev = Some(p);
        fitted.push(p);
    }
    if pass + 1 < passes {
        let ankle_at = |p: &PoseParams, i: usize| pose_vertex(Vec3::new(if i == 0 { -HIP_X } else { HIP_X }, 0.0, ANKLE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, p);
        let mut sweep = Vec::new();
        for i in 0..2 {
            for k in 0..n {
                if down[k][i] && down[(k + 1) % n][i] {
                    sweep.push(-(ankle_at(&fitted[(k + 1) % n], i) - ankle_at(&fitted[k], i)).dot(travel) * n as f32);
                }
            }
        }
        sweep.sort_by(f32::total_cmp);
        let s = if sweep.is_empty() { 0.0 } else { sweep[sweep.len() / 2] };
        eprintln!("plant: boots sweep {s:.2} m per cycle");
        for i in 0..2 {
            // Each run of down samples (wrapping round the loop) becomes one straight sweep about its middle.
            let start = (0..n).find(|&k| down[k][i] && !down[(k + n - 1) % n][i]);
            let Some(start) = start else { continue };
            let mut k = start;
            loop {
                if down[k][i] && !down[(k + n - 1) % n][i] {
                    let run: Vec<usize> = (0..n).map(|j| (k + j) % n).take_while(|&j| down[j][i]).collect();
                    let mid = (run.len() as f32 - 1.0) / 2.0;
                    let c = run.iter().map(|&j| ankle_at(&fitted[j], i)).sum::<Vec3>() / run.len() as f32;
                    for (m, &j) in run.iter().enumerate() {
                        plant[j][i] = Some(c - travel * s * ((m as f32 - mid) / n as f32));
                    }
                }
                k = (k + 1) % n;
                if k == start { break; }
            }
        }
    }
    }
    if passes > 1 {
        let mut miss = Vec::new();
        for k in 0..n { for i in 0..2 { if let Some(t) = plant[k][i] {
            let a = pose_vertex(Vec3::new(if i == 0 { -HIP_X } else { HIP_X }, 0.0, ANKLE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, &fitted[k]);
            miss.push(Vec3::new(a.x - t.x, a.y - t.y, 0.0).length());
        } } }
        miss.sort_by(f32::total_cmp);
        eprintln!("plant: {dropped} pins dropped to avoid a pop");
        if !miss.is_empty() { eprintln!("plant: {} pinned samples, miss median {:.3} m, worst {:.3} m", miss.len(), miss[miss.len() / 2], miss[miss.len() - 1]); }
    }
    // `height` (gaits): the body lift (field `bob`, AN-02-S13; the game sets drop = max(ground contact, lift), boots
    // flat). While the reference has a boot down, the lift puts that boot on the ground (both, if both are down);
    // in flight it follows the reference hips, never letting a boot touch the ground. So the boot the game stands
    // on is the one the reference plants, and `plant` keeps that boot still.
    if groups.contains(&"height") {
        let bob: Vec<f32> = rows.iter().map(|r| (r[42] - mean_h) * LEG).collect();
        let low = |p: &PoseParams, i: usize| {
            let (px, part) = if i == 0 { (-HIP_X, Part::LeftFoot) } else { (HIP_X, Part::RightFoot) };
            let mut q = *p;
            (q.drop, q.bob, q.flat) = (0.0, 0.0, 1.0);
            [HEEL_Y, TOE_Y].iter().map(|&y| pose_vertex(Vec3::new(px, y, 0.0), part, &q).z).fold(f32::MAX, f32::min)
        };
        let gd: Vec<f32> = fitted.iter().map(|p| -(low(p, 0).min(low(p, 1)))).collect();
        // The reference hips, offset to match the planted boots on average.
        let planted: Vec<Option<f32>> = (0..n).map(|k| {
            let v: Vec<f32> = (0..2).filter(|&i| down[k][i]).map(|i| -low(&fitted[k], i)).collect();
            (!v.is_empty()).then(|| v.iter().copied().fold(f32::MIN, f32::max))
        }).collect();
        let offs: Vec<f32> = (0..n).filter_map(|k| planted[k].map(|d| d - bob[k])).collect();
        let c = if offs.is_empty() { 0.0 } else { offs.iter().sum::<f32>() / offs.len() as f32 };
        let mut flights = 0;
        for k in 0..n {
            out[k][19] = match planted[k] {
                Some(d) => d,
                None => { flights += 1; (c + bob[k]).max(gd[k] + 0.02) }
            };
        }
        eprintln!("height: {flights} of {n} samples in flight");
    }
    for r in &out { println!("{}", r.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>().join(" ")); }
    let names = ["thigh L", "shin L", "thigh R", "shin R", "spine", "shoulders", "upper arm L", "forearm L", "upper arm R", "forearm R", "head", "pelvis"];
    for i in 0..12 { eprintln!("{:12} mean {:5.1} max {:5.1} deg", names[i], err[i][0], err[i][1]); }
}
