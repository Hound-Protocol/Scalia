//! The low crawl, solved (Patch: Animations 3): body flat on the ground, chest just up on the forearms, the rifle
//! cradled across them. In turn one knee draws up to the side along the ground while the opposite forearm reaches
//! ahead, then the leg pushes back to straight while that forearm pulls; the other side does the same half a cycle
//! on. Each phase places the knees and elbows on their marks and solves the joint angles that reach them. The result
//! is baked into `pose::HIGH_CRAWL` (the crawl's table), which both the CPU and the shader read; a test keeps that
//! table equal to `solve`.

use crate::mesh::Part;
use crate::pose::{pose_rigid, PoseParams, ANKLE_Z, ELBOW_X, ELBOW_Z, HIP_X, KNEE_Z, PALM, PRONE_LIFT};
use glam::Vec3;

/// Rows in the baked loop.
pub const ROWS: usize = 32;
/// Fields per row.
pub const F: usize = 21;
/// Fields per row, in this order.
pub const FIELDS: [&str; F] = [
    "hipL", "hipR", "kneeL", "kneeR", "sideL", "sideR", "shL", "shR", "elL", "elR", "abL", "abR", "reachL", "reachR", "lean", "raise", "twist",
    "head", "hipRoll", "rollL", "rollR",
];

/// Distance one crawl cycle moves him (m). `pose::CRAWL_STRIDE` must equal it.
pub const STRIDE: f32 = 0.45;
/// Shoulder joint height (m): the chest just up off the ground on the forearms. The hips lie on the ground.
pub const SHOULDER_H: f32 = 0.24;
/// How far the flat body is lifted (m): the chest pitching up on the forearms tips the lower belly down about the
/// hips; this keeps it on the ground, not in it.
pub const BODY_RAISE: f32 = 0.028;
/// Joint centre heights on the ground (m): knee, elbow, ankle, palm left and right (the right hand on the grip).
pub const KNEE_H: f32 = 0.09;
pub const ELBOW_H: f32 = 0.10;
pub const ANKLE_H: f32 = 0.08;
pub const PALM_H: [f32; 2] = [0.06, 0.10];
/// How far a drawing knee or reaching forearm lifts off the ground as it slides forward (m).
pub const LIFT: f32 = 0.03;
/// The knee's angle out from the straight-back leg (rad, round the hip, along the ground): lying back, drawn up.
pub const KNEE_SWING: [f32; 2] = [0.15, 1.15];
/// The elbow against its shoulder (m): forward when pulled back and when reaching, and out to the side.
pub const ELBOW_PATH: [f32; 3] = [-0.10, 0.22, 0.10];

/// Where a limb is in its stroke at `u` (0..1): (how far drawn up or reached, 0..1; lift). The first half it slides
/// forward (the knee draws up, the forearm reaches), the second it pushes or pulls back.
pub fn stroke(u: f32) -> (f32, f32) {
    let u = u.rem_euclid(1.0);
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    if u < 0.5 {
        let t = u * 2.0;
        (ease(t), LIFT * (std::f32::consts::PI * t).sin())
    } else {
        (1.0 - ease((u - 0.5) * 2.0), 0.0)
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

/// Least squares over `N` angles and `M` misses (numeric Gauss-Newton; a step is taken only as far as it lowers the
/// miss, halving otherwise, so it cannot bounce): moves `x` so `f(x)` nears zero.
fn solve_n<const N: usize, const M: usize>(mut x: [f32; N], f: impl Fn([f32; N]) -> [f32; M]) -> [f32; N] {
    let cost = |r: &[f32; M]| r.iter().map(|v| v * v).sum::<f32>();
    for _ in 0..80 {
        let r = f(x);
        let c0 = cost(&r);
        let h = 1e-3;
        let j: [[f32; M]; N] = std::array::from_fn(|k| {
            let mut y = x;
            y[k] += h;
            let ry = f(y);
            std::array::from_fn(|m| (ry[m] - r[m]) / h)
        });
        // Normal equations with a little damping, by Gaussian elimination.
        let mut a = [[0.0f32; N]; N];
        let mut g = [0.0f32; N];
        for p in 0..N {
            for q in 0..N {
                a[p][q] = (0..M).map(|m| j[p][m] * j[q][m]).sum::<f32>() + if p == q { 1e-4 } else { 0.0 };
            }
            g[p] = (0..M).map(|m| j[p][m] * r[m]).sum();
        }
        for c in 0..N {
            for p in 0..N {
                if p != c {
                    let k = a[p][c] / a[c][c];
                    for q in 0..N {
                        a[p][q] -= k * a[c][q];
                    }
                    g[p] -= k * g[c];
                }
            }
        }
        let d: [f32; N] = std::array::from_fn(|p| g[p] / a[p][p]);
        let mut t = 1.0;
        loop {
            let y: [f32; N] = std::array::from_fn(|p| x[p] - (d[p] * t).clamp(-0.3, 0.3));
            if cost(&f(y)) < c0 {
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

fn solve2(x: [f32; 2], f: impl Fn([f32; 2]) -> [f32; 3]) -> [f32; 2] {
    solve_n(x, f)
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

/// The pose parts every row shares: prone and flat, rifle cradled across the forearms in the right hand.
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
    p.roll = [r[19], r[20]];
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
        let (drawn, lift) = stroke(phase + if i == 0 { 0.0 } else { 0.5 });
        let w = 1.0;
        let hip = hip_at(&p, i);
        // Round the hip along the ground, from lying straight back to drawn up to the side.
        let a = KNEE_SWING[0] + (KNEE_SWING[1] - KNEE_SWING[0]) * drawn;
        let reach = (0.42f32 * 0.42 - (hip.z - KNEE_H).powi(2)).max(0.01).sqrt();
        let want = Vec3::new(hip.x + g * reach * a.sin(), hip.y - reach * a.cos(), KNEE_H + lift);
        // Hip bend, side swing, thigh roll and knee bend together: the knee on its spot, the shin straight back
        // behind it to the toes on the ground (the thigh turns out, the shin does not splay).
        let ah = ANKLE_H + lift;
        let [h, s, ro, kn] = solve_n([r[i], r[4 + i], r[19 + i], r[2 + i]], |x| {
            let mut q = p;
            q.hip[i] = x[0];
            q.side[i] = x[1];
            q.roll[i] = x[2];
            q.knee[i] = x[3];
            let k = knee_at(&q, i) - want;
            let a = ankle_at(&q, i);
            let kk = knee_at(&q, i);
            [k.z * 4.0, k.y * 2.0, k.x * 2.0, (a.z - ah) * 3.0, (a.x - kk.x) * 2.0, (a.y - kk.y + 0.3).max(0.0) * 3.0]
        });
        r[i] = h;
        r[4 + i] = s;
        r[19 + i] = ro;
        r[2 + i] = kn;
        apply(&mut p, r);
        let k = knee_at(&p, i) - want;
        miss[i * 3..i * 3 + 3].copy_from_slice(&[k.x * w, k.y * w, k.z * 2.0 * w]);
        // The forearm opposite the knee: reaching as it draws up, pulling as it pushes.
        let (reached, lift) = stroke(phase + if i == 0 { 0.5 } else { 0.0 });
        let w = 1.0;
        let sh = shoulder_at(&p, i);
        let want = Vec3::new(sh.x + g * ELBOW_PATH[2], sh.y + ELBOW_PATH[0] + (ELBOW_PATH[1] - ELBOW_PATH[0]) * reached, ELBOW_H + lift);
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
        // The forearm lies forward on the ground (the right hand a little up, on the rifle's grip).
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

/// Solves the row at `phase`, starting from `prev` (the previous row) so the angles run on continuously: the body
/// flat (no raise), the chest pitched so the shoulders sit at SHOULDER_H, then the limbs on their marks.
pub fn solve_row(phase: f32, prev: Option<&[f32; F]>) -> [f32; F] {
    let mut r = prev.copied().unwrap_or([0.3, 0.3, 0.3, 0.3, -0.3, 0.3, 1.4, 1.4, 1.6, 1.6, 0.2, 0.2, 0.0, 0.0, -0.2, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0]);
    r[15] = BODY_RAISE;
    r[16] = 0.0;
    r[18] = 0.0;
    let mut p = base();
    for _ in 0..8 {
        apply(&mut p, &r);
        let sh = (shoulder_at(&p, 0) + shoulder_at(&p, 1)) / 2.0;
        let hip = (hip_at(&p, 0) + hip_at(&p, 1)) / 2.0;
        let len = (sh - hip).length();
        let want = ((hip.z - SHOULDER_H) / len).clamp(-0.9, 0.9).asin();
        let now = ((hip.z - sh.z) / len).clamp(-0.9, 0.9).asin();
        r[14] += want - now;
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
