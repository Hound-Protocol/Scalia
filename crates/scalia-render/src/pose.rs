//! Figure poses: the reference implementation of what the vertex shader does (`wgsl/common.wgsl` ports it).
//! A figure faces +y. Angles rotate in the y-z plane; positive swings a hanging limb forward.

use glam::Vec3;

use crate::mesh::Part;

pub const HIP_Z: f32 = 0.88;
pub const KNEE_Z: f32 = 0.46;
pub const SHOULDER_Z: f32 = 1.40;
pub const ELBOW_Z: f32 = 1.09;
/// Shoulders sit this far either side of the centre line.
pub const SHOULDER_X: f32 = 0.238;
pub const NECK_Z: f32 = 1.46;
/// Where the right hand grips the rifle in the rest pose.
pub const GRIP_Z: f32 = 0.81;
/// How far the lying body is lifted so the chest rests on the ground.
pub const PRONE_LIFT: f32 = 0.23;
/// Lying on the back (prone < 0): the back and its kit are thinner than the chest and its pouches.
pub const SUPINE_LIFT: f32 = 0.16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pose {
    Stand = 0,
    Walk = 1,
    Run = 2,
    Crouch = 3,
    /// Crouched behind cover, aiming over it.
    CoverAim = 4,
    Prone = 5,
    StandAim = 6,
    /// Flat-out sprint, rifle angled down across the body.
    Sprint = 7,
    /// Standing behind a wall, rifle at high ready (muzzle up), not shooting.
    Cover = 8,
    /// Radioing: the left hand holds the handset to the ear, the right keeps the rifle hanging.
    Radio = 9,
    /// Holding the command tablet in both hands at the chest, looking down at it; the rifle slung on the back.
    Tablet = 10,
    /// Crouched walk: knees deeply bent, short steps, rifle at low ready (moving low behind cover).
    CrouchWalk = 11,
    /// Leaning out to the left around a corner while aiming.
    LeanLeft = 12,
    /// Leaning out to the right around a corner while aiming.
    LeanRight = 13,
    /// Half crouch with the rifle up: popping up to aim over a low wall.
    CrouchAim = 14,
    /// Stacked at a door: half crouch, weight forward, rifle at low ready, ready to go through.
    Stack = 15,
    /// Clearing a room: aiming, the upper body sweeping the corners left and right over the cycle.
    Sweep = 16,
    /// Grenade throw over the phase: rifle on the back, overhand wind-up, release, follow-through.
    Throw = 17,
    /// Magazine change over the phase: the left hand to the magazine well, the chest pouch and back.
    Reload = 18,
    /// Hand signal with the free left hand: fist up (hold), then a forward chop (move up).
    Signal = 19,
    /// Kneeling drone launch: rifle on the back, both hands lift the drone overhead and let go.
    Drone = 20,
    /// Wounded and down: on his back, one knee drawn up, a hand pressed to the wound, the other arm out,
    /// moving weakly over the phase.
    Wounded = 21,
    /// Dragging a wounded man: crouched, walking backwards, both hands low gripping his vest straps (upper body;
    /// the legs come from the crouched walk when moving).
    Drag = 22,
    /// First aid: kneeling over the wounded man, hands pressing on the wound over the phase.
    Aid = 23,
    /// Killed: face down and limp, arms loose out to the sides, legs splayed, rifle slung across his back.
    Dead = 24,
    /// Shot and going down wounded, over the phase (0..1, once): the impact rocks him back, he stumbles a step,
    /// his knees give and he falls onto his back, ending exactly in the wounded pose.
    FallBack = 25,
    /// Killed, over the phase (0..1, once): a jolt, the knees go and he crumples forward onto his face,
    /// ending exactly in the dead pose.
    FallForward = 26,
    /// Killed while lying on his back (wounded, then bled out or hit again): limp on his back, arms fallen
    /// out, head rolled to the side. No turning over.
    DeadBack = 27,
}

/// Where the tablet sits between the hands in the tablet pose (figure frame, metres).
pub const TABLET_AT: Vec3 = Vec3::new(0.0, 0.375, 1.19);
/// How far the tablet's face tilts back towards the face (radians from upright).
pub const TABLET_TILT: f32 = 0.85;

impl Pose {
    pub fn from_id(id: u32) -> Pose {
        match id {
            1 => Pose::Walk,
            2 => Pose::Run,
            3 => Pose::Crouch,
            4 => Pose::CoverAim,
            5 => Pose::Prone,
            6 => Pose::StandAim,
            7 => Pose::Sprint,
            8 => Pose::Cover,
            9 => Pose::Radio,
            10 => Pose::Tablet,
            11 => Pose::CrouchWalk,
            12 => Pose::LeanLeft,
            13 => Pose::LeanRight,
            14 => Pose::CrouchAim,
            15 => Pose::Stack,
            16 => Pose::Sweep,
            17 => Pose::Throw,
            18 => Pose::Reload,
            19 => Pose::Signal,
            20 => Pose::Drone,
            _ => Pose::Stand,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PoseParams {
    /// Upper body lean forward (radians).
    pub lean: f32,
    /// Hip, knee (bend, positive backwards) and shoulder angles, left then right.
    pub hip: [f32; 2],
    pub knee: [f32; 2],
    pub shoulder: [f32; 2],
    /// Elbow bend, left then right (positive swings the forearm forward).
    pub elbow: [f32; 2],
    /// Inward swing of each arm around the vertical through its shoulder (radians, positive towards the centre
    /// line), so the support hand can reach across to the rifle.
    pub reach: [f32; 2],
    /// Head nod around the neck (positive tilts the face up).
    pub head: f32,
    /// Rifle pitch around the grip (positive points the muzzle up).
    pub rifle: f32,
    /// 0 upright, 1 lying flat; fractions only while blending.
    pub prone: f32,
    /// Vertical shift that keeps the lowest foot point on the ground.
    pub drop: f32,
    /// 0 rifle in the right hand, 1 slung across the back (hands free).
    pub sling: f32,
    /// Whole-body rise and fall with the stride (metres).
    pub bob: f32,
    /// Upper body turn around the vertical through the hips, against the legs (radians, positive to the left).
    pub twist: f32,
    /// Head turn left and right (radians, positive to the left).
    pub look: f32,
    /// Each upper arm raised out to the side (radians, positive away from the body), left then right.
    pub abduct: [f32; 2],
    /// Each leg swung out to the side around the front-to-back line through its hip (radians, positive moves the
    /// foot to the figure's right, +x), left then right: walking sideways and at angles while facing ahead.
    pub side: [f32; 2],
    /// Upper body leaned sideways around the front-to-back line through the hips (radians, positive to the
    /// right, +x): leaning out around a corner.
    pub tilt: f32,
    /// 1 keeps the planted boots flat whatever the knee bend (crouched walking); 0 lets the ankle follow the gait.
    pub flat: f32,
    /// Rifle turn around the vertical through the grip (positive swings the muzzle to the left, across the
    /// body, and the stock out to the right shoulder), applied after `rifle` pitch.
    pub rifle_yaw: f32,
}

/// The rifle slung on the back: rolled flat (magazine inwards), muzzle down, hung diagonally from the right
/// shoulder blade.
pub fn slung(v: Vec3) -> Vec3 {
    let d = v - Vec3::new(SHOULDER_X, 0.0, GRIP_Z);
    let r = Vec3::new(d.z, d.y, -d.x);
    let (s, c) = (-1.5f32).sin_cos();
    let p = Vec3::new(r.x, r.y * c - r.z * s, r.y * s + r.z * c);
    let (s2, c2) = 0.3f32.sin_cos();
    Vec3::new(p.x * c2 - p.z * s2, p.y, p.x * s2 + p.z * c2) + SLING_AT
}

/// Where the slung rifle's grip hangs (figure frame).
pub const SLING_AT: Vec3 = Vec3::new(0.06, -0.27, 1.2);

/// Rotates `q` in the y-z plane around (pivot_y, pivot_z).
pub fn rot(q: Vec3, pivot_y: f32, pivot_z: f32, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    let (dy, dz) = (q.y - pivot_y, q.z - pivot_z);
    Vec3::new(q.x, pivot_y + dy * c - dz * s, pivot_z + dy * s + dz * c)
}

/// An arm point: bend at the elbow, swing at the shoulder, then turn inward around the vertical through the
/// shoulder (`side_x` is the shoulder's x; the turn is towards the centre line).
fn arm(q: Vec3, shoulder: f32, elbow: f32, reach: f32, abduct: f32, side_x: f32) -> Vec3 {
    let p = rot(rot(q, 0.0, ELBOW_Z, elbow), 0.0, SHOULDER_Z, shoulder);
    // Raise the arm out to the side (elbow away from the body) around the front-to-back axis through the shoulder.
    let g = side_x.signum();
    let (sa, ca) = abduct.sin_cos();
    let (dx, dz) = (p.x - side_x, p.z - SHOULDER_Z);
    let p = Vec3::new(side_x + dx * ca - dz * sa * g, p.y, SHOULDER_Z + dx * sa * g + dz * ca);
    let a = reach * side_x.signum();
    let (s, c) = a.sin_cos();
    let (dx, dy) = (p.x - side_x, p.y);
    Vec3::new(side_x + dx * c - dy * s, dx * s + dy * c, p.z)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// The palms and elbows of the converted soldier in the rest pose (measured by examples/rig_report): the arms
/// hang closer in than the shoulder pivots.
pub const PALM: Vec3 = Vec3::new(0.18, 0.0, 0.80);
pub const ELBOW_X: f32 = 0.18;
/// The rifle's pistol grip in the rest pose: the point that sits in the right palm.
pub const RIFLE_GRIP: Vec3 = Vec3::new(0.238, 0.03, 0.78);
/// The left hand's place on the rifle: the handguard, under the barrel (rest pose).
pub const RIFLE_GUARD: Vec3 = Vec3::new(0.238, 0.34, 0.82);

/// Where a palm is, in the figure's frame before lean.
pub fn hand(p: &PoseParams, side: Side) -> Vec3 {
    let (i, x, g) = if side == Side::Left { (0, -SHOULDER_X, -1.0) } else { (1, SHOULDER_X, 1.0) };
    arm(Vec3::new(PALM.x * g, PALM.y, PALM.z), p.shoulder[i], p.elbow[i], p.reach[i], p.abduct[i], x)
}

/// Where an elbow is in a pose.
pub fn elbow(p: &PoseParams, side: Side) -> Vec3 {
    let (i, x, g) = if side == Side::Left { (0, -SHOULDER_X, -1.0) } else { (1, SHOULDER_X, 1.0) };
    arm(Vec3::new(ELBOW_X * g, 0.0, ELBOW_Z), p.shoulder[i], p.elbow[i], p.reach[i], p.abduct[i], x)
}

/// Where a rest-pose rifle point goes when the rifle is held in the right palm at the pose's pitch.
pub fn held(v: Vec3, p: &PoseParams) -> Vec3 {
    let d = rot(v, RIFLE_GRIP.y, RIFLE_GRIP.z, p.rifle) - RIFLE_GRIP;
    let (s, c) = p.rifle_yaw.sin_cos();
    hand(p, Side::Right) + Vec3::new(d.x * c - d.y * s, d.x * s + d.y * c, d.z)
}

/// Puts the left palm back on the handguard after blends and idle motion have pulled it off: a closed-form
/// rifle tilt for the moments a blend swings the handguard out of the arm's reach, then damped least-squares
/// steps on the left shoulder, elbow and reach. `w` is how much the pose holds the rifle in both hands
/// (`two_handed`, blended like the pose itself), so the hand lets go and grabs as smoothly as the blend runs.
/// Mirrored in common.wgsl `grip`.
pub fn grip(p: &PoseParams, w: f32) -> PoseParams {
    let w = w * (1.0 - p.sling);
    if w <= 0.0 {
        return *p;
    }
    // First tilt the rifle (around the right hand) just enough to bring the handguard within the left arm's
    // reach; then the arm alone can always get there.
    let mut p0 = *p;
    p0.rifle += reach_pitch(p) * GRIP_TILT * w;
    let p = &p0;
    reach_left_capped(p, held(RIFLE_GUARD, p), w, GRIP_MAX)
}

/// Puts the left palm on `target` (figure frame): damped least-squares steps on the left shoulder, elbow and
/// reach, blended in by `w`. `grip` uses it for the handguard; the reload for the magazine well and pouch.
/// Mirrored in common.wgsl `reach_left`.
pub fn reach_left(p: &PoseParams, target: Vec3, w: f32) -> PoseParams {
    reach_left_capped(p, target, w, REACH_MAX)
}

/// `reach_left` with each joint's total correction softly capped at `max` radians.
fn reach_left_capped(p: &PoseParams, target: Vec3, w: f32, max: f32) -> PoseParams {
    if w <= 0.0 {
        return *p;
    }
    // Out of reach even after the tilt (going prone, say): damp harder the further it is, so the arm only
    // stretches towards it instead of flailing.
    let excess = (target.distance(Vec3::new(-SHOULDER_X, 0.0, SHOULDER_Z)) - GRIP_REACH).max(0.0);
    let damp = GRIP_DAMP + 2.0 * excess;
    // Unknowns: left shoulder, elbow and reach.
    let apply = |q: &PoseParams, x: [f32; 3]| {
        let mut t = *q;
        t.shoulder[0] += x[0];
        t.elbow[0] += x[1];
        t.reach[0] += x[2];
        t
    };
    let res = |q: &PoseParams| target - hand(q, Side::Left);
    let mut q = *p;
    for _ in 0..GRIP_STEPS {
        let r = res(&q);
        if r.length_squared() < 1e-6 {
            break; // within 1 mm: most frames stop after a step or two
        }
        let d = 0.01;
        let j: [Vec3; 3] = std::array::from_fn(|k| {
            let mut x = [0.0; 3];
            x[k] = d;
            (res(&apply(&q, x)) - r) / d
        });
        // x = -J^T (J J^T + l^2 I)^-1 r
        let mut m = glam::Mat3::from_diagonal(Vec3::splat(damp * damp));
        for c in &j {
            m += glam::Mat3::from_cols(*c * c.x, *c * c.y, *c * c.z);
        }
        if m.determinant().abs() < 1e-9 {
            break;
        }
        let y = m.inverse() * r;
        let x: [f32; 3] = std::array::from_fn(|k| (-j[k].dot(y)).clamp(-0.5, 0.5));
        q = apply(&q, x);
    }
    // Soft cap on each joint's total correction.
    let cap = |d: f32| max * (d / max).tanh() * w;
    let mut out = *p;
    out.shoulder[0] += cap(q.shoulder[0] - p.shoulder[0]);
    out.elbow[0] += cap(q.elbow[0] - p.elbow[0]);
    out.reach[0] += cap(q.reach[0] - p.reach[0]);
    out
}
const GRIP_STEPS: usize = 6;
/// Correction cap for reaching to points other than the handguard (actions move the arm a long way).
const REACH_MAX: f32 = 2.0;
pub const GRIP_DAMP: f32 = 0.05;
/// About the smallest extra rifle pitch (radians) that brings the handguard within GRIP_REACH of the left shoulder.
/// The handguard turns on a circle around the rifle's (turned) x axis through the right palm; the squared distance to the
/// shoulder along it is k + m cos(d + phi), so the answer is closed-form.
pub fn reach_pitch(p: &PoseParams) -> f32 {
    // Work in the rifle's turned frame, where pitch is a turn around x.
    let h = hand(p, Side::Right);
    let v = rot(RIFLE_GUARD, RIFLE_GRIP.y, RIFLE_GRIP.z, p.rifle) - RIFLE_GRIP;
    let a = h - Vec3::new(-SHOULDER_X, 0.0, SHOULDER_Z);
    let (sy, cy) = (-p.rifle_yaw).sin_cos();
    let a = Vec3::new(a.x * cy - a.y * sy, a.x * sy + a.y * cy, a.z);
    let k = a.length_squared() + v.length_squared() + 2.0 * a.x * v.x;
    if k + 2.0 * (a.y * v.y + a.z * v.z) <= GRIP_REACH * GRIP_REACH {
        return 0.0;
    }
    // (a.y, a.z) . rot(d) (v.y, v.z) = ra rv cos(d + phi)
    let (ra, rv) = ((a.y * a.y + a.z * a.z).sqrt(), (v.y * v.y + v.z * v.z).sqrt());
    if ra * rv < 1e-6 {
        return 0.0;
    }
    let phi = v.z.atan2(v.y) - a.z.atan2(a.y);
    let c = ((GRIP_REACH * GRIP_REACH - k) / (2.0 * ra * rv)).clamp(-1.0, 1.0);
    let base = c.acos();
    let wrap = |x: f32| (x + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let (d1, d2) = (wrap(base - phi), wrap(-base - phi));
    // Both roots, weighted towards the smaller: continuous where they swap (the rifle crossing the band in
    // front where neither tilt reaches; there the hand comes off the handguard for a moment).
    let (q1, q2) = (d1 * d1, d2 * d2);
    if q1 + q2 < 1e-9 { 0.0 } else { (d1 * q2 + d2 * q1) / (q1 + q2) }
}
/// How far the left palm can get from the shoulder pivot with a slightly bent elbow (m).
pub const GRIP_REACH: f32 = 0.595;
/// Share of `reach_pitch` the rifle actually tilts (the arm stretches for the rest), so a blend that flips the
/// rifle does not whip it round.
const GRIP_TILT: f32 = 0.6;
/// Largest correction `grip` makes to any one joint (radians, soft).
const GRIP_MAX: f32 = 0.7;

/// `grip`'s weight while blending pose `a` into `b` (`w0` as given to `blend`, eased the same way).
pub fn grip_weight(a: Pose, b: Pose, w0: f32) -> f32 {
    let (ha, hb) = (two_handed(a), two_handed(b));
    // Let go early when leaving a two-handed hold, grab late when coming into one: the hand moves while the
    // body is still close to the holding pose, where the correction is small.
    let w = if ha > hb { (w0 / GRIP_SWAP).clamp(0.0, 1.0) } else { ((w0 - (1.0 - GRIP_SWAP)) / GRIP_SWAP).clamp(0.0, 1.0) };
    let w = w * w * (3.0 - 2.0 * w);
    ha + (hb - ha) * w
}
/// Share of a blend over which the hand lets go of or grabs the handguard.
pub const GRIP_SWAP: f32 = 0.6;

/// Where the left hand is going during a reload at `t` (0..1): handguard, magazine well, chest pouch, and back.
pub fn reload_target(p: &PoseParams, t: f32) -> Vec3 {
    let guard = held(RIFLE_GUARD, p);
    let well = held(RIFLE_WELL, p);
    // Same times as common.wgsl `reload_target` (written inline there).
    let kt = [0.0, 0.15, 0.4, 0.55, 0.75];
    let path = [guard, well, RELOAD_POUCH, RELOAD_POUCH, well];
    if t >= 0.75 {
        let u = ((t - 0.75) / 0.25).clamp(0.0, 1.0);
        return well.lerp(guard, u * u * (3.0 - 2.0 * u));
    }
    for i in 0..4 {
        if t <= kt[i + 1] {
            let u = ((t - kt[i]) / (kt[i + 1] - kt[i])).clamp(0.0, 1.0);
            return path[i].lerp(path[i + 1], u * u * (3.0 - 2.0 * u));
        }
    }
    well
}
/// The magazine well, a rifle point (rest pose): under the receiver just ahead of the grip.
pub const RIFLE_WELL: Vec3 = Vec3::new(0.238, 0.11, 0.70);
/// The magazine pouch on the front of the vest, left of centre (figure frame).
pub const RELOAD_POUCH: Vec3 = Vec3::new(-0.04, 0.17, 1.08);

/// 1 for poses that hold the rifle with both hands (left palm on the handguard), else 0.
pub fn two_handed(pose: Pose) -> f32 {
    match pose {
        Pose::Stand | Pose::Walk | Pose::Run | Pose::Sprint | Pose::Crouch | Pose::CoverAim | Pose::Cover | Pose::StandAim | Pose::CrouchWalk | Pose::LeanLeft | Pose::LeanRight | Pose::CrouchAim | Pose::Stack | Pose::Sweep => 1.0,
        _ => 0.0,
    }
}

fn leg(q: Vec3, hip: f32, knee: f32) -> Vec3 {
    rot(rot(q, 0.0, KNEE_Z, -knee), 0.0, HIP_Z, hip)
}

/// The upper body leaned sideways by `a` around the front-to-back line through the hips (positive to +x).
pub fn tilt(q: Vec3, a: f32) -> Vec3 {
    if a == 0.0 {
        return q;
    }
    let (s, c) = a.sin_cos();
    let dz = q.z - HIP_Z;
    Vec3::new(q.x * c + dz * s, q.y, HIP_Z - q.x * s + dz * c)
}

/// Head nod while aiming: down onto the stock (cheek weld), eyes along the sights.
pub const AIM_HEAD: f32 = -0.35;

/// How far the upper body sweeps either way clearing a room (radians).
pub const SWEEP: f32 = 0.6;

/// A value through five keyframes (times `k` from 0 to 1, values `v`), eased between each pair: one-shot actions.
pub fn keyed(t: f32, k: [f32; 5], v: [f32; 5]) -> f32 {
    let t = t.clamp(0.0, 1.0);
    for i in 0..4 {
        if t <= k[i + 1] {
            let u = ((t - k[i]) / (k[i + 1] - k[i]).max(1e-6)).clamp(0.0, 1.0);
            return v[i] + (v[i + 1] - v[i]) * u * u * (3.0 - 2.0 * u);
        }
    }
    v[4]
}

/// How far a soldier leans out around a corner (radians).
pub const LEAN_TILT: f32 = 0.32;

/// Where each hip joint is across the body (x), the pivot for `side`.
pub const HIP_X: f32 = 0.1;

/// A leg point swung out to the side by `a` around the front-to-back line through the hip at `px`.
pub fn sideways(q: Vec3, a: f32, px: f32) -> Vec3 {
    if a == 0.0 {
        return q;
    }
    let (s, c) = a.sin_cos();
    let (dx, dz) = (q.x - px, q.z - HIP_Z);
    Vec3::new(px + dx * c - dz * s, q.y, HIP_Z + dx * s + dz * c)
}

/// Height of the ankle joint above the ground (rest pose).
pub const ANKLE_Z: f32 = 0.085;
/// The boot's heel and toe along the foot (y), where it touches the ground.
pub const HEEL_Y: f32 = -0.09;
pub const TOE_Y: f32 = 0.14;

/// Ankle bend that keeps the boot level while the leg is under weight: it cancels the shin's pitch (`hip - knee`)
/// fully while the knee is straight-ish and not at all once the knee lifts for the swing. Past the vertical (leg
/// behind) it cancels only part, so the heel rolls up at toe-off.
pub fn ankle(hip: f32, knee: f32) -> f32 {
    let shin = hip - knee;
    let t = ((knee - 0.45) / 0.45).clamp(0.0, 1.0);
    let planted = 1.0 - t * t * (3.0 - 2.0 * t);
    -(shin.max(0.0) + 0.4 * shin.min(0.0)) * planted
}

/// `ankle` with the boot held flat to the ground by `flat` (0..1).
pub fn ankle_flat(hip: f32, knee: f32, flat: f32) -> f32 {
    let a = ankle(hip, knee);
    a + (-(hip - knee) - a) * flat
}

/// A foot point: ankle bend, then the leg.
fn foot(q: Vec3, hip: f32, knee: f32, flat: f32) -> Vec3 {
    leg(rot(q, 0.0, ANKLE_Z, ankle_flat(hip, knee, flat)), hip, knee)
}

/// Crouched gait in any direction (see `gait_dir`): each foot's target moves in a straight line at an even speed
/// while planted (`reach` metres either side of under the hip, along the line of travel) and arcs forward in the
/// swing, rising by `lift`; hip and knee are solved from it by two-bone IK (knee ahead of the foot), with the ankle
/// `height` below the hip. The planted boot then stays put on the ground however deep the crouch.
fn crouch_gait(phase: f32, reach: f32, height: f32, lift: f32, stance: f32, dir: f32) -> ([f32; 2], [f32; 2], [f32; 2]) {
    use std::f32::consts::PI;
    let (l1, l2) = (HIP_Z - KNEE_Z, KNEE_Z - ANKLE_Z);
    let k = stride_scale(dir);
    let (s, c) = dir.sin_cos();
    let (mut hip, mut knee, mut side) = ([0.0; 2], [0.0; 2], [0.0; 2]);
    for (i, off) in [0.0f32, 0.5].into_iter().enumerate() {
        let u = (phase + off).fract();
        // Along the line of travel: +reach ahead at heel strike, -reach behind at toe-off.
        let (along, raise) = if u < stance {
            (reach * (1.0 - 2.0 * u / stance), 0.0)
        } else {
            let w = (u - stance) / (1.0 - stance);
            let m = -2.0 * reach / stance * (1.0 - stance);
            let (w2, w3) = (w * w, w * w * w);
            let a = (2.0 * w3 - 3.0 * w2 + 1.0) * -reach + (w3 - 2.0 * w2 + w) * m + (-2.0 * w3 + 3.0 * w2) * reach + (w3 - w2) * m;
            (a, lift * (PI * w).sin().powi(2))
        };
        let out = if i == 0 { -1.0 } else { 1.0 };
        let fy = along * k * c;
        let fx = -along * k * s + out * 0.06 * s.abs();
        let h = height - raise;
        side[i] = fx.atan2(h);
        let hz = (h * h + fx * fx).sqrt();
        let d = (fy * fy + hz * hz).sqrt().min(l1 + l2 - 1e-3);
        let bend = ((d * d - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0).acos();
        hip[i] = fy.atan2(hz) + (l2 * bend.sin() / d).clamp(-1.0, 1.0).asin();
        knee[i] = bend;
    }
    (hip, knee, side)
}

/// Gait for both legs at a phase: (hip, knee) angles. Each leg swings `swing` radians about `forward`; its knee
/// flexes most mid-swing (as it passes under the body) and a little at mid-stance, carrying the weight, so the hips
/// dip twice per stride (motion guide rule 2). The right leg is half a cycle behind the left.
fn gait(phase: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32) -> ([f32; 2], [f32; 2]) {
    let mut hip = [0.0; 2];
    let mut knee = [0.0; 2];
    for (i, off) in [0.0f32, 0.5].into_iter().enumerate() {
        (hip[i], knee[i]) = gait_leg((phase + off).fract(), swing, forward, lift, base, stance);
    }
    (hip, knee)
}

/// Gait in any direction while facing ahead: `dir` is the way the body moves, measured from straight ahead
/// towards the left (radians). Each leg's swing about `forward` splits into a forward part (hip, cos dir) and a
/// sideways part (side, sin dir), so the planted foot sweeps straight back along the line of travel.
fn gait_dir(phase: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32, dir: f32) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let (hip, knee) = gait(phase, swing, forward, lift, base, stance);
    if dir == 0.0 {
        return (hip, knee, [0.0; 2]);
    }
    let (s, c) = dir.sin_cos();
    // Side-stepping is a shuffle: shorter steps (the cadence rises to match, `stride_scale`) with the feet kept
    // a little apart, so the legs do not scissor across each other.
    let k = stride_scale(dir);
    let mut h = [0.0; 2];
    let mut sd = [0.0; 2];
    for i in 0..2 {
        let e = (hip[i] - forward) * k;
        h[i] = forward + e * c;
        let out = if i == 0 { -1.0 } else { 1.0 };
        sd[i] = -e * s + out * SIDE_SPREAD * s.abs();
    }
    (h, knee, sd)
}

/// How far the feet sit apart (radians out from each hip) when stepping straight sideways.
const SIDE_SPREAD: f32 = 0.06;

/// Stride length when moving in direction `dir`, as a share of the forward stride: 1 forwards and backwards,
/// down to 0.55 straight sideways.
pub fn stride_scale(dir: f32) -> f32 {
    1.0 - 0.45 * dir.sin().abs()
}

/// Instance value c.z for the shader: the upper-body pose layered over the legs (whole part: id + 1, 0 for none)
/// and the way the soldier moves from his facing (fractional part: `dir` as a share of a turn). Every instance
/// slot is taken, so the two share one float; a whole number alone still means "moving straight ahead".
pub fn pack_layer(upper: Option<Pose>, dir: f32) -> f32 {
    let whole = upper.map_or(0.0, |p| p as u32 as f32 + 1.0);
    let turn = (dir / std::f32::consts::TAU).rem_euclid(1.0);
    whole + if turn >= 0.9999 { 0.0 } else { turn }
}

/// The inverse of `pack_layer` (as common.wgsl reads it): (upper pose id + 1 or 0, dir in -pi..pi).
pub fn unpack_layer(v: f32) -> (f32, f32) {
    let whole = v.floor();
    let d = (v - whole) * std::f32::consts::TAU;
    (whole, if d > std::f32::consts::PI { d - std::f32::consts::TAU } else { d })
}

/// Ground covered per gait cycle moving in direction `dir` (see `stride_m`); the cadence is speed / this.
pub fn stride_m_dir(pose: Pose, dir: f32) -> f32 {
    stride_m(pose) * stride_scale(dir)
}

/// One leg through a stride (u in 0..1, heel strike at 0). Stance (the first `stance` of the cycle): the hip sweeps
/// back at an even rate, so the planted foot travels backwards at a steady speed (no slide once the cadence matches
/// the ground speed), with a little knee give mid-stance. Swing: the leg comes forward eased, the knee lifting it clear.
pub fn gait_leg(u: f32, swing: f32, forward: f32, lift: f32, base: f32, stance: f32) -> (f32, f32) {
    use std::f32::consts::PI;
    if u < stance {
        let s = u / stance;
        (forward + swing * (1.0 - 2.0 * s), base + 0.03 * (PI * s).sin().powi(2))
    } else {
        // Hermite from back to front whose end speeds match the stance sweep, so the leg never snaps at toe-off
        // or heel strike.
        let w = (u - stance) / (1.0 - stance);
        let m = -2.0 * swing / stance * (1.0 - stance);
        let (w2, w3) = (w * w, w * w * w);
        let hip = (2.0 * w3 - 3.0 * w2 + 1.0) * -swing + (w3 - 2.0 * w2 + w) * m + (-2.0 * w3 + 3.0 * w2) * swing + (w3 - w2) * m;
        (forward + hip, base + lift * (PI * w).sin().powi(2))
    }
}

/// Ground covered per gait cycle (m): the planted foot sweeps back `stride` while the body moves the same, so the
/// cadence is speed / stride. Measured by `examples/anim_audit` (keep in step with the gait parameters).
pub fn stride_m(pose: Pose) -> f32 {
    match pose {
        Pose::Walk => WALK_STRIDE,
        Pose::Run => RUN_STRIDE,
        Pose::Sprint => SPRINT_STRIDE,
        Pose::CrouchWalk => CROUCH_STRIDE,
        _ => 1.0,
    }
}
pub const WALK_STRIDE: f32 = 1.21;
pub const RUN_STRIDE: f32 = 2.12;
pub const SPRINT_STRIDE: f32 = 2.50;
pub const CROUCH_STRIDE: f32 = 1.0;

/// Pose parameters at a gait phase (0..1; only the gaits use it). Rifle carries per pose (rule 3): low ready when
/// walking, high port when running, angled down across the body when sprinting, shouldered when aiming.
pub fn params(pose: Pose, phase: f32) -> PoseParams {
    params_dir(pose, phase, 0.0)
}

/// `params` for a soldier moving in direction `dir` (from straight ahead towards the left, radians) while facing
/// ahead: the gaits step sideways and backwards; other poses ignore it.
pub fn params_dir(pose: Pose, phase: f32, dir: f32) -> PoseParams {
    let mut p = PoseParams::default();
    match pose {
        Pose::Stand => carry(&mut p, Carry::SafeHang),
        Pose::Walk => {
            (p.hip, p.knee, p.side) = gait_dir(phase, 0.45, 0.0, 0.75, 0.06, 0.6, dir);
            p.lean = 0.05;
            carry(&mut p, Carry::LowReady);
            stride(&mut p, phase, 0.018, 0.06, 0.035, 0.025, 0.04);
        }
        Pose::Run => {
            // Combat run (reference: BF3 E3 trailer, the squad rushing the street): low, knees well bent, torso
            // pitched forward, rifle nearly level across the chest.
            (p.hip, p.knee, p.side) = gait_dir(phase, 0.6, 0.2, 1.15, 0.4, 0.45, dir);
            p.lean = 0.32;
            carry(&mut p, Carry::Rush);
            p.head = 0.28;
            stride(&mut p, phase, 0.03, 0.2, 0.08, 0.06, 0.16);
        }
        Pose::Sprint => {
            (p.hip, p.knee, p.side) = gait_dir(phase, 0.56, 0.1, 1.45, 0.2, 0.36, dir);
            p.lean = 0.42;
            carry(&mut p, Carry::Collapsed);
            p.head = 0.38;
            stride(&mut p, phase, 0.035, 0.3, 0.06, 0.08, 0.24);
        }
        Pose::CrouchWalk => {
            (p.hip, p.knee, p.side) = crouch_gait(phase, 0.3, 0.575, 0.12, 0.6, dir);
            p.flat = 1.0;
            p.lean = 0.3;
            carry(&mut p, Carry::LowReady);
            p.head = 0.3;
            stride(&mut p, phase, 0.012, 0.05, 0.02, 0.02, 0.03);
        }
        Pose::LeanLeft | Pose::LeanRight => {
            // Out around a corner: feet braced apart, upper body leaned over the outside foot, aim held level.
            let g = if pose == Pose::LeanRight { 1.0 } else { -1.0 };
            aim_arms(&mut p);
            p.head = AIM_HEAD;
            p.tilt = LEAN_TILT * g;
            p.side = [-0.06 + 0.04 * g, 0.06 + 0.04 * g];
            p.knee = [0.12, 0.12];
            p.hip = [0.06, 0.06];
        }
        Pose::CrouchAim => {
            // Half crouch, feet staggered and flat, rifle up: over a low wall.
            (p.hip, p.knee, p.side) = crouch_gait(0.0, 0.15, 0.6, 0.0, 0.6, 0.0);
            p.flat = 1.0;
            p.lean = 0.15;
            aim_arms(&mut p);
            p.head = AIM_HEAD;
        }
        Pose::Stack => {
            (p.hip, p.knee, p.side) = crouch_gait(0.0, 0.12, 0.7, 0.0, 0.6, 0.0);
            p.flat = 1.0;
            p.lean = 0.22;
            carry(&mut p, Carry::LowReady);
            p.head = 0.05;
        }
        Pose::Sweep => {
            aim_arms(&mut p);
            p.head = AIM_HEAD;
            let t = phase * std::f32::consts::TAU;
            p.twist += SWEEP * t.sin();
            p.look -= 0.3 * SWEEP * t.sin();
            (p.hip, p.knee) = ([0.05, 0.05], [0.12, 0.12]);
        }
        Pose::Throw => {
            p.sling = 1.0;
            p.hip = [0.22, -0.12];
            p.knee = [0.18, 0.12];
            let k = [0.0, 0.35, 0.55, 0.75, 1.0];
            p.shoulder = [keyed(phase, k, [0.9, 1.5, 0.8, 0.5, 0.9]), keyed(phase, k, [0.7, 2.6, 2.0, 0.9, 0.7])];
            p.elbow = [keyed(phase, k, [1.4, 0.3, 1.4, 1.6, 1.4]), keyed(phase, k, [1.6, 1.9, 0.3, 0.4, 1.6])];
            p.reach = [keyed(phase, k, [0.4, 0.2, 0.5, 0.5, 0.4]), keyed(phase, k, [0.3, 0.0, 0.2, 0.6, 0.3])];
            p.abduct = [0.1, keyed(phase, k, [0.2, 0.6, 0.2, 0.1, 0.2])];
            p.twist = keyed(phase, k, [0.0, -0.5, 0.4, 0.5, 0.0]);
            p.lean = keyed(phase, k, [0.05, -0.1, 0.25, 0.3, 0.05]);
            p.head = 0.0;
        }
        Pose::Reload => {
            // The rifle comes in and up to look into the magazine well; the left hand is placed by `reach_left`
            // on the path `reload_target` gives (the shader does the same).
            carry(&mut p, Carry::LowReady);
            p.rifle += 0.3 * (std::f32::consts::PI * phase).sin();
            p.head = 0.4;
        }
        Pose::Signal => {
            carry(&mut p, Carry::LowReady);
            let k = [0.0, 0.2, 0.45, 0.7, 1.0];
            p.shoulder[0] = keyed(phase, k, [0.8, 2.9, 2.9, 1.6, 0.8]);
            p.elbow[0] = keyed(phase, k, [1.4, 1.9, 1.9, 0.15, 1.4]);
            p.reach[0] = keyed(phase, k, [0.5, 0.1, 0.1, 0.1, 0.5]);
            p.abduct[0] = keyed(phase, k, [0.0, 0.3, 0.3, 0.0, 0.0]);
            p.head = 0.1;
        }
        Pose::Drone => {
            kneel(&mut p);
            p.sling = 1.0;
            let k = [0.0, 0.35, 0.55, 0.8, 1.0];
            let sh = keyed(phase, k, [0.9, 1.0, 2.7, 2.7, 0.9]);
            let el = keyed(phase, k, [1.6, 1.6, 0.4, 0.4, 1.6]);
            p.shoulder = [sh, sh];
            p.elbow = [el, el];
            p.reach = [0.45, 0.45];
            p.head = keyed(phase, k, [0.2, 0.2, 0.6, 0.7, 0.2]);
        }
        Pose::Crouch => {
            kneel(&mut p);
            carry(&mut p, Carry::LowReady);
            p.head = 0.35;
        }
        Pose::Wounded => {
            p.prone = -1.0;
            p.sling = 1.0;
            let k = [0.0, 0.3, 0.5, 0.75, 1.0];
            p.hip = [keyed(phase, k, [0.85, 1.05, 0.95, 0.75, 0.85]), 0.1];
            p.knee = [keyed(phase, k, [1.45, 1.7, 1.55, 1.3, 1.45]), 0.12];
            // Left hand pressed to the belly, right arm flung out.
            p.shoulder = [0.55, 0.15];
            p.elbow = [1.95, 0.3];
            p.reach = [0.75, 0.0];
            p.abduct = [0.0, 1.05];
            p.head = keyed(phase, k, [-0.45, -0.6, -0.5, -0.35, -0.45]);
            p.look = 0.25;
        }
        Pose::Drag => {
            (p.hip, p.knee, p.side) = crouch_gait(0.0, 0.15, 0.6, 0.0, 0.6, 0.0);
            p.flat = 1.0;
            p.lean = 0.55;
            p.sling = 1.0;
            p.shoulder = [1.0, 1.0];
            p.elbow = [0.25, 0.25];
            p.reach = [0.3, 0.3];
            p.head = 0.45;
        }
        Pose::Aid => {
            kneel(&mut p);
            p.lean = 0.4;
            p.sling = 1.0;
            let k = [0.0, 0.25, 0.5, 0.75, 1.0];
            let sh = keyed(phase, k, [0.85, 1.0, 0.85, 1.0, 0.85]);
            let el = keyed(phase, k, [0.55, 0.3, 0.55, 0.3, 0.55]);
            p.shoulder = [sh, sh];
            p.elbow = [el, el];
            p.reach = [0.35, 0.35];
            p.head = -0.55;
        }
        Pose::CoverAim => {
            kneel(&mut p);
            aim_arms(&mut p);
            p.head = AIM_HEAD + 0.1;
        }
        Pose::Cover => carry(&mut p, Carry::HighReady),
        Pose::Tablet => {
            p.shoulder = [0.35, 0.35];
            p.elbow = [1.5, 1.5];
            p.reach = [-0.07, -0.07];
            p.head = -0.45;
            p.sling = 1.0;
        }
        Pose::Radio => {
            carry(&mut p, Carry::SafeHang);
            p.shoulder[0] = 1.45;
            p.elbow[0] = 2.4;
            p.reach[0] = 1.05;
            p.head = 0.1;
        }
        Pose::FallBack => {
            let k = [0.0, 0.2, 0.45, 0.75, 1.0];
            let t = phase;
            p.sling = keyed(t, k, [0.0, 0.6, 1.0, 1.0, 1.0]);
            p.lean = keyed(t, k, [0.0, -0.35, -0.15, 0.0, 0.0]);
            p.hip = [keyed(t, k, [0.0, -0.25, 0.9, 0.95, 0.85]), keyed(t, k, [0.0, 0.35, 0.7, 0.3, 0.1])];
            p.knee = [keyed(t, k, [0.0, 0.3, 1.5, 1.6, 1.45]), keyed(t, k, [0.0, 0.5, 1.3, 0.6, 0.12])];
            p.shoulder = [keyed(t, k, [0.3, 0.9, 1.2, 0.8, 0.55]), keyed(t, k, [0.3, 1.1, 1.4, 0.6, 0.15])];
            p.elbow = [keyed(t, k, [1.2, 0.6, 0.8, 1.6, 1.95]), keyed(t, k, [1.2, 0.5, 0.4, 0.3, 0.3])];
            p.reach = [keyed(t, k, [0.4, 0.2, 0.3, 0.6, 0.75]), 0.0];
            p.abduct = [0.0, keyed(t, k, [0.0, 0.5, 0.8, 1.0, 1.05])];
            p.prone = keyed(t, k, [0.0, 0.0, -0.15, -0.7, -1.0]);
            p.head = keyed(t, k, [0.0, 0.35, -0.2, -0.5, -0.45]);
            p.look = keyed(t, k, [0.0, 0.0, 0.1, 0.2, 0.25]);
        }
        Pose::FallForward => {
            let k = [0.0, 0.18, 0.45, 0.75, 1.0];
            let t = phase;
            p.sling = keyed(t, k, [0.0, 0.7, 1.0, 1.0, 1.0]);
            p.lean = keyed(t, k, [0.0, -0.25, 0.45, 0.2, 0.0]);
            p.hip = [keyed(t, k, [0.0, 0.1, 1.3, 0.8, 0.15]), keyed(t, k, [0.0, -0.1, 1.1, 0.5, -0.2])];
            p.knee = [keyed(t, k, [0.0, 0.2, 1.7, 1.0, 0.25]), keyed(t, k, [0.0, 0.15, 1.6, 1.0, 0.6])];
            p.shoulder = [keyed(t, k, [0.3, 0.6, 0.4, 1.3, 1.9]), keyed(t, k, [0.3, 0.7, 0.3, 0.8, 0.6])];
            p.elbow = [keyed(t, k, [1.2, 0.8, 0.3, 0.5, 0.6]), keyed(t, k, [1.2, 0.8, 0.3, 0.7, 0.9])];
            p.reach = [0.0, 0.0];
            p.abduct = [keyed(t, k, [0.0, 0.2, 0.3, 0.5, 0.7]), keyed(t, k, [0.0, 0.2, 0.3, 0.6, 0.9])];
            p.prone = keyed(t, k, [0.0, 0.0, 0.0, 0.55, 1.0]);
            p.head = keyed(t, k, [0.0, 0.3, -0.4, 0.2, 0.55]);
            p.look = keyed(t, k, [0.0, 0.0, 0.2, 0.6, 0.9]);
        }
        Pose::DeadBack => {
            p.prone = -1.0;
            p.sling = 1.0;
            p.hip = [0.25, 0.1];
            p.knee = [0.45, 0.15];
            p.shoulder = [0.35, 0.3];
            p.elbow = [0.5, 0.4];
            p.reach = [0.0, 0.0];
            p.abduct = [0.9, 1.0];
            p.head = -0.3;
            p.look = 0.8;
        }
        Pose::Dead => {
            p.prone = 1.0;
            p.sling = 1.0;
            p.hip = [0.15, -0.2];
            p.knee = [0.25, 0.6];
            p.shoulder = [1.9, 0.6];
            p.elbow = [0.6, 0.9];
            p.reach = [0.0, 0.0];
            p.abduct = [0.7, 0.9];
            p.head = 0.55;
            p.look = 0.9;
        }
        Pose::Prone => {
            p.prone = 1.0;
            p.shoulder = [2.8, 2.8];
            p.elbow = [1.3, 1.35];
            p.rifle = std::f32::consts::FRAC_PI_2;
            p.head = 1.0;
        }
        Pose::StandAim => {
            aim_arms(&mut p);
            p.head = AIM_HEAD;
        }
    }
    p.drop = ground_drop_side(p.hip, p.knee, p.side, p.flat);
    p
}

/// Two layers (motion guide rule 1): the legs of one pose (gait, crouch) under the upper body of another (aim,
/// carry), so a soldier can walk while aiming.
pub fn layer(legs: &PoseParams, upper: &PoseParams) -> PoseParams {
    PoseParams { lean: upper.lean, shoulder: upper.shoulder, elbow: upper.elbow, reach: upper.reach, rifle: upper.rifle, rifle_yaw: upper.rifle_yaw, tilt: upper.tilt, twist: legs.twist + upper.twist, head: upper.head, sling: upper.sling, look: upper.look, abduct: upper.abduct, ..*legs }
}

/// The life a stride puts into the body: a rise and fall twice per cycle, the upper body turning against the legs,
/// and the rifle swaying with each footfall (both arms together, so the hands stay on it).
/// `swing`: the rifle swinging across the body with the shoulders (turn around the grip, radians), the side
/// to side motion of a hard run; the head turns against the shoulders so the eyes stay on the way ahead.
fn stride(p: &mut PoseParams, phase: f32, bob: f32, twist: f32, arms: f32, rifle: f32, swing: f32) {
    let t = phase * std::f32::consts::TAU;
    p.bob = bob * (2.0 * t).cos();
    p.twist = twist * t.sin();
    p.look -= 0.8 * p.twist;
    let sway = (2.0 * t + 0.5).sin();
    p.shoulder[0] += arms * sway;
    p.shoulder[1] += arms * sway;
    p.rifle += rifle * (2.0 * t).sin();
    p.rifle_yaw += swing * t.sin();
}

/// Idle life for a soldier standing still (`t` seconds, `seed` 0..1 per soldier, so a squad is never in step):
/// breathing, a slow shift of weight, glances around, and now and then a hitch of the rifle. Moving soldiers get
/// their life from the stride instead; aiming soldiers keep their eyes and rifle on the target.
pub fn idle(p: &PoseParams, pose: Pose, t: f32, seed: f32) -> PoseParams {
    let mut q = *p;
    // Holding the tablet he keeps still: the close-up camera is on it, and any sway smears the screen.
    if matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint | Pose::Prone | Pose::CrouchWalk | Pose::Tablet) {
        return q;
    }
    let tau = std::f32::consts::TAU;
    let breathe = (tau * (t / 3.8 + seed)).sin();
    if matches!(pose, Pose::StandAim | Pose::CoverAim | Pose::LeanLeft | Pose::LeanRight | Pose::CrouchAim) {
        q.shoulder[0] += 0.008 * breathe;
        q.shoulder[1] += 0.008 * breathe;
        q.look += 0.03 * (tau * (t / 7.0 + seed)).sin();
        return q;
    }
    q.shoulder[0] += 0.03 * breathe;
    q.shoulder[1] += 0.03 * breathe;
    q.lean += 0.008 * breathe;
    q.twist += 0.06 * (tau * (t / 9.5 + seed * 1.7)).sin();
    if pose != Pose::Tablet {
        // Mostly looking ahead, now and then a glance to one side (the cube holds it near the middle).
        q.look += 0.6 * (tau * (t / 14.0 + seed * 2.3)).sin().powi(3);
    }
    if pose == Pose::Stand {
        q.rifle += 0.1 * (tau * (t / 12.0 + seed)).sin().max(0.0).powi(12);
    }
    q
}

/// Kneeling: left thigh level with the foot planted ahead, right thigh down with the knee on the ground and the
/// shin lying back; torso upright.
fn kneel(p: &mut PoseParams) {
    p.hip = [1.55, -0.1];
    p.knee = [1.8, 1.75];
    p.lean = 0.12;
}

/// How the rifle is carried (the reference chart's positions); each sets both arms and the rifle angle so the
/// right hand holds the grip and the left the handguard. Angles found by search; tests check both hands.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Carry {
    /// Standing at ease: rifle hanging muzzle down in front of the body.
    SafeHang,
    /// Walking: stock in the shoulder, muzzle 30 degrees down.
    LowReady,
    /// Running: rifle pulled in close, muzzle down.
    Collapsed,
    /// Behind cover: muzzle up, ready to bring down onto a target.
    HighReady,
    /// Combat run: rifle nearly level across the chest, muzzle forward and a little down.
    Rush,
    /// Shooting: shouldered and level.
    ReadyUp,
}

pub fn carry(p: &mut PoseParams, c: Carry) {
    let (shoulder, elbow, reach, abduct, rifle, rifle_yaw) = match c {
        // Solved by examples/carry_ik: right hand on the pistol grip on the right side of the chest, the rifle
        // turned across the body (stock out to the right shoulder, muzzle to the left), left hand reaching
        // across to the handguard, elbows bent and out to the side, arms clear of the torso.
        Carry::SafeHang => ([0.42, -0.44], [0.44, 2.01], [0.54, 1.22], [0.12, 0.57], -0.85, 0.90),
        Carry::LowReady => ([0.55, -0.09], [0.73, 1.92], [0.13, 0.87], [-0.04, 0.53], -0.50, 0.74),
        Carry::Collapsed => ([0.48, -0.39], [0.42, 1.96], [0.17, 1.24], [-0.11, 0.63], -0.75, 0.81),
        Carry::HighReady => ([1.21, 0.12], [0.95, 1.76], [0.43, 0.57], [0.18, 0.51], 0.85, 0.41),
        Carry::Rush => ([0.97, 0.13], [0.52, 1.86], [0.24, 0.80], [0.22, 0.69], -0.20, 0.58),
        Carry::ReadyUp => ([1.13, 0.31], [0.51, 1.84], [0.14, 0.58], [-0.03, 0.50], 0.00, 0.55),
    };
    p.shoulder = shoulder;
    p.elbow = elbow;
    p.reach = reach;
    p.abduct = abduct;
    p.rifle = rifle;
    p.rifle_yaw = rifle_yaw;
}

/// Shouldered rifle: right elbow low with the grip by the chest, left arm reaching across to hold the rifle in
/// front of the magazine (angles found by search; `both_hands_hold_the_rifle_when_aiming` checks them).
fn aim_arms(p: &mut PoseParams) {
    carry(p, Carry::ReadyUp);
    // Bladed stance: the upper body turns right (left shoulder forward) by the rifle's turn across the chest,
    // so the muzzle points straight ahead; the head turns back to look along it.
    p.twist = -p.rifle_yaw;
    p.look = p.rifle_yaw;
}

/// Vertical shift that puts the lowest heel or toe of either leg on the ground.
pub fn ground_drop(hip: [f32; 2], knee: [f32; 2]) -> f32 {
    ground_drop_side(hip, knee, [0.0; 2], 0.0)
}

/// `ground_drop` with the legs swung out to the side and the boots held flat by `flat`.
pub fn ground_drop_side(hip: [f32; 2], knee: [f32; 2], side: [f32; 2], flat: f32) -> f32 {
    let mut low = f32::MAX;
    for i in 0..2 {
        let px = if i == 0 { -HIP_X } else { HIP_X };
        for y in [HEEL_Y, TOE_Y] {
            low = low.min(sideways(foot(Vec3::new(px, y, 0.0), hip[i], knee[i], flat), side[i], px).z);
        }
    }
    -low
}

/// Moves one vertex of a figure part into the pose (in the figure's frame, before heading and position).
pub fn pose_vertex(v: Vec3, part: Part, p: &PoseParams) -> Vec3 {
    let mut q = match part {
        Part::Body | Part::Sway | Part::Glow => return v,
        Part::LeftThigh => sideways(rot(v, 0.0, HIP_Z, p.hip[0]), p.side[0], -HIP_X),
        Part::RightThigh => sideways(rot(v, 0.0, HIP_Z, p.hip[1]), p.side[1], HIP_X),
        Part::LeftShin => sideways(leg(v, p.hip[0], p.knee[0]), p.side[0], -HIP_X),
        Part::RightShin => sideways(leg(v, p.hip[1], p.knee[1]), p.side[1], HIP_X),
        Part::LeftFoot => sideways(foot(v, p.hip[0], p.knee[0], p.flat), p.side[0], -HIP_X),
        Part::RightFoot => sideways(foot(v, p.hip[1], p.knee[1], p.flat), p.side[1], HIP_X),
        Part::LeftArm => arm(v, p.shoulder[0], 0.0, p.reach[0], p.abduct[0], -SHOULDER_X),
        Part::RightArm => arm(v, p.shoulder[1], 0.0, p.reach[1], p.abduct[1], SHOULDER_X),
        Part::LeftForearm => arm(v, p.shoulder[0], p.elbow[0], p.reach[0], p.abduct[0], -SHOULDER_X),
        Part::RightForearm => arm(v, p.shoulder[1], p.elbow[1], p.reach[1], p.abduct[1], SHOULDER_X),
        Part::Rifle => {
            let h = held(v, p);
            if p.sling > 0.0 { h.lerp(slung(v), p.sling) } else { h }
        }
        Part::Head => turn(rot(v, 0.0, NECK_Z, p.head), p.look),
        Part::Torso => v,
    };
    if !matches!(part, Part::LeftThigh | Part::RightThigh | Part::LeftShin | Part::RightShin | Part::LeftFoot | Part::RightFoot) {
        q = rot(turn(q, p.twist), 0.0, HIP_Z, -p.lean);
        q = tilt(q, p.tilt);
    }
    q.z += p.drop + p.bob;
    if p.prone != 0.0 {
        // Positive tips him forward onto his front, negative back onto his back.
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2 * p.prone);
        // Mid-transition the bent knees would dip into the ground: lift a little, most at the halfway point.
        let a = p.prone.abs();
        q.z += if p.prone > 0.0 { PRONE_LIFT } else { SUPINE_LIFT } * a + 0.12 * (std::f32::consts::PI * a).sin();
    }
    q
}

/// Turns a point around the vertical through the figure's centre line (positive to the left).
fn turn(q: Vec3, a: f32) -> Vec3 {
    if a == 0.0 {
        return q;
    }
    let (s, c) = a.sin_cos();
    Vec3::new(q.x * c - q.y * s, q.x * s + q.y * c, q.z)
}

/// Linear blend between two poses (`w` 0 gives `a`, 1 gives `b`); the shader blends over `BLEND_S` after a change.
pub fn blend(a: &PoseParams, b: &PoseParams, w: f32) -> PoseParams {
    // Eased: pose changes start and settle gently (motion guide rule 5). The ends are exact.
    if w <= 0.0 {
        return *a;
    }
    if w >= 1.0 {
        return *b;
    }
    let w = w.clamp(0.0, 1.0);
    let w = w * w * (3.0 - 2.0 * w);
    let m = |x: f32, y: f32| x + (y - x) * w;
    let m2 = |x: [f32; 2], y: [f32; 2]| [m(x[0], y[0]), m(x[1], y[1])];
    let (hip, knee) = (m2(a.hip, b.hip), m2(a.knee, b.knee));
    let side = m2(a.side, b.side);
    let flat = m(a.flat, b.flat);
    let drop = if w <= 0.0 { a.drop } else if w >= 1.0 { b.drop } else { ground_drop_side(hip, knee, side, flat) };
    PoseParams {
        lean: m(a.lean, b.lean),
        hip,
        knee,
        side,
        flat,
        shoulder: m2(a.shoulder, b.shoulder),
        elbow: m2(a.elbow, b.elbow),
        reach: m2(a.reach, b.reach),
        head: m(a.head, b.head),
        rifle: m(a.rifle, b.rifle),
        rifle_yaw: m(a.rifle_yaw, b.rifle_yaw),
        tilt: m(a.tilt, b.tilt),
        prone: m(a.prone, b.prone),
        drop,
        sling: m(a.sling, b.sling),
        bob: m(a.bob, b.bob),
        twist: m(a.twist, b.twist),
        look: m(a.look, b.look),
        abduct: m2(a.abduct, b.abduct),
    }
}

/// Seconds a pose change takes.
pub const BLEND_S: f32 = 0.35;
