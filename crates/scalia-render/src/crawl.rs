//! The high crawl, solved: on elbows and knees with the body held off the ground, the rifle cradled across the
//! forearms. Each phase places the knees and elbows on the ground along a stroke (opposite elbow and knee together)
//! and solves the joint angles that reach them. The result is baked into `pose::HIGH_CRAWL`, which both the CPU and
//! the shader read; a test keeps that table equal to `solve`.

use crate::mesh::Part;
use crate::pose::{pose_rigid, PoseParams, ANKLE_Z, ELBOW_X, ELBOW_Z, HIP_X, KNEE_Z, PALM, PRONE_LIFT};
use glam::Vec3;

/// Rows in the baked loop.
pub const ROWS: usize = 32;
/// Fields per row.
pub const F: usize = 19;
/// Fields per row, in this order.
pub const FIELDS: [&str; F] = [
    "hipL", "hipR", "kneeL", "kneeR", "sideL", "sideR", "shL", "shR", "elL", "elR", "abL", "abR", "reachL", "reachR", "lean", "raise", "twist",
    "head", "hipRoll",
];

/// Distance one crawl cycle moves him (m). `pose::CRAWL_STRIDE` must equal it.
pub const STRIDE: f32 = 0.4;
/// Share of a limb's stroke spent planted.
pub const DUTY: f32 = 0.6;
/// Hip joint height (m): the body off the ground.
pub const HIP_H: f32 = 0.40;
/// Shoulder joint height (m): lower than the hips, chest down over the elbows.
pub const SHOULDER_H: f32 = 0.32;
/// Joint centre heights when planted (m): knee, elbow, ankle, palm (the toes and fingers touch).
pub const KNEE_H: f32 = 0.06;
pub const ELBOW_H: f32 = 0.075;
pub const ANKLE_H: f32 = 0.05;
pub const PALM_H: [f32; 2] = [0.06, 0.13];
/// How high a swinging knee and elbow lift (m).
pub const LIFT: f32 = 0.05;
/// Where each stroke is centred relative to its hip or shoulder (m, forward) and how far out it sits (m, sideways).
pub const KNEE_AT: [f32; 2] = [0.0, 0.21];
pub const ELBOW_AT: [f32; 2] = [0.05, 0.12];

/// Where a limb is in its stroke at `u` (0..1): (forward offset, lift). Planted it slides back as the body moves on;
/// in the air it swings forward, lifted most at the middle, leaving and landing at the planted speed.
pub fn stroke(u: f32) -> (f32, f32) {
    let a = STRIDE * DUTY;
    let u = u.rem_euclid(1.0);
    if u < DUTY {
        (a / 2.0 - a * u / DUTY, 0.0)
    } else {
        // Hermite from the back to the front, leaving and landing at the planted speed so the limb is still
        // against the ground at both ends (no skid as it lifts or lands).
        let t = (u - DUTY) / (1.0 - DUTY);
        let m = -a * (1.0 - DUTY) / DUTY;
        let (t2, t3) = (t * t, t * t * t);
        let x = (2.0 * t3 - 3.0 * t2 + 1.0) * (-a / 2.0) + (t3 - 2.0 * t2 + t) * m + (-2.0 * t3 + 3.0 * t2) * (a / 2.0) + (t3 - t2) * m;
        (x, LIFT * (std::f32::consts::PI * t).sin())
    }
}

fn knee_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * HIP_X, 0.0, KNEE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, p)
}
fn ankle_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * HIP_X, 0.0, ANKLE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, p)
}
fn elbow_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * ELBOW_X, 0.0, ELBOW_Z), if i == 0 { Part::LeftForearm } else { Part::RightForearm }, p)
}
fn palm_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * PALM.x, PALM.y, PALM.z), if i == 0 { Part::LeftForearm } else { Part::RightForearm }, p)
}
fn hip_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * HIP_X, 0.0, crate::pose::HIP_Z), if i == 0 { Part::LeftThigh } else { Part::RightThigh }, p)
}
fn shoulder_at(p: &PoseParams, i: usize) -> Vec3 {
    let g = if i == 0 { -1.0 } else { 1.0 };
    pose_rigid(Vec3::new(g * crate::pose::SHOULDER_X, 0.0, crate::pose::SHOULDER_Z), if i == 0 { Part::LeftArm } else { Part::RightArm }, p)
}

/// Least squares over two angles (numeric Gauss-Newton with a step that must lower the miss): moves `x` so `f(x)` nears zero.
fn solve2(mut x: [f32; 2], f: impl Fn([f32; 2]) -> [f32; 3]) -> [f32; 2] {
    for _ in 0..60 {
        let r = f(x);
        let h = 1e-3;
        let j: [[f32; 3]; 2] = std::array::from_fn(|k| {
            let mut y = x;
            y[k] += h;
            let ry = f(y);
            std::array::from_fn(|m| (ry[m] - r[m]) / h)
        });
        let dot = |a: &[f32; 3], b: &[f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (a, b, c) = (dot(&j[0], &j[0]) + 1e-4, dot(&j[0], &j[1]), dot(&j[1], &j[1]) + 1e-4);
        let (g0, g1) = (dot(&j[0], &r), dot(&j[1], &r));
        let det = a * c - b * b;
        let d0 = (c * g0 - b * g1) / det;
        let d1 = (a * g1 - b * g0) / det;
        // Take the step only as far as it lowers the miss (halving it otherwise), so it cannot bounce.
        let cost = |r: [f32; 3]| r[0] * r[0] + r[1] * r[1] + r[2] * r[2];
        let c0 = cost(r);
        let mut t = 1.0;
        loop {
            let y = [x[0] - (d0 * t).clamp(-0.3, 0.3), x[1] - (d1 * t).clamp(-0.3, 0.3)];
            if cost(f(y)) < c0 {
                x = y;
                break;
            }
            t *= 0.5;
            if t < 1e-3 {
                return x;
            }
        }
    }
    x
}

/// One angle by bisection so `f` crosses zero in `[lo, hi]` (`f` must change sign).
fn solve1(lo: f32, hi: f32, f: impl Fn(f32) -> f32) -> f32 {
    let (mut a, mut b) = (lo, hi);
    let fa = f(a);
    for _ in 0..40 {
        let m = (a + b) / 2.0;
        if (f(m) > 0.0) == (fa > 0.0) { a = m } else { b = m }
    }
    (a + b) / 2.0
}

/// The pose parts every row shares: prone, rifle cradled across the forearms in the right hand.
pub fn base() -> PoseParams {
    PoseParams { prone: 1.0, rifle: 0.0, rifle_yaw: std::f32::consts::FRAC_PI_2, ..Default::default() }
}

/// Applies a solved row to a pose.
pub fn apply(p: &mut PoseParams, r: &[f32; F]) {
    p.hip = [r[0], r[1]];
    p.knee = [r[2], r[3]];
    p.side = [r[4], r[5]];
    p.shoulder = [r[6], r[7]];
    p.elbow = [r[8], r[9]];
    p.abduct = [r[10], r[11]];
    p.reach = [r[12], r[13]];
    p.lean = r[14];
    p.raise = r[15];
    p.twist = r[16];
    p.head = r[17];
    p.yaw = r[18];
}

/// Places the four limbs for the body set in `r` (hips, chest, roll): each knee and elbow on its spot (or as near as
/// it reaches), shins down to the toes, forearms forward. Returns the misses, weighted: planted limbs count fully.
fn place_limbs(phase: f32, r: &mut [f32; F]) -> [f32; 12] {
    let mut p = base();
    apply(&mut p, r);
    let mut miss = [0.0; 12];
    for i in 0..2 {
        let g = if i == 0 { -1.0 } else { 1.0 };
        // Each knee strokes with the opposite elbow: left knee and right elbow at `phase`, the others half a cycle on.
        let (along, lift) = stroke(phase + if i == 0 { 0.0 } else { 0.5 });
        let w = 1.0 - 0.7 * lift / LIFT;
        let hip = hip_at(&p, i);
        let want = Vec3::new(hip.x + g * KNEE_AT[1], hip.y + KNEE_AT[0] + along, KNEE_H + lift);
        let [h, s] = solve2([r[i], r[4 + i]], |x| {
            let mut q = p;
            q.hip[i] = x[0];
            q.side[i] = x[1];
            let k = knee_at(&q, i) - want;
            [k.z * 4.0, k.y * 2.0, k.x * 2.0]
        });
        r[i] = h;
        r[4 + i] = s;
        apply(&mut p, r);
        let k = knee_at(&p, i) - want;
        miss[i * 3..i * 3 + 3].copy_from_slice(&[k.x * w, k.y * w, k.z * 2.0 * w]);
        let ah = ANKLE_H + lift;
        r[2 + i] = solve1(0.0, 2.6, |k| {
            let mut q = p;
            q.knee[i] = k;
            ankle_at(&q, i).z - ah
        });
        apply(&mut p, r);
        let (along, lift) = stroke(phase + if i == 0 { 0.5 } else { 0.0 });
        let w = 1.0 - 0.7 * lift / LIFT;
        let sh = shoulder_at(&p, i);
        let want = Vec3::new(sh.x + g * ELBOW_AT[1], sh.y + ELBOW_AT[0] + along, ELBOW_H + lift);
        let [a, b] = solve2([r[6 + i], r[10 + i]], |x| {
            let mut q = p;
            q.shoulder[i] = x[0];
            q.abduct[i] = x[1];
            let e = elbow_at(&q, i) - want;
            [e.z * 4.0, e.y * 2.0, e.x * 2.0]
        });
        r[6 + i] = a;
        r[10 + i] = b;
        apply(&mut p, r);
        let e = elbow_at(&p, i) - want;
        miss[6 + i * 3..9 + i * 3].copy_from_slice(&[e.x * 0.3 * w, e.y * w, e.z * 2.0 * w]);
        // The forearm lies forward on the ground (the right hand lifted on the rifle's grip).
        let ph = PALM_H[i] + lift;
        r[8 + i] = solve1(0.2, 2.9, |e| {
            let mut q = p;
            q.elbow[i] = e;
            palm_at(&q, i).z - ph
        });
        apply(&mut p, r);
    }
    miss
}

/// Solves the row at `phase`, starting from `prev` (the previous row) so the angles run on continuously. The body's
/// height, pitch and rolls (hips and chest) give way so the planted knees and elbows hold their spots.
pub fn solve_row(phase: f32, prev: Option<&[f32; F]>) -> [f32; F] {
    let mut r = prev.copied().unwrap_or([0.9, 0.9, 0.9, 0.9, -0.4, 0.4, 1.2, 1.2, 1.4, 1.4, 0.2, 0.2, 0.0, 0.0, 0.15, 0.25, 0.0, 0.9, 0.0]);
    // Start from the body's set height and pitch: hips at HIP_H, shoulders at SHOULDER_H, no roll.
    r[16] = 0.0;
    r[18] = 0.0;
    let mut p = base();
    for _ in 0..8 {
        apply(&mut p, &r);
        let hip = (hip_at(&p, 0) + hip_at(&p, 1)) / 2.0;
        r[15] += HIP_H - hip.z;
        apply(&mut p, &r);
        let sh = (shoulder_at(&p, 0) + shoulder_at(&p, 1)) / 2.0;
        let hip = (hip_at(&p, 0) + hip_at(&p, 1)) / 2.0;
        let want = ((hip.z - SHOULDER_H) / (sh - hip).length()).clamp(-0.9, 0.9).asin();
        let now = ((hip.z - sh.z) / (sh - hip).length()).asin();
        r[14] += want - now;
    }
    let rest = [r[15], r[18], r[14], r[16]];
    // Body freedoms: raise, hip roll (yaw), chest pitch (lean), chest roll (twist); each kept near its rest value.
    const BODY: [usize; 4] = [15, 18, 14, 16];
    const KEEP: [f32; 4] = [0.6, 0.5, 1.5, 1.2];
    let cost = |r: &mut [f32; F]| -> [f32; 16] {
        let m = place_limbs(phase, r);
        let mut c = [0.0; 16];
        c[..12].copy_from_slice(&m);
        for k in 0..4 {
            c[12 + k] = (r[BODY[k]] - rest[k]) * KEEP[k];
        }
        c
    };
    for _ in 0..8 {
        let c0 = cost(&mut r);
        let h = 2e-3;
        let mut j = [[0.0f32; 16]; 4];
        for k in 0..4 {
            let mut t = r;
            t[BODY[k]] += h;
            let c = cost(&mut t);
            for m in 0..16 {
                j[k][m] = (c[m] - c0[m]) / h;
            }
        }
        // Normal equations, 4x4, solved by Gaussian elimination with a little damping.
        let mut a = [[0.0f32; 5]; 4];
        for x in 0..4 {
            for y in 0..4 {
                a[x][y] = (0..16).map(|m| j[x][m] * j[y][m]).sum::<f32>() + if x == y { 1e-3 } else { 0.0 };
            }
            a[x][4] = (0..16).map(|m| j[x][m] * c0[m]).sum::<f32>();
        }
        for col in 0..4 {
            let piv = a[col][col];
            for x in 0..4 {
                if x != col {
                    let f = a[x][col] / piv;
                    for y in col..5 {
                        a[x][y] -= f * a[col][y];
                    }
                }
            }
        }
        for k in 0..4 {
            r[BODY[k]] -= (a[k][4] / a[k][k]).clamp(-0.05, 0.05);
        }
    }
    place_limbs(phase, &mut r);
    // Eyes ahead: the head takes back the chest's pitch.
    r[17] = 1.0 + r[14];
    r
}

/// The whole loop: solved twice round so the first row starts from the last, then the body's height, pitch and
/// rolls are smoothed round the loop (no jolt as a limb lands) and the limbs placed again on the smoothed body.
pub fn solve() -> Vec<[f32; F]> {
    let mut rows: Vec<[f32; F]> = Vec::with_capacity(ROWS);
    let mut prev: Option<[f32; F]> = None;
    for _ in 0..2 {
        rows.clear();
        for k in 0..ROWS {
            let r = solve_row(k as f32 / ROWS as f32, prev.as_ref());
            prev = Some(r);
            rows.push(r);
        }
    }
    for _ in 0..3 {
        let old = rows.clone();
        for k in 0..ROWS {
            for f in [14, 15, 16, 18] {
                let at = |d: isize| old[(k as isize + d).rem_euclid(ROWS as isize) as usize][f];
                rows[k][f] = (at(-2) + 4.0 * at(-1) + 6.0 * at(0) + 4.0 * at(1) + at(2)) / 16.0;
            }
        }
    }
    for k in 0..ROWS {
        let mut r = rows[k];
        place_limbs(k as f32 / ROWS as f32, &mut r);
        r[17] = 1.0 + r[14];
        rows[k] = r;
    }
    rows
}

/// Sanity: the hips' lift at full prone (`pose::PRONE_LIFT`), so `raise` reads as height above lying flat.
pub const _LIFT: f32 = PRONE_LIFT;
