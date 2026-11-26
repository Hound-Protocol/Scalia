//! The low crawl, solved (Patch: Animations 3): body flat on the ground, chest just up on the forearms, the rifle
//! cradled across them. In turn one knee draws up to the side along the ground while the opposite forearm reaches
//! ahead, then the leg pushes back to straight off its boot while that hand pulls; the other side does the same half a
//! cycle on. AN-07: the boot and the hand grip, holding their spots on the ground while the body surges and rises over
//! them. Each phase places the boots and hands on their marks and solves the joint angles that reach them. The result
//! is baked into `pose::HIGH_CRAWL` (the crawl's table), which both the CPU and the shader read; a test keeps that
//! table equal to `solve`.

use crate::mesh::Part;
use crate::pose::{pose_rigid, PoseParams, ANKLE_Z, ELBOW_X, ELBOW_Z, HIP_X, KNEE_Z, PALM, PRONE_LIFT};
use glam::Vec3;

/// Rows in the baked loop.
pub const ROWS: usize = 32;
/// Fields per row.
pub const F: usize = 23;
/// Fields per row, in this order.
pub const FIELDS: [&str; F] = [
    "hipL", "hipR", "kneeL", "kneeR", "sideL", "sideR", "shL", "shR", "elL", "elR", "abL", "abR", "reachL", "reachR", "lean", "raise", "twist",
    "head", "hipRoll", "rollL", "rollR", "bend", "look",
];

/// Share of a cycle a side spends drawing its knee up and reaching its forearm ahead; it then pushes and pulls until
/// half way, then rests (leg straight back, forearm drawn in) while the other side works.
pub const DRAW: f32 = 0.18;
/// How far the legs roll with the body away from the drawn knee at full draw (rad, `yaw` lying prone).
pub const ROLL: f32 = 0.26;
/// How far the torso rolls away from the drawn knee at full draw (rad, `twist`): the drawn knee's hip lifts about 6 cm.
/// The torso rolls as one piece, hips and chest together. AN-04: twice Patch 5's.
pub const CHEST_ROLL: f32 = 0.26;
/// How far the spine bends toward the drawn knee at full draw (rad): about 4 cm of swing at the shoulders.
pub const BEND: f32 = 0.14;
/// How far behind the knee's draw the roll and bend come (share of a cycle): they peak as the drawn leg pushes,
/// not as the opposite elbow lands (where the arm has least reach to spare).
pub const ROLL_LAG: f32 = 0.0;
/// Share of the body's roll the head takes back.
pub const HEAD_STEADY: f32 = 0.6;
/// How far a push carries the body (m): the knee and the forearm go back this far against the body while planted.
pub const PUSH: f32 = 0.26;
/// Distance one crawl cycle moves him (m): a push per half cycle, spread over the push's share so a planted knee or
/// forearm holds its place. `pose::CRAWL_STRIDE` must equal it.
pub const STRIDE: f32 = PUSH / (0.5 - DRAW);
/// Shoulder joint height (m): the chest just up off the ground on the forearms. The hips lie on the ground.
pub const SHOULDER_H: f32 = 0.24;
/// How far the flat body is lifted (m): the chest pitching up on the forearms tips the lower belly down about the
/// hips; this keeps it on the ground, not in it.
pub const BODY_RAISE: f32 = 0.028;
/// Joint centre heights on the ground (m): knee, elbow, ankle, palm left and right (the right hand on the rifle).
pub const KNEE_H: f32 = 0.09;
pub const ELBOW_H: f32 = 0.11;
pub const ANKLE_H: f32 = 0.08;
pub const PALM_H: [f32; 2] = [0.06, 0.095];
/// How far a drawing knee or reaching forearm lifts off the ground as it slides forward (m).
pub const LIFT: f32 = 0.03;
/// How far round from straight ahead a reaching elbow plants (rad): nearly in line with its shoulder.
pub const ELBOW_REACH: f32 = 0.12;
/// The rifle along the right forearm: pitched to point ahead along the ground and rolled on its side the way the
/// prone carry is (Patch: Animations 5; rolled the other way, setting off and stopping turned it half over through the
/// ground),
/// held in the right hand (rad). common.wgsl mirrors them.
pub const RIFLE_PITCH: f32 = std::f32::consts::FRAC_PI_2;
pub const RIFLE_ROLL: f32 = std::f32::consts::FRAC_PI_2;

/// How far a forearm pulls the body (m): it is planted half the cycle, so it pulls half a stride (AN-04).
pub const ARM_PULL: f32 = STRIDE * 0.5;
/// How high a reaching forearm's elbow lifts off the ground (m).
pub const ARM_LIFT: f32 = 0.08;

/// Where a forearm is in its stroke at `u` (0..1, `arm_u`): (how far back from its forward mark, lift). AN-04: from
/// landing on its mark it pulls back ARM_PULL over half a cycle at the body's speed (planted), then reaches forward off
/// the ground over the other half, leaving and landing at the ground's speed (no jolt); one forearm is always pulling,
/// neither is ever dragged along with the body.
pub fn arm_stroke(u: f32) -> (f32, f32) {
    let u = u.rem_euclid(1.0);
    if u < 0.5 {
        (ARM_PULL * u / 0.5, 0.0)
    } else {
        let t = (u - 0.5) / 0.5;
        (ARM_PULL * (4.0 * t * t * t - 6.0 * t * t + t + 1.0), ARM_LIFT * (std::f32::consts::PI * t).sin())
    }
}

/// Whether a forearm is planted (pulling) at its stroke `u`.
pub fn arm_planted(u: f32) -> bool {
    u.rem_euclid(1.0) < 0.5
}

/// Forearm `i`'s stroke at `phase`: each lands as the opposite knee finishes drawing up (the right with the left knee).
pub fn arm_u(phase: f32, i: usize) -> f32 {
    phase - DRAW + if i == 0 { 0.5 } else { 0.0 }
}

/// AN-07: how far out from its hip a pushing boot plants (m): small enough that the leg, straight at the end of the
/// push, lies straight back.
pub const BOOT_OUT: f32 = 0.08;
/// AN-07: how far behind its hip a boot plants as its push begins (m, along the way he moves); it ends PUSH further
/// back, the leg all but straight.
pub const BOOT_BACK: f32 = 0.535;
/// AN-07: how far ahead of its shoulder a hand lands (m) and how far out from it (in line); it ends the pull ARM_PULL
/// further back, under the face.
pub const HAND_AHEAD: f32 = 0.52;
pub const PALM_OUT: f32 = 0.0;
/// AN-07: how far a straight leg lies turned out (rad of thigh roll): the boot on its inside edge.
pub const LEG_TURN: f32 = 0.9;
/// AN-07: how much the solver resists moving a joint from the previous row (per rad, against misses per m): no joint
/// jumps between rows.
pub const STEADY: f32 = 0.02;
/// AN-07: the surge. The body moves fastest while a leg pushes and slowest while it draws up: its speed is the
/// crawl's average times 1 - SURGE cos(4 pi phase), so it gains and loses SURGE STRIDE / 4 pi (m) on the average.
/// Planted forearms and boots hold their spots while it surges over them.
pub const SURGE: f32 = 0.6;
/// AN-07: the rise. The hips lift HIP_RISE (m) as a leg pushes (highest at HIP_PEAK), the chest CHEST_RISE as the
/// forearm under it pulls (CHEST_PEAK): a wave from hips to chest twice a cycle.
pub const HIP_RISE: f32 = 0.025;
pub const HIP_PEAK: f32 = 0.22;
pub const CHEST_RISE: f32 = 0.035;
pub const CHEST_PEAK: f32 = 0.31;
/// AN-07: the head lags the surge: tipped back HEAD_NOD (rad) as the body speeds up, HEAD_LAG of a cycle late.
pub const HEAD_NOD: f32 = 0.06;
pub const HEAD_LAG: f32 = 0.04;

/// How far ahead of the steady crawl the body is at `phase` (m, `SURGE`): 0 with both legs back (phase 0 and a half).
pub fn surge(phase: f32) -> f32 {
    -SURGE * STRIDE / (4.0 * std::f32::consts::PI) * (4.0 * std::f32::consts::PI * phase).sin()
}

/// 1 at `peak` (and half a cycle on), 0 a quarter cycle either side.
fn wave(phase: f32, peak: f32) -> f32 {
    0.5 + 0.5 * (4.0 * std::f32::consts::PI * (phase - peak)).cos()
}

/// The body's lift above lying flat at `phase` (m): BODY_RAISE plus the hips' rise.
pub fn raise_at(phase: f32) -> f32 {
    BODY_RAISE + HIP_RISE * wave(phase, HIP_PEAK)
}

/// The head's nod at `phase` on top of taking back the chest's pitch (rad).
pub fn nod(phase: f32) -> f32 {
    HEAD_NOD * (4.0 * std::f32::consts::PI * (phase - HEAD_LAG)).sin()
}

/// Where a boot is at stroke `u` (`stroke`'s timing): (how far behind its hip, m; lift). Pushing it holds its spot on
/// the ground (going back at the body's speed); then the leg lies straight back, dragged; drawing up it lifts and
/// comes forward, leaving at the body's speed and landing at the ground's (no jolt).
pub fn boot_stroke(u: f32) -> (f32, f32) {
    let u = u.rem_euclid(1.0);
    let rate = PUSH / (0.5 - DRAW);
    if u < DRAW {
        let t = u / DRAW;
        let (p0, p1, m1) = (BOOT_BACK + PUSH, BOOT_BACK, rate * DRAW);
        let back = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p0 + (-2.0 * t * t * t + 3.0 * t * t) * p1 + (t * t * t - t * t) * m1;
        (back, LIFT * (std::f32::consts::PI * t).sin())
    } else if u < 0.5 {
        (BOOT_BACK + rate * (u - DRAW), 0.0)
    } else {
        (BOOT_BACK + PUSH, 0.0)
    }
}

/// Whether a side is pushing (its boot planted) at stroke `u`: the body moves over it.
pub fn planted(u: f32) -> bool {
    let u = u.rem_euclid(1.0);
    (DRAW..0.5).contains(&u)
}

/// Where a side is in its stroke at `u` (0..1): (how far back from its forward mark it is, 0 at the mark to PUSH
/// at rest; lift). Drawing up it slides forward, lifted; pushing it goes back at the body's speed; then it rests.
pub fn stroke(u: f32) -> (f32, f32) {
    let u = u.rem_euclid(1.0);
    if u < DRAW {
        let t = u / DRAW;
        (PUSH * (1.0 - t * t * (3.0 - 2.0 * t)), LIFT * (std::f32::consts::PI * t).sin())
    } else if u < 0.5 {
        (PUSH * (u - DRAW) / (0.5 - DRAW), 0.0)
    } else {
        (PUSH, 0.0)
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


/// The pose parts every row shares: prone and flat, the rifle along the right forearm in the right hand.
pub fn base() -> PoseParams {
    PoseParams { prone: 1.0, rifle: RIFLE_PITCH, rifle_yaw: RIFLE_ROLL, ..Default::default() }
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
    p.tilt = r[21];
    p.look = r[22];
}

/// Places the four limbs for the body set in `r` (hips, chest, roll): each knee and elbow on its spot (or as near as
/// it reaches), shins down to the toes, forearms forward. Returns the misses, weighted: planted limbs count fully.
fn place_limbs(phase: f32, r: &mut [f32; F], up: [f32; 2]) -> [f32; 12] {
    let mut p = base();
    apply(&mut p, r);
    p.bob = surge(phase);
    let mut miss = [0.0; 12];
    for i in 0..2 {
        let g = if i == 0 { -1.0 } else { 1.0 };
        // AN-07: each boot strokes with the opposite elbow (left boot and right elbow at `phase`, the others half a cycle
        // on). Its spot is taken from the hip as it would lie unrolled and without the surge, so a pushing boot holds
        // its place on the ground while the body rolls and surges over it; the knee lies out to the side on the
        // ground (lifted with the boot as it draws up) and comes in as the leg straightens.
        let (back, lift) = boot_stroke(phase + if i == 0 { 0.0 } else { 0.5 });
        let w = 1.0;
        let hip = hip_at(&PoseParams { yaw: 0.0, twist: 0.0, tilt: 0.0, bob: 0.0, ..p }, i);
        // A pushing boot holds its spot; a resting leg is dragged along with the body, surge and all, and drawing up it
        // goes from one to the other.
        let us = (phase + if i == 0 { 0.0 } else { 0.5 }).rem_euclid(1.0);
        let dragged = if us >= 0.5 { 1.0 } else if us < DRAW { let t = us / DRAW; 1.0 - t * t * (3.0 - 2.0 * t) } else { 0.0 };
        let want = Vec3::new(hip.x + g * BOOT_OUT, hip.y - back + dragged * surge(phase), ANKLE_H + lift);
        let kh = KNEE_H + lift;
        let x0 = [r[i], r[4 + i], r[19 + i], r[2 + i]];
        let [h, s, ro, kn] = solve_n(x0, |x| {
            let mut q = p;
            q.hip[i] = x[0];
            q.side[i] = x[1];
            q.roll[i] = x[2];
            q.knee[i] = x[3];
            let a = ankle_at(&q, i) - want;
            let kk = knee_at(&q, i);
            // A straight leg's roll reaches nothing: it lies turned out (LEG_TURN, the boot on its inside edge, ready to
            // push), and no angle jumps from one row to the next.
            let c = |k: usize| (x[k] - x0[k]) * STEADY;
            [a.x * 3.0, a.y * 3.0, a.z * 3.0, (kk.z - kh) * 3.0, ((kk.x - hip.x) * g - 0.02).min(0.0) * 2.0, (x[2] + g * LEG_TURN) * 0.1, c(0), c(1), c(2), c(3)]
        });
        r[i] = h;
        r[4 + i] = s;
        r[19 + i] = ro;
        r[2 + i] = kn;
        apply(&mut p, r);
        let k = ankle_at(&p, i) - want;
        miss[i * 3..i * 3 + 3].copy_from_slice(&[k.x * w, k.y * w, k.z * 2.0 * w]);
        // The forearm opposite the knee: reaching ahead as it draws up, pulling back as it pushes, in line with the
        // shoulder; the arm's swing out to the side is held near rest (a swimmer's sweep otherwise).
        let (back, lift) = arm_stroke(arm_u(phase, i));
        let w = 1.0;
        // AN-07: the hand's mark is taken from the shoulder before the body's roll, bend and surge, so a planted hand
        // (and the front of the forearm) holds its place on the ground while they move the shoulder over it: it lands
        // HAND_AHEAD in front, in line with the shoulder, and stays there as the body passes until it lies under the
        // face. The elbow stays on the ground and swings out round the hand as the arm folds (the chest is lower than
        // the upper arm is long, so a planted elbow could not stay put under a passing shoulder).
        let sh = shoulder_at(&PoseParams { tilt: 0.0, twist: 0.0, bob: 0.0, ..p }, i);
        let want = Vec3::new(sh.x + g * PALM_OUT, sh.y + HAND_AHEAD - back, PALM_H[i] + lift + up[i]);
        let eh = ELBOW_H + lift + up[i];
        // A planted hand holds its place before anything else (it skates otherwise as the body rolls).
        let hold = if arm_planted(arm_u(phase, i)) { 6.0 } else { 2.0 };
        // The whole arm at once (shoulder swing, swing out, inward turn, elbow bend). Start from an arm reaching ahead
        // and down (prone, about 2.6 rad): hanging back toward the feet is a second solution that also reaches.
        let x0 = [r[6 + i].max(2.2), r[10 + i], r[12 + i], r[8 + i].clamp(0.2, 2.6)];
        let planted_arm = arm_planted(arm_u(phase, i));
        let [a, b, c, el] = solve_n(x0, |x| {
            let mut q = p;
            q.shoulder[i] = x[0];
            q.abduct[i] = x[1];
            q.reach[i] = x[2];
            q.elbow[i] = x[3];
            let (h, ee) = (palm_at(&q, i), elbow_at(&q, i));
            let e = h - want;
            // Planted, the elbow lies on the ground; reaching, it may come up as the arm straightens ahead.
            let lie = if planted_arm { ee.z - eh } else { (ee.z - eh).min(0.0) };
            let c = |k: usize| (x[k] - x0[k]) * STEADY;
            // Planted, the folding arm keeps its elbow out to the side; reaching straight ahead it lines up. An elbow
            // never bends past straight.
            let out = if planted_arm { ((ee.x - sh.x) * g - 0.02).min(0.0) * 2.0 } else { 0.0 };
            [e.x * hold, e.y * hold, e.z * 3.0, lie * 3.0, out, (h.y - ee.y - 0.05).min(0.0) * 3.0, (x[2] - x0[2]) * 0.05, c(0), c(1), c(2), c(3), (x[3] - 0.05).min(0.0) * 5.0]
        });
        r[6 + i] = a;
        r[10 + i] = b;
        // The same swing a whole turn round is the same arm, but would spin it blending to and from prone.
        r[12 + i] = (c + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        r[8 + i] = el;
        apply(&mut p, r);
        let e = palm_at(&p, i) - want;
        miss[6 + i * 3..9 + i * 3].copy_from_slice(&[e.x * 0.3 * w, e.y * w, e.z * 2.0 * w]);
        apply(&mut p, r);
    }
    miss
}

/// Solves the row at `phase`, starting from `prev` (the previous row) so the angles run on continuously: the body
/// flat (no raise), the chest pitched so the shoulders sit at SHOULDER_H, then the limbs on their marks.
pub fn solve_row(phase: f32, prev: Option<&[f32; F]>) -> [f32; F] {
    let mut r = prev.copied().unwrap_or([0.3, 0.3, 0.3, 0.3, -0.3, 0.3, 1.4, 1.4, 1.6, 1.6, 0.2, 0.2, 0.0, 0.0, -0.2, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0]);
    r[15] = raise_at(phase);
    body_roll(phase, &mut r);
    let shoulder_h = SHOULDER_H + CHEST_RISE * wave(phase, CHEST_PEAK);
    let mut p = base();
    for _ in 0..8 {
        apply(&mut p, &r);
        let sh = (shoulder_at(&p, 0) + shoulder_at(&p, 1)) / 2.0;
        let hip = (hip_at(&p, 0) + hip_at(&p, 1)) / 2.0;
        let len = (sh - hip).length();
        let want = ((hip.z - shoulder_h) / len).clamp(-0.9, 0.9).asin();
        let now = ((hip.z - sh.z) / len).clamp(-0.9, 0.9).asin();
        r[14] += want - now;
    }
    place_limbs(phase, &mut r, [0.0; 2]);
    // Eyes ahead: the head takes back the chest's pitch, and lags the surge (AN-07).
    r[17] = 1.0 + r[14] + nod(phase);
    r
}

/// How far the left knee is drawn up less the right: 1 with the left knee fully up (end of its draw), -1 the right.
pub fn drawn(phase: f32) -> f32 {
    let d = |u: f32| 1.0 - stroke(u).0 / PUSH;
    d(phase) - d(phase + 0.5)
}

/// Patch: Animations 5: the body works with the stroke (set before the limbs are placed, so planted ones hold): it
/// rolls onto its side away from the drawn knee, lifting that hip (ROLL, chest and hips together), and the spine
/// bends toward the drawn knee (BEND), swinging the shoulders that way; the head takes back most of the roll
/// (HEAD_STEADY) so the eyes stay level.
fn body_roll(phase: f32, r: &mut [f32; F]) {
    let s = drawn(phase - ROLL_LAG);
    r[16] = CHEST_ROLL * s;
    r[18] = ROLL * s;
    r[21] = -BEND * s; // tilt turns the shoulders the other way round
    r[22] = -HEAD_STEADY * r[16];
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
            for f in [14, 15, 16, 18, 21, 22] {
                let at = |d: isize| old[(k as isize + d).rem_euclid(ROWS as isize) as usize][f];
                rows[k][f] = (at(-2) + 4.0 * at(-1) + 6.0 * at(0) + 4.0 * at(1) + at(2)) / 16.0;
            }
        }
    }
    // AN-04: each arm is lifted where its skin would touch down below the ground (the elbow's centre sits on its mark,
    // but a roll or a landing can tip the elbow's point under), so nothing pokes through.
    let mut up = vec![[0.0f32; 2]; ROWS];
    for pass in 0..4 {
        for k in 0..ROWS {
            let mut r = rows[k];
            if pass == 0 {
                place_limbs(k as f32 / ROWS as f32, &mut r, up[k]);
            }
            let mut p = base();
            apply(&mut p, &r);
            let low = arm_low(&p);
            let mut again = false;
            for i in 0..2 {
                if low[i] < ARM_CLEAR {
                    up[k][i] += ARM_CLEAR - low[i];
                    again = true;
                }
            }
            if again {
                place_limbs(k as f32 / ROWS as f32, &mut r, up[k]);
            }
            r[17] = 1.0 + r[14] + nod(k as f32 / ROWS as f32);
            rows[k] = r;
        }
    }
    rows
}

/// How far an arm's skin stays above the ground at least (m, `solve`).
pub const ARM_CLEAR: f32 = 0.012;

/// The lowest point of each arm's skin (upper arm and forearm; m).
fn arm_low(p: &PoseParams) -> [f32; 2] {
    use std::sync::OnceLock;
    static ARMS: OnceLock<Vec<(Vec3, f32, usize)>> = OnceLock::new();
    let arms = ARMS.get_or_init(|| {
        crate::mesh::figure()
            .vertices
            .iter()
            .filter_map(|v| match crate::mesh::part_of(v.color) {
                Part::LeftArm | Part::LeftForearm => Some((Vec3::from(v.pos), v.color, 0)),
                Part::RightArm | Part::RightForearm => Some((Vec3::from(v.pos), v.color, 1)),
                _ => None,
            })
            .collect()
    });
    let mut low = [f32::MAX; 2];
    for (at, c, i) in arms {
        low[*i] = low[*i].min(crate::pose::pose_mesh_vertex(*at, *c, p).z);
    }
    low
}

/// Sanity: the hips' lift at full prone (`pose::PRONE_LIFT`), so `raise` reads as height above lying flat.
pub const _LIFT: f32 = PRONE_LIFT;
