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
pub const PRONE_LIFT: f32 = 0.14;
/// The lift through the move onto his front, before he settles flat (see `pose_vertex`).
pub const PRONE_LIFT_MID: f32 = 0.23;
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
    /// Rifle slung on the back, arms loose at the sides (weapon holstered).
    Slung = 28,
    /// Combat ready (low ready): athletic stance, feet staggered and knees flexed, hips back and chest over the
    /// toes, stock in the shoulder, muzzle 30-40 degrees down, eyes up.
    Ready = 30,
    /// Hard stop: the left leg shot forward and out to the side, the right bent behind, body leaning back
    /// against the slide, rifle still at the ready.
    Brake = 31,
    /// Crawling on the belly: arms and legs working in turn (phase = the crawl cycle), rifle carried forward.
    Crawl = 32,
    /// Getting down to prone over the phase (0 standing at the ready, `DROP_KNEEL` down on a knee, 1 flat), played
    /// backwards to get up: a knee to the ground, the left hand planted ahead, then down onto the front. Built from
    /// the ready, the kneel and prone (`drop_params`), so it always lands exactly on them.
    Drop = 33,
    /// The drop's middle keys (`drop_params`): over the knee with the left palm reaching for the ground, and on
    /// both knees with the left palm planted. Drawn only as keys of the drop.
    DropReach = 34,
    DropPlant = 35,
    /// Prone, rifle shouldered and held on a target; can also be layered over the crawl legs.
    ProneAim = 36,
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
            28 => Pose::Slung,
            30 => Pose::Ready,
            31 => Pose::Brake,
            32 => Pose::Crawl,
            33 => Pose::Drop,
            34 => Pose::DropReach,
            35 => Pose::DropPlant,
            36 => Pose::ProneAim,
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
    /// Pelvis turn: the hips and both legs turned around the vertical through the hips (radians, positive to the
    /// left), the upper body and the rifle staying on the facing. Riflemen walk and strafe with the hips turned
    /// towards their steps and the chest on the aim (AN-02-S13, from the reference clips).
    pub yaw: f32,
    /// Height added after the body tips prone (metres): the high crawl holds the body off the ground on elbows and
    /// knees. Applied to every part; 0 in every other pose.
    pub raise: f32,
    /// Extra turn of each leg about its hip, on top of `leg_roll_angle`'s (radians, as that angle): the high crawl
    /// turns a thigh out while its shin runs back along the ground. 0 in every other pose.
    pub roll: [f32; 2],
    /// Each boot's roll about its ankle (radians, positive lifts the toe): heel strike, toe-off and the toe hanging
    /// in the swing (Patch: Animations 2). The baked gaits carry it with their legs solved to match (`polish_loop`).
    pub toe: [f32; 2],
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
    // Prone support reaches farther across the body than the standing carry; allow the arm to extend fully.
    let prone = p.prone.clamp(0.0, 1.0);
    let max = GRIP_MAX + (PRONE_GRIP_MAX - GRIP_MAX) * prone;
    reach_left_capped(p, held(RIFLE_GUARD, p), w, max)
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
    // As the torso lowers, let the support arm use its full reach instead of damping away the correction.
    let damp = GRIP_DAMP + 2.0 * excess * (1.0 - p.prone.clamp(0.0, 1.0));
    // Unknowns: left shoulder, elbow and reach.
    let apply = |q: &PoseParams, x: [f32; 4]| {
        let mut t = *q;
        t.shoulder[0] += x[0];
        t.elbow[0] += x[1];
        t.reach[0] += x[2];
        t.abduct[0] += x[3];
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
        let j: [Vec3; 4] = std::array::from_fn(|k| {
            let mut x = [0.0; 4];
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
        let x: [f32; 4] = std::array::from_fn(|k| (-j[k].dot(y)).clamp(-0.5, 0.5));
        q = apply(&q, x);
    }
    // Soft cap on each joint's total correction.
    let cap = |d: f32| max * (d / max).tanh() * w;
    let mut out = *p;
    out.shoulder[0] += cap(q.shoulder[0] - p.shoulder[0]);
    out.elbow[0] += cap(q.elbow[0] - p.elbow[0]);
    out.reach[0] += cap(q.reach[0] - p.reach[0]);
    out.abduct[0] += cap(q.abduct[0] - p.abduct[0]);
    out
}
const GRIP_STEPS: usize = 16;
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
/// Larger correction budget for the support arm in the prone firing position.
const PRONE_GRIP_MAX: f32 = 4.0;

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

/// Moves the left palm along the reload path, blended by weight so reloads can enter and leave over a pose blend.
pub fn reload_hand(p: &PoseParams, phase: f32, weight: f32) -> PoseParams {
    reach_left(p, reload_target(p, phase), weight)
}

/// The magazine well, a rifle point (rest pose): under the receiver just ahead of the grip.
pub const RIFLE_WELL: Vec3 = Vec3::new(0.238, 0.11, 0.70);
/// The magazine pouch on the front of the vest, left of centre (figure frame).
pub const RELOAD_POUCH: Vec3 = Vec3::new(-0.04, 0.17, 1.08);

/// 1 for poses that hold the rifle with both hands (left palm on the handguard), else 0.
pub fn two_handed(pose: Pose) -> f32 {
    match pose {
        Pose::Stand | Pose::Walk | Pose::Run | Pose::Sprint | Pose::Crouch | Pose::CoverAim | Pose::Cover | Pose::StandAim | Pose::CrouchWalk | Pose::LeanLeft | Pose::LeanRight | Pose::CrouchAim | Pose::Stack | Pose::Sweep | Pose::Ready | Pose::Brake | Pose::Prone | Pose::ProneAim => 1.0,
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

/// Lying on his front the feet point back, boots flat on the ground, so the toes do not prop the body up.
pub const PRONE_POINT: f32 = -1.2;
/// Upper-body lean lying prone: back, so the chest rises off the ground onto the elbows (was `PRONE_LEAN_OLD`).
pub const PRONE_LEAN: f32 = -0.28;
/// The same, aiming: a little more, so the support forearm clears the ground under the rifle.
pub const PRONE_AIM_LEAN: f32 = -0.30;
const PRONE_LEAN_OLD: f32 = -0.1;

/// How far he has settled flat, 0 to 1: nothing through the move onto his front, all of it at the end, so
/// the dive and the get-up keep their shape and only the fully prone body lies flat.
pub fn prone_settle(a: f32) -> f32 {
    a.powi(12)
}

/// Leg roll lying prone (AN-02-S10): each leg turns about its own long axis, up to a quarter turn as the knee
/// draws up (`LEG_ROLL_FROM` to `LEG_ROLL_FULL` knee bend), so the bent knee folds sideways along the ground
/// instead of lifting the shin into the air. Nothing until fully settled prone (`prone_settle`), so standing,
/// kneeling and the drop are unchanged; straight prone legs (knee under `LEG_ROLL_FROM`) are not rolled.
pub fn leg_roll_angle(p: &PoseParams, i: usize) -> f32 {
    let t = ((p.knee[i] - LEG_ROLL_FROM) / (LEG_ROLL_FULL - LEG_ROLL_FROM)).clamp(0.0, 1.0);
    // Only lying flat: raised on the knees (the high crawl, `raise`) the legs kneel straight down.
    let flat = 1.0 - (p.raise / LEG_ROLL_RAISE).clamp(0.0, 1.0);
    let r = std::f32::consts::FRAC_PI_2 * prone_settle(p.prone.max(0.0)) * t * t * (3.0 - 2.0 * t) * flat;
    // Inward: the bent knee tucks the foot in behind it, as in the reference crawl (fits the reference to 7 cm,
    // against 30 cm turning outward).
    (if i == 0 { r } else { -r }) + p.roll[i]
}
pub const LEG_ROLL_FROM: f32 = 0.2;
pub const LEG_ROLL_FULL: f32 = 0.4;
/// Body raise (m) at which the leg roll is gone (`leg_roll_angle`).
pub const LEG_ROLL_RAISE: f32 = 0.1;

/// A leg point turned by `leg_roll_angle` about the vertical through its hip (the leg's own axis when straight).
fn leg_roll(q: Vec3, p: &PoseParams, i: usize) -> Vec3 {
    let a = leg_roll_angle(p, i);
    if a == 0.0 { return q; }
    let x0 = if i == 0 { -HIP_X } else { HIP_X };
    let (s, c) = a.sin_cos();
    let dx = q.x - x0;
    Vec3::new(x0 + dx * c - q.y * s, dx * s + q.y * c, q.z)
}

/// A foot point: ankle bend, then the leg. `prone` points the foot back (`PRONE_POINT`) as he lies down.
fn foot(q: Vec3, hip: f32, knee: f32, flat: f32, prone: f32, toe: f32) -> Vec3 {
    leg(rot(q, 0.0, ANKLE_Z, ankle_flat(hip, knee, flat) + PRONE_POINT * prone_settle(prone.max(0.0)) + toe), hip, knee)
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
    let row = match pose { Pose::Walk => 0, Pose::Run => 1, Pose::Sprint => 2, _ => return stride_m(pose) * stride_scale(dir) };
    let (i, j, w) = gait_sector(dir);
    GAIT_STRIDE[row][i] + (GAIT_STRIDE[row][j] - GAIT_STRIDE[row][i]) * w
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
        Pose::Crawl => CRAWL_STRIDE,
        _ => 1.0,
    }
}
/// One crawl cycle (both arms) moves him this far (m).
pub const CRAWL_STRIDE: f32 = crate::crawl::STRIDE;
pub const WALK_STRIDE: f32 = GAIT_STRIDE[0][0];
pub const RUN_STRIDE: f32 = GAIT_STRIDE[1][0];
pub const SPRINT_STRIDE: f32 = GAIT_STRIDE[2][0];
/// The crouched walk is still procedural: the reference crouch's long, deep steps do not fit our rig without
/// sliding or popping (AN-02-S13).
pub const CROUCH_STRIDE: f32 = 1.0;

/// Pose parameters at a gait phase (0..1; only the gaits use it). Rifle carries per pose (rule 3): low ready when
/// walking, high port when running, angled down across the body when sprinting, shouldered when aiming.
pub fn params(pose: Pose, phase: f32) -> PoseParams {
    params_dir(pose, phase, 0.0)
}

/// Interpolates a baked clip table (`scripts/clip_table.py`) at `phase`: one loop, wrapping at 1.0.
pub fn clip_at<const F: usize>(t: &[[f32; F]], phase: f32) -> [f32; F] {
    let n = t.len();
    let x = phase.rem_euclid(1.0) * n as f32;
    let i0 = (x.floor() as usize) % n;
    let i1 = (i0 + 1) % n;
    let f = x - x.floor();
    std::array::from_fn(|k| t[i0][k] + (t[i1][k] - t[i0][k]) * f)
}

/// Baked rifle gaits (AN-02-S13): one loop per eighth of a turn, from the Pro Rifle Pack clips, in the order
/// forward, forward-left, left, back-left, back, back-right, right, forward-right (`dir` turns towards the left).
/// Every loop starts at the left heel strike, so neighbours line up phase for phase.
/// Rows per baked gait loop (scripts: `clip_ref.py ... 48 --strike`).
pub const GAIT_ROWS: usize = 48;
type GaitLoop = [[f32; 11]; GAIT_ROWS];
fn gait_loops(pose: Pose) -> Option<[&'static GaitLoop; 8]> {
    gait_polished(pose).map(|l| std::array::from_fn(|i| &l[i].0))
}

/// The boot roll that goes with each of `gait_loops` (same order).
fn gait_toes(pose: Pose) -> Option<[&'static ToeLoop; 8]> {
    gait_polished(pose).map(|l| std::array::from_fn(|i| &l[i].1))
}

/// The baked loops, polished, with their boot roll.
fn gait_polished(pose: Pose) -> Option<&'static [(GaitLoop, ToeLoop); 8]> {
    static WALK: std::sync::OnceLock<[(GaitLoop, ToeLoop); 8]> = std::sync::OnceLock::new();
    static RUN: std::sync::OnceLock<[(GaitLoop, ToeLoop); 8]> = std::sync::OnceLock::new();
    static SPRINT: std::sync::OnceLock<[(GaitLoop, ToeLoop); 8]> = std::sync::OnceLock::new();
    let (cell, raw) = match pose {
        Pose::Walk => (&WALK, [&WALK_F, &WALK_FL, &WALK_L, &WALK_BL, &WALK_B, &WALK_BR, &WALK_R, &WALK_FR]),
        Pose::Run => (&RUN, [&RUN_F, &RUN_FL, &RUN_L, &RUN_BL, &RUN_B, &RUN_BR, &RUN_R, &RUN_FR]),
        Pose::Sprint => (&SPRINT, [&SPRINT_F, &SPRINT_FL, &SPRINT_L, &SPRINT_BL, &SPRINT_B, &SPRINT_BR, &SPRINT_R, &SPRINT_FR]),
        _ => return None,
    };
    // Ahead and back the legs are straightened; the strafes keep their hip turn (it is how they step sideways).
    let rise = GAIT_RISE[match pose { Pose::Walk => 0, Pose::Run => 1, _ => 2 }];
    let strafe = PLANT_STRAFE[match pose { Pose::Walk => 0, Pose::Run => 1, _ => 2 }];
    let roll = FOOT_ROLL[match pose { Pose::Walk => 0, Pose::Run => 1, _ => 2 }];
    let loops = cell.get_or_init(|| {
        let mut i = 0;
        raw.map(|l| {
            let straight = i % 4 == 0;
            // Only going ahead stands taller: backing up and the strafes reach the solver's knee limit with it.
            let up = if i == 0 { rise } else { 0.0 };
            i += 1;
            if straight { polish_loop(&centre_leg_yaw(&smooth_lift(l)), true, PLANT_STRAIGHT, up, roll) } else { polish_loop(&smooth_lift(l), false, strafe, 0.0, roll) }
        })
    });
    Some(loops)
}

/// Where a leg's boot is for a baked row (legs frame): the ankle, which a planted boot pivots about.
fn gait_boot(hip: f32, knee: f32, side: f32, yaw: f32, px: f32) -> Vec3 {
    turn(sideways(leg(Vec3::new(px, 0.0, ANKLE_Z), hip, knee), side, px), yaw)
}

/// Rows of smoothing (a circular Gaussian's sigma) applied to the boots' paths and the body's sway in a baked loop:
/// it takes out the capture's frame-to-frame noise (legs that stutter, a jump at the loop seam) and only rounds the
/// corners of a planted boot's path, which stays put (a fixed point smooths to itself).
const GAIT_SMOOTH_ROWS: f32 = 1.0;
/// How far off the planted speed (in units of it) a boot's smoothing starts and is full: going straight, and per gait
/// on the strafes, which plant less cleanly (the step crosses over) and so keep more of their path as captured.
const PLANT_STRAIGHT: [f32; 2] = [0.2, 0.6];
/// Walk polish: how much taller (m) each gait stands than its clip. The rifle walk kept the knees bent past 40
/// degrees the whole way round (a crouched waddle on our figure); walk, run, sprint.
const GAIT_RISE: [f32; 3] = [0.05, 0.0, 0.0];
const PLANT_STRAFE: [[f32; 2]; 3] = [[0.2, 0.6], [0.4, 1.0], [0.6, 1.4]];

/// Walk polish: a baked loop with the boots' paths smoothed and, going straight (`mirror`), each boot made the other's
/// mirror half a cycle on. The rifle clips lead with the rifle-side leg, which on our figure read as a limp (the
/// legs differed by up to 0.6 rad). The work is done on where the boots are, then the legs are solved to put them
/// there, so a planted boot stays planted. The body's sway follows suit: odd fields (legs turn, twist, tilt) mirror
/// about their mean, even ones (lean, lift) average with half a cycle on.
fn polish_loop(l: &GaitLoop, mirror: bool, plant: [f32; 2], rise: f32, roll: [f32; 3]) -> (GaitLoop, ToeLoop) {
    let n = GAIT_ROWS;
    let h = n / 2;
    let px = [-HIP_X, HIP_X];
    let flip = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
    let feet: Vec<[Vec3; 2]> = l.iter().map(|r| [0, 1].map(|i| gait_boot(r[i], r[2 + i], r[4 + i], r[6], px[i]))).collect();
    // `rise`: the boots asked for this much lower against the hips, so the legs straighten and the body stands taller.
    let feet: Vec<[Vec3; 2]> = feet.iter().map(|f| f.map(|v| v - Vec3::Z * rise)).collect();
    let mut want = feet.clone();
    let mut sway: Vec<[f32; 5]> = l.iter().map(|r| [r[6], r[7], r[8], r[9], r[10]]).collect();
    if mirror {
        let mean: Vec<f32> = (0..5).map(|f| sway.iter().map(|s| s[f]).sum::<f32>() / n as f32).collect();
        // The clips' right heel strike is not exactly half a cycle after the left: line the right boot's path up
        // with the left's first, so a planted boot is only ever averaged with a planted boot.
        let lag = (0..n)
            .min_by(|&a, &b| {
                let miss = |o: usize| (0..n).map(|k| (feet[k][0] - flip(feet[(k + o) % n][1])).length_squared()).sum::<f32>();
                miss(a).total_cmp(&miss(b))
            })
            .unwrap();
        let left: Vec<Vec3> = (0..n).map(|k| (feet[k][0] + flip(feet[(k + lag) % n][1])) * 0.5).collect();
        for k in 0..n {
            let o = (k + h) % n;
            want[k] = [left[k], flip(left[(k + n - h) % n])];
            for f in 0..5 {
                let (a, b) = (l[k][6 + f], l[o][6 + f]);
                sway[k][f] = if matches!(f, 1 | 4) { (a + b) * 0.5 } else { mean[f] + (a - b) * 0.5 };
            }
        }
    }
    let kernel: Vec<f32> = (-4..=4).map(|d: i32| (-(d * d) as f32 / (2.0 * GAIT_SMOOTH_ROWS * GAIT_SMOOTH_ROWS)).exp()).collect();
    let total: f32 = kernel.iter().sum();
    let at = |k: usize, d: usize| (k + n + d - 4) % n;
    let smoothed: Vec<[Vec3; 2]> =
        (0..n).map(|k| [0, 1].map(|i| (0..9).map(|d| want[at(k, d)][i] * kernel[d]).sum::<Vec3>() / total)).collect();
    // A planted boot sweeps back through the legs frame at the body's speed (the commonest velocity); it is kept
    // exactly. Away from that speed (lift-off, the swing, landing) the path is smoothed: the clips' ankle turns from
    // sweeping back to swinging forward within one row at toe-off, which read as a snap in every step.
    let airborne: Vec<[f32; 2]> = {
        let vel = |k: usize, i: usize| want[(k + 1) % n][i] - want[k][i];
        let off: Vec<[f32; 2]> = (0..n)
            .map(|k| {
                [0, 1].map(|i| {
                    let mut c: Vec<[f32; 3]> = (0..n).map(|j| vel(j, i).to_array()).collect();
                    let planted = Vec3::from_array([0, 1, 2].map(|a| {
                        c.sort_by(|x, y| x[a].total_cmp(&y[a]));
                        c[n / 2][a]
                    }));
                    let unit = planted.length().max(1e-4);
                    // Either step out of this row (to it, from it) off the planted speed counts.
                    let d = (vel(k, i) - planted).length().max((vel((k + n - 1) % n, i) - planted).length());
                    smooth(plant[0] * unit, plant[1] * unit, d)
                })
            })
            .collect();
        off
    };
    let target: Vec<[Vec3; 2]> = (0..n).map(|k| [0, 1].map(|i| want[k][i] + (smoothed[k][i] - want[k][i]) * airborne[k][i])).collect();
    let (toe, lift) = foot_roll(&target, &sway, roll, mirror);
    let mut out = *l;
    // Each row's solve starts from the row before (twice round, so the seam starts from the loop's own end): left
    // to start from the clip's own angles, the solver can land on another solution and the leg jumps.
    let mut prev = [0, 1].map(|i| Vec3::new(l[0][i], l[0][2 + i], l[0][4 + i]));
    for pass in 0..2 {
        for k in 0..n {
            let foot = |i: usize| target[k][i] + lift[k][i];
            let r = &mut out[k];
            if pass == 0 {
                for f in 0..5 {
                    r[6 + f] = sway[k][f];
                }
            }
            for i in 0..2 {
                // Strafes keep the clip's own angles as the start (their legs cross, and chaining can drift).
                let start = if mirror { prev[i] } else { Vec3::new(l[k][i], l[k][2 + i], l[k][4 + i]) };
                // The leg turns about its hip by the row's `roll` so the planted boot keeps its heading.
                let tw = toe[k][2 + i];
                let boot = |h: f32, kn: f32, sd: f32, yw: f32, x: f32| turn(sideways(roll_about(leg(Vec3::new(x, 0.0, ANKLE_Z), h, kn), tw, x), sd, x), yw);
                let v = solve_boot(&boot, foot(i), start, r[6], px[i]);
                prev[i] = v;
                r[i] = v.x;
                r[2 + i] = v.y;
                r[4 + i] = v.z;
            }
        }
    }
    (out, toe)
}

/// A baked loop's boot roll per row, left and right (radians, positive lifts the toe), then each leg's turn about
/// its hip (`PoseParams::roll`) that keeps a planted boot's heading while the pelvis turns through the stride.
pub type ToeLoop = [[f32; 4]; GAIT_ROWS];

/// A leg point turned by `a` about the vertical through its hip at `x0` (as `leg_roll`).
fn roll_about(q: Vec3, a: f32, x0: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    let dx = q.x - x0;
    Vec3::new(x0 + dx * c - q.y * s, dx * s + q.y * c, q.z)
}

/// Where the boot pivots as the heel lifts, ahead of the ankle (m): its toe end. The boot is rigid, so rolling about
/// the ball would push the toe end into the ground (and skid it).
pub const BALL_Y: f32 = TOE_Y;
/// Boot roll per gait (radians): toe up at heel strike, heel up at toe-off, toe hanging mid-swing; walk, run, sprint.
/// Share of `FOOT_ROLL` the strafing loops get.
pub const STRAFE_ROLL: f32 = 0.5;
pub const FOOT_ROLL: [[f32; 3]; 3] = [[0.21, -0.61, -0.17], [0.14, -0.70, -0.26], [0.05, -0.60, -0.30]];

/// Foot roll for a polished loop (Patch: Animations 2): per row, each boot's roll and how far its ankle must move so
/// the point it rolls on stays where it is. Planted (the longest run of rows the boot holds still), it strikes on
/// the heel toe-up and rolls flat, then lifts the heel and pivots on the ball to toe-off; in the air the toe hangs
/// and comes up for the next strike. Going backwards the boot lands on the ball and leaves from the heel; stepping
/// sideways it rolls less, by how far the step runs along the boot.
fn foot_roll(target: &[[Vec3; 2]], sway: &[[f32; 5]], roll: [f32; 3], straight: bool) -> (ToeLoop, Vec<[Vec3; 2]>) {
    let n = GAIT_ROWS;
    let mut toe = [[0.0f32; 4]; GAIT_ROWS];
    let mut lift = vec![[Vec3::ZERO; 2]; n];
    let ease = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    for i in 0..2 {
        // Planted: the boot at its lowest (a runner's boot is in the air most of the time, so not its commonest speed).
        let low = (0..n).map(|k| target[k][i].z).fold(f32::MAX, f32::min);
        let planted: Vec<bool> = (0..n).map(|k| target[k][i].z < low + 0.03).collect();
        // The longest planted run (round the loop): its first and last rows.
        let mut best = (0, 0);
        for s0 in 0..n {
            if planted[s0] && !planted[(s0 + n - 1) % n] {
                let len = (0..n).take_while(|d| planted[(s0 + d) % n]).count();
                if len > best.1 {
                    best = (s0, len);
                }
            }
        }
        let (s0, len) = best;
        if len < 4 || len == n {
            continue;
        }
        // Which way the planted boot sweeps against the way it points: back (walking ahead) or forward (backing up).
        let sweep = target[(s0 + len - 1) % n][i] - target[s0][i];
        let fwd = |_: usize| turn(Vec3::Y, sway[(s0 + len / 2) % n][0]);
        let along = sweep.dot(fwd(s0)) / sweep.length().max(1e-4);
        let (k_on, k_off) = (-along.min(0.0), along.max(0.0));
        // Strafing steps (legs crossing) roll half as much: flatter-footed, and their plant is looser.
        let gain = along.abs() * if straight { 1.0 } else { STRAFE_ROLL };
        let (strike, off, hang) = (roll[0] * gain, roll[1] * gain, roll[2] * gain);
        // Ahead: heel first, ball last. Backwards: ball first, heel last.
        let (land_y, leave_y, land, leave) = if k_on >= k_off { (HEEL_Y, BALL_Y, strike, off) } else { (BALL_Y, HEEL_Y, off * 0.6, roll[0].max(0.2) * 1.5 * gain) };
        let a = (0.15 * len as f32).max(1.0);
        let b = (0.35 * len as f32).max(1.0);
        let mut theta = vec![0.0f32; n];
        let mut pivot_y = vec![0.0f32; n];
        for d in 0..n {
            let k = (s0 + d) % n;
            let d = d as f32;
            let l = len as f32;
            theta[k] = if d < l {
                if d < a {
                    pivot_y[k] = land_y;
                    land * (1.0 - ease(d / a))
                } else if d > l - 1.0 - b {
                    pivot_y[k] = leave_y;
                    leave * ease((d - (l - 1.0 - b)) / b)
                } else {
                    0.0
                }
            } else {
                // In the air: from the toe-off roll to the hang, then up to the next strike.
                let t = (d - l + 1.0) / (n as f32 - l + 1.0);
                if t < 0.5 { leave + (hang - leave) * ease(t * 2.0) } else { hang + (land - hang) * ease(t * 2.0 - 1.0) }
            };
        }
        // Planted: the ankle moves so the rolling point stays put. In the air the move eases from lift-off's to
        // the landing's.
        let shift = |k: usize| {
            let f = fwd(k);
            let o = f * pivot_y[k] - Vec3::Z * ANKLE_Z;
            let q = glam::Quat::from_axis_angle(f.cross(Vec3::Z).normalize(), theta[k]);
            o + q * -o - Vec3::ZERO
        };
        // Heading: planted, the leg turns against the pelvis so the boot points where it landed pointing (mid-stance
        // heading, so the hold is even either side); in the air it eases from lift-off's turn to the landing's.
        let mid = sway[(s0 + len / 2) % n][0];
        let hold = |k: usize| (mid - sway[k][0]) * gain;
        let (h_end, h_land) = (hold((s0 + len - 1) % n), hold(s0));
        for d in 0..n {
            let k = (s0 + d) % n;
            toe[k][2 + i] = if d < len {
                hold(k)
            } else {
                let t = (d - len + 1) as f32 / (n - len + 1) as f32;
                h_end + (h_land - h_end) * ease(t)
            };
        }
        let (k_end, k_land) = ((s0 + len - 1) % n, (s0 + n) % n);
        let (from, to) = (shift(k_end), shift(k_land));
        for d in 0..n {
            let k = (s0 + d) % n;
            toe[k][i] = theta[k];
            lift[k][i] = if d < len {
                shift(k)
            } else {
                let t = (d - len + 1) as f32 / (n - len + 1) as f32;
                from + (to - from) * ease(t)
            };
        }
    }
    // A light pass round the loop over both (the same weights, so the rolling point still stays put): the roll's
    // starts and stops would otherwise kink the solved legs.
    let k: Vec<f32> = (-3..=3).map(|d: i32| (-(d * d) as f32 / 2.0).exp()).collect();
    let sum: f32 = k.iter().sum();
    let at = |j: usize, d: usize| (j + n + d - 3) % n;
    let toe_s: ToeLoop = std::array::from_fn(|j| [0, 1, 2, 3].map(|i| (0..7).map(|d| toe[at(j, d)][i] * k[d]).sum::<f32>() / sum));
    let lift_s: Vec<[Vec3; 2]> = (0..n).map(|j| [0, 1].map(|i| (0..7).map(|d| lift[at(j, d)][i] * k[d]).sum::<Vec3>() / sum)).collect();
    (toe_s, lift_s)
}

/// A baked loop with its body lift (field 10) smoothed round the cycle. The clips' lift steps up for the airborne
/// part of a stride and down again at landing between one row and the next; under the procedural legs that is a
/// 6 cm pop on every step. Both the CPU (`gait_bake`) and the shader (`clip_buffer`) read the smoothed loops.
/// A baked loop with the legs' average turn (field 6) taken out, keeping its swing, and each foot kept exactly where
/// the clip puts it: the rifle clips walk and run with the hips turned about 35 degrees towards the rifle side, which
/// on our figure reads as legs tilted off to the right. With the turn gone, each row's hip swing and side swing
/// (fields 0..1, 4..5) are solved again so the boot lands in the same place (knee kept): straight legs, no slide.
/// How much of the rifle clips' average leg turn is taken out (the rest keeps the longest strides within reach).
const LEG_YAW_REMOVED: f32 = 1.0;
fn centre_leg_yaw(l: &GaitLoop) -> GaitLoop {
    let mean = l.iter().map(|r| r[6]).sum::<f32>() / l.len() as f32;
    let boot = |hip: f32, knee: f32, side: f32, yaw: f32, px: f32| {
        // The ankle: a planted boot pivots about it.
        turn(sideways(leg(Vec3::new(px, 0.0, ANKLE_Z), hip, knee), side, px), yaw)
    };
    let mut out = *l;
    for row in out.iter_mut() {
        let yaw = row[6] - LEG_YAW_REMOVED * mean;
        for i in 0..2 {
            let px = if i == 0 { -HIP_X } else { HIP_X };
            let want = boot(row[i], row[2 + i], row[4 + i], row[6], px);
            let solved = solve_boot(&boot, want, Vec3::new(row[i], row[2 + i], row[4 + i]), yaw, px);
            row[i] = solved.x;
            row[2 + i] = solved.y;
            row[4 + i] = solved.z;
        }
        row[6] = yaw;
    }
    out
}

/// (hip, knee, side) close to `start` that put the boot at `want` with the legs turned `yaw`: first hip and side
/// for the ground position (always solvable), then all three in small checked steps for the height too. Steps that
/// do not help are refused, and a knee left nearly straight (no say over the height) keeps the first solution.
fn solve_boot(boot: &dyn Fn(f32, f32, f32, f32, f32) -> Vec3, want: Vec3, start: Vec3, yaw: f32, px: f32) -> Vec3 {
    let at = |v: Vec3| boot(v.x, v.y, v.z, yaw, px);
    let e = 1e-3;
    let mut v = start;
    for _ in 0..12 {
        let p = at(v);
        let (ex, ey) = (want.x - p.x, want.y - p.y);
        if ex * ex + ey * ey < 1e-10 {
            break;
        }
        let dh = (at(v + Vec3::X * e) - p) / e;
        let ds = (at(v + Vec3::Z * e) - p) / e;
        let det = dh.x * ds.y - ds.x * dh.y;
        if det.abs() < 1e-6 {
            break;
        }
        v.x += ((ex * ds.y - ds.x * ey) / det).clamp(-0.2, 0.2);
        v.z += ((dh.x * ey - ex * dh.y) / det).clamp(-0.2, 0.2);
    }
    let ground = v;
    for _ in 0..16 {
        let p = at(v);
        let err = want - p;
        if err.length_squared() < 1e-10 {
            break;
        }
        let j = glam::Mat3::from_cols((at(v + Vec3::X * e) - p) / e, (at(v + Vec3::Y * e) - p) / e, (at(v + Vec3::Z * e) - p) / e);
        if j.determinant().abs() < 1e-5 {
            break;
        }
        let mut step = j.inverse() * err;
        step *= (0.1 / step.abs().max_element().max(1e-9)).min(1.0);
        let mut improved = false;
        for _ in 0..6 {
            let mut next = v + step;
            // Never straighter than the clip's own knee or 0.35 rad (a planted knee stays bent at a run).
            next.y = next.y.clamp(start.y.min(0.35), 2.6);
            if (want - at(next)).length_squared() < err.length_squared() {
                v = next;
                improved = true;
                break;
            }
            step *= 0.5;
        }
        if !improved {
            break;
        }
    }
    if (v - ground).abs().max_element() > 1.2 { ground } else { v }
}

fn smooth_lift(l: &GaitLoop) -> GaitLoop {
    const SIGMA_ROWS: f32 = 1.5;
    let n = l.len() as i32;
    let mut out = *l;
    for (i, row) in out.iter_mut().enumerate() {
        let (mut sum, mut wsum) = (0.0, 0.0);
        for k in -4i32..=4 {
            let w = (-0.5 * (k as f32 / SIGMA_ROWS).powi(2)).exp();
            sum += w * l[(i as i32 + k).rem_euclid(n) as usize][10];
            wsum += w;
        }
        row[10] = sum / wsum;
    }
    out
}

/// Ground covered per cycle by each baked loop (m), in `gait_loops` order, rows Walk, Run, Sprint:
/// the sweep `examples/fit_clip` pins each planted boot to (its `plant: boots sweep` line; examples/gait_strides
/// measures it back). The game sets the cadence from it, so the boot on the ground stays put.
pub const GAIT_STRIDE: [[f32; 8]; 3] = [
    [1.53, 1.56, 1.67, 1.79, 1.56, 1.67, 1.60, 1.61], // Walk
    [1.89, 1.65, 1.65, 1.83, 1.79, 1.53, 1.92, 2.00], // Run
    [2.63, 2.64, 3.17, 3.17, 3.14, 2.79, 2.84, 2.84], // Sprint
];

/// Which eighth-of-a-turn loop `dir` falls in and how far towards the next (0..1).
fn gait_sector(dir: f32) -> (usize, usize, f32) {
    let x = dir.rem_euclid(std::f32::consts::TAU) / (std::f32::consts::TAU / 8.0);
    let i = (x.floor() as usize) % 8;
    (i, (i + 1) % 8, x - x.floor())
}

/// The baked gait at `phase` moving in `dir`: the two nearest loops blended. Fields: hip L, R, knee L, R,
/// side L, R, pelvis turn, then lean, twist, tilt as motion around the pose's own, then the body lift (the
/// lowest the body may sit: it stands the reference's planted boot on the ground and lifts both clear in flight).
/// The baked gaits' boot roll at `phase` moving in `dir` (left, right), blended as `gait_bake`; 0 for other poses.
/// Fields: boot roll left, right, then each leg's heading turn (`PoseParams::roll`) left, right.
pub fn gait_toe(pose: Pose, phase: f32, dir: f32) -> [f32; 4] {
    let Some(toes) = gait_toes(pose) else { return [0.0; 4] };
    let (i, j, w) = gait_sector(dir);
    let (a, b) = (clip_at(toes[i], phase), clip_at(toes[j], phase));
    std::array::from_fn(|f| a[f] + (b[f] - a[f]) * w)
}

pub fn gait_bake(pose: Pose, phase: f32, dir: f32) -> Option<[f32; 11]> {
    let loops = gait_loops(pose)?;
    let (i, j, w) = gait_sector(dir);
    let (a, b) = (clip_at(loops[i], phase), clip_at(loops[j], phase));
    Some(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * w))
}

/// Puts a baked gait on the pose: legs and pelvis from the clip; the upper body keeps the pose's carry and lean
/// and takes the clip's sway; the head turns against the shoulders as before. Body height comes from the legs
/// (`ground_drop_side`, boots flat) or, in the flight of a run, from the clip's lift; the procedural bob is dropped.
/// How much of the baked sprint's torso twist is kept (common.wgsl `SPRINT_TWIST`): the Mixamo sprint swings a
/// one-handed rifle a third of a turn each way, which on the soldier reads as the body buzzing side to side.
pub const SPRINT_TWIST: f32 = 0.8;
/// How much of the baked run and sprint upper-body side lean is kept (common.wgsl `RUN_TILT`): in full it sways
/// the head 13 cm each stride, which reads as the body rocking from side to side at speed.
pub const RUN_TILT: f32 = 0.4;

fn apply_gait(p: &mut PoseParams, g: [f32; 11]) {
    p.hip = [g[0], g[1]];
    p.knee = [g[2], g[3]];
    p.side = [g[4], g[5]];
    p.yaw = g[6];
    p.lean += g[7];
    p.look += 0.8 * (p.twist - g[8]);
    p.twist = g[8];
    p.tilt += g[9];
    p.bob = 0.0;
    p.flat = 1.0;
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
            // The combat ready carried into the jog: the same hold (stock in the shoulder, muzzle ahead and down,
            // bladed), so starting off from the ready the rifle never changes hands.
            carry(&mut p, Carry::LowReadyFwd);
            p.twist = -p.rifle_yaw;
            p.look = p.rifle_yaw;
            // He leans further into the jog than at the ready: the muzzle comes up by the difference, so it still
            // points ahead and well down (0.15: any more pulls the handguard from the left hand).
            p.rifle += 0.15;
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
        Pose::Slung => {
            p.shoulder = [0.08, 0.08];
            p.elbow = [0.3, 0.3];
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
        Pose::Brake => {
            // Planted left foot forward and wide, right knee bent behind; leaning back against the stop.
            p.hip = [0.5, -0.3];
            p.knee = [0.25, 0.3];
            p.side = [-0.2, 0.04];
            p.flat = 1.0;
            carry(&mut p, Carry::LowReadyFwd);
            p.twist = -p.rifle_yaw;
            p.look = p.rifle_yaw;
            p.lean = -0.12;
            p.head = 0.12;
        }
        Pose::Crawl => {
            // Patch: Animations 2 high crawl: on elbows and knees, the body off the ground, the rifle cradled across
            // the forearms in the right hand (`HIGH_CRAWL`, solved by `crawl.rs`). Backwards plays the loop in
            // reverse; sideways keeps the stroke and leans the knees and chest toward the side.
            // common.wgsl mirrors this.
            let (fwd, lateral) = (dir.cos(), dir.sin());
            let wb = ((-0.2 - fwd) * 5.0).clamp(0.0, 1.0);
            let (a, b) = (clip_at(&HIGH_CRAWL, phase), clip_at(&HIGH_CRAWL, -phase));
            let r: [f32; 21] = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * wb);
            p.prone = 1.0;
            p.hip = [r[0], r[1]];
            p.knee = [r[2], r[3]];
            p.side = [r[4] + 0.3 * lateral, r[5] + 0.3 * lateral];
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
            p.look = -0.4 * lateral;
            p.rifle = 0.0;
            p.rifle_yaw = std::f32::consts::FRAC_PI_2;
        }
        Pose::Drop => return drop_params(phase),
        Pose::DropReach => return drop_reach(),
        Pose::DropPlant => return drop_plant(),
        Pose::Prone | Pose::ProneAim => {
            p.prone = 1.0;
            p.hip = [0.18, 0.16];
            p.knee = [0.06, 0.04];
            p.rifle = std::f32::consts::FRAC_PI_2;
            p.rifle_yaw = 0.55;
            p.twist = -0.55;
            p.look = 0.55;
            // Chest up on the elbows, hips and legs flat on the ground (AN-02-S09); rifle and head take back the
            // extra lean so the aim and the eyes stay where they were.
            p.lean = if pose == Pose::ProneAim { PRONE_LEAN } else { PRONE_AIM_LEAN };
            let arch = p.lean - PRONE_LEAN_OLD;
            p.rifle += arch;
            if pose == Pose::ProneAim {
                // Aiming: the rifle shouldered and pointed ahead along the ground.
                p.shoulder = [2.8, 1.571];
                p.elbow = [1.3, 2.084];
                p.reach = [0.0, 0.401];
                p.abduct = [0.0, 0.604];
                p.head = 1.0 + arch;
            } else {
                // At rest: elbows down under the chest, forearms ahead on the ground, the rifle lying across in front
                // (muzzle to his left; grip() keeps the left hand on the handguard), head down.
                p.rifle = 0.0;
                p.rifle_yaw = std::f32::consts::FRAC_PI_2;
                p.twist = 0.0;
                p.look = 0.0;
                p.shoulder = [1.75, 1.75];
                p.elbow = [1.45, 1.45];
                p.head = 0.8 + arch;
            }
        }
        Pose::StandAim => {
            // On the same athletic base as the ready (feet staggered, knees flexed, leaning in), rifle shouldered:
            // the rifle and head take back the lean so the sights stay level at the eye.
            athletic(&mut p);
            aim_arms(&mut p);
            p.lean = AIM_LEAN;
            p.rifle += AIM_LEAN;
            p.head = AIM_HEAD - AIM_LEAN;
        }
        Pose::Ready => {
            // Bladed like the aim (upper body turned by the rifle's turn across the chest), so the muzzle points
            // ahead and down; eyes up over it.
            athletic(&mut p);
            carry(&mut p, Carry::LowReadyFwd);
            p.twist = -p.rifle_yaw;
            p.look = p.rifle_yaw;
            p.lean = READY_LEAN;
            p.head = -READY_LEAN;
        }
    }
    let lift = gait_bake(pose, phase, dir).map_or(f32::MIN, |mut g| {
        if pose == Pose::Sprint {
            g[8] *= SPRINT_TWIST;
        }
        if matches!(pose, Pose::Run | Pose::Sprint) {
            g[9] *= RUN_TILT;
        }
        apply_gait(&mut p, g);
        let t = gait_toe(pose, phase, dir);
        p.toe = [t[0], t[1]];
        p.roll = [t[2], t[3]];
        g[10]
    });
    p.drop = ground_drop_toe(p.hip, p.knee, p.side, p.flat, p.toe).max(lift);
    clear_arms(&mut p);
    if pose == Pose::Crawl {
        // Lying down the grounding offset runs fore and aft: the crawl keeps the prone legs' value so the body does
        // not surge with the stroke, and nothing shifts between lying and crawling. common.wgsl mirrors this.
        p.drop = ground_drop_side([0.18, 0.16], [0.06, 0.04], [0.0, 0.0], 0.0);
    }
    p
}

/// Two layers (motion guide rule 1): the legs of one pose (gait, crouch) under the upper body of another (aim,
/// carry), so a soldier can walk while aiming.
pub fn layer(legs: &PoseParams, upper: &PoseParams) -> PoseParams {
    // A fully prone upper pose is already braced. The unaimed crawl's torso roll must not be added to it.
    let twist = if upper.prone == 1.0 { upper.twist } else { legs.twist + upper.twist };
    PoseParams { lean: upper.lean, shoulder: upper.shoulder, elbow: upper.elbow, reach: upper.reach, rifle: upper.rifle, rifle_yaw: upper.rifle_yaw, tilt: upper.tilt, twist, head: upper.head, sling: upper.sling, look: upper.look, abduct: upper.abduct, ..*legs }
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
    // The weapon swings more than the body (SWAY_GAIN): a carried rifle never sits still on the move.
    let sway = (2.0 * t + 0.5).sin();
    p.shoulder[0] += SWAY_GAIN * arms * sway;
    p.shoulder[1] += SWAY_GAIN * arms * sway;
    // The pitch and the swing across the body less: more pulls the handguard out of the left hand at a sprint.
    p.rifle += 1.25 * rifle * (2.0 * t).sin();
    p.rifle_yaw += swing * t.sin();
}

/// How much livelier the weapon is than the stride numbers alone (both moving and aiming).
pub const SWAY_GAIN: f32 = 1.6;

const IDLE_PRONE_BREATH: f32 = 0.006;
const IDLE_PRONE_HEAD_BREATH: f32 = 0.010;
const IDLE_PRONE_SCAN: f32 = 0.045;
const IDLE_PRONE_SCAN_PERIOD_S: f32 = 8.5;
const IDLE_PRONE_RIFLE_SWAY: f32 = 0.004;
const IDLE_PRONE_RIFLE_PERIOD_S: f32 = 3.2;
const IDLE_PRONE_RIFLE_YAW: f32 = 0.006;
const IDLE_PRONE_RIFLE_YAW_PERIOD_S: f32 = 4.1;
const IDLE_TACTICAL_BREATH: f32 = 0.012;
const IDLE_TACTICAL_LEAN: f32 = 0.003;
const IDLE_TACTICAL_TWIST: f32 = 0.028;
const IDLE_TACTICAL_TWIST_PERIOD_S: f32 = 10.0;
const IDLE_TACTICAL_SCAN: f32 = 0.16;
const IDLE_TACTICAL_SCAN_PERIOD_S: f32 = 12.0;
const IDLE_TACTICAL_RIFLE_SWAY: f32 = 0.006;
const IDLE_TACTICAL_RIFLE_PERIOD_S: f32 = 4.6;
const IDLE_TACTICAL_RIFLE_YAW: f32 = 0.009;
const IDLE_TACTICAL_RIFLE_YAW_PERIOD_S: f32 = 6.0;
const IDLE_PEEK_MIN: f32 = 0.12;
const IDLE_PEEK_PERIOD_S: f32 = 5.2;

/// Idle life for a soldier standing still (`t` seconds, `seed` 0..1 per soldier, so a squad is never in step):
/// breathing, a slow shift of weight, glances around, and now and then a hitch of the rifle. Moving soldiers get
/// their life from the stride instead; aiming soldiers keep their eyes and rifle on the target.
pub fn idle(p: &PoseParams, pose: Pose, t: f32, seed: f32) -> PoseParams {
    let mut q = *p;
    // Moving gaits and the close-up tablet view supply their own motion (or must stay perfectly still).
    if matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint | Pose::CrouchWalk | Pose::Tablet | Pose::ProneAim) {
        return q;
    }
    let tau = std::f32::consts::TAU;
    let breathe = (tau * (t / 3.8 + seed)).sin();

    // Prone: the chest works against the elbows, with only a small head scan and rifle settle.
    // Keep the legs and body-to-ground transform fixed so the soldier never floats or sinks.
    if pose == Pose::Prone {
        q.shoulder[0] += IDLE_PRONE_BREATH * breathe;
        q.shoulder[1] += IDLE_PRONE_BREATH * breathe;
        q.head += IDLE_PRONE_HEAD_BREATH * breathe;
        q.look += IDLE_PRONE_SCAN * (tau * (t / IDLE_PRONE_SCAN_PERIOD_S + seed * 1.4)).sin();
        q.rifle += IDLE_PRONE_RIFLE_SWAY * (tau * (t / IDLE_PRONE_RIFLE_PERIOD_S + seed)).sin();
        q.rifle_yaw += IDLE_PRONE_RIFLE_YAW * (tau * (t / IDLE_PRONE_RIFLE_YAW_PERIOD_S + seed * 1.3)).sin();
        return q;
    }

    // Behind cover and stacked: quiet breathing, a restrained weight shift and short sector scans.
    if matches!(pose, Pose::Cover | Pose::Stack) {
        q.shoulder[0] += IDLE_TACTICAL_BREATH * breathe;
        q.shoulder[1] += IDLE_TACTICAL_BREATH * breathe;
        q.lean += IDLE_TACTICAL_LEAN * breathe;
        q.twist += IDLE_TACTICAL_TWIST * (tau * (t / IDLE_TACTICAL_TWIST_PERIOD_S + seed * 1.7)).sin();
        q.look += IDLE_TACTICAL_SCAN * (tau * (t / IDLE_TACTICAL_SCAN_PERIOD_S + seed * 2.3)).sin();
        q.rifle += IDLE_TACTICAL_RIFLE_SWAY * (tau * (t / IDLE_TACTICAL_RIFLE_PERIOD_S + seed)).sin();
        q.rifle_yaw += IDLE_TACTICAL_RIFLE_YAW * (tau * (t / IDLE_TACTICAL_RIFLE_YAW_PERIOD_S + seed * 1.3)).sin();
        return q;
    }

    if matches!(pose, Pose::StandAim | Pose::CoverAim | Pose::LeanLeft | Pose::LeanRight | Pose::CrouchAim) {
        q.shoulder[0] += 0.008 * breathe;
        q.shoulder[1] += 0.008 * breathe;
        q.look += 0.03 * (tau * (t / 7.0 + seed)).sin();
        // Aiming: the muzzle wanders in a slow loop around the aim point (breath and pulse).
        q.rifle += 0.018 * (tau * (t / 2.3 + seed)).sin();
        q.rifle_yaw += 0.022 * (tau * (t / 3.1 + seed * 1.3)).sin();

        // A peek eases back towards cover, then leans out again. The cosine envelope has zero velocity
        // at both ends, avoiding a mechanical snap while preserving the chosen left/right side.
        if matches!(pose, Pose::LeanLeft | Pose::LeanRight) {
            let cycle = tau * (t / IDLE_PEEK_PERIOD_S + seed);
            let exposure = 0.5 - 0.5 * cycle.cos();
            q.tilt *= IDLE_PEEK_MIN + (1.0 - IDLE_PEEK_MIN) * exposure;
        }
        return q;
    }

    q.shoulder[0] += 0.03 * breathe;
    q.shoulder[1] += 0.03 * breathe;
    q.lean += 0.008 * breathe;
    q.twist += 0.06 * (tau * (t / 9.5 + seed * 1.7)).sin();
    if pose != Pose::Tablet {
        q.look += 0.6 * (tau * (t / 14.0 + seed * 2.3)).sin().powi(3);
    }
    if pose == Pose::Stand {
        q.rifle += 0.1 * (tau * (t / 12.0 + seed)).sin().max(0.0).powi(12);
    }
    q
}

/// Kneeling: left thigh level with the foot planted ahead, right thigh down with the knee on the ground and the
/// shin lying back; torso upright.
/// The athletic base of the ready and the aim: feet shoulder-width and staggered (left forward), knees flexed,
/// boots flat (common.wgsl `athletic`).
fn athletic(p: &mut PoseParams) {
    // Stagger 0.13 m either side of the hip, ankle 0.75 m below it (knees flexed from a straight 0.80).
    (p.hip, p.knee, p.side) = crouch_gait(0.0, 0.13, 0.75, 0.0, 0.62, 0.0);
    p.flat = 1.0;
}
/// Forward lean of the ready (radians): chest over the toes.
pub const READY_LEAN: f32 = 0.16;
/// Forward lean while aiming (radians): into the rifle.
pub const AIM_LEAN: f32 = 0.1;

/// Where the kneel sits on the drop (`Pose::Drop` phase): crouching joins and leaves the drop here.
pub const DROP_KNEEL: f32 = 0.35;
/// Where, still on his knee, he has leaned over and put the left hand down ahead.
pub const DROP_REACH: f32 = 0.55;
/// Where his weight is on the planted hand, most of the way over onto the front.
pub const DROP_PLANT: f32 = 0.78;

/// The drop to prone at phase `t` (`Pose::Drop`): ready, then down on a knee, then over onto the planted left
/// hand, then flat. common.wgsl `drop_params` mirrors it.
pub fn drop_params(t: f32) -> PoseParams {
    let (from, to, u) = drop_keys(t);
    blend(&params(from, 0.0), &params(to, 0.0), u)
}

/// The two keys of the drop around phase `t` and how far between them (0..1): the shader is handed these (the
/// pose and the previous pose, with the blend set to `u`) instead of evaluating the drop itself, which kept the
/// vertex shader small enough to compile quickly everywhere.
pub fn drop_keys(t: f32) -> (Pose, Pose, f32) {
    let t = t.clamp(0.0, 1.0);
    if t < DROP_KNEEL {
        (Pose::Ready, Pose::Crouch, t / DROP_KNEEL)
    } else if t < DROP_REACH {
        (Pose::Crouch, Pose::DropReach, (t - DROP_KNEEL) / (DROP_REACH - DROP_KNEEL))
    } else if t < DROP_PLANT {
        (Pose::DropReach, Pose::DropPlant, (t - DROP_REACH) / (DROP_PLANT - DROP_REACH))
    } else {
        (Pose::DropPlant, Pose::Prone, (t - DROP_PLANT) / (1.0 - DROP_PLANT))
    }
}

/// The left palm's plant on the drop (figure frame, feet at the origin, facing +y): Battlefield 3's way down is
/// contact first, the hand goes to a spot on the ground and the body comes down over it, then the arm slides
/// forward along the ground into the prone rest. Found by search.
pub const DROP_HAND: [f32; 3] = [-0.28, 0.8, 0.04];
/// On the drop phase: the hand reaches for the spot from here to here, holds it until the slide.
pub const DROP_HAND_REACH: [f32; 2] = [0.38, 0.5];
/// The slide forward into the prone rest, along the ground.
pub const DROP_HAND_SLIDE: [f32; 2] = [0.8, 1.0];

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let u = ((x - a) / (b - a)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The drop's left hand at phase `t` (after the grip, as common.wgsl `animate` applies it): planted on the ground
/// (`DROP_HAND`), then slid along it into wherever the pose's own hand is by the end (the prone rest).
pub fn drop_hand(p: &PoseParams, t: f32) -> PoseParams {
    let w = smooth(DROP_HAND_REACH[0], DROP_HAND_REACH[1], t);
    if w <= 0.0 {
        return *p;
    }
    let own = pose_upper(hand(p, Side::Left), p);
    let s = smooth(DROP_HAND_SLIDE[0], DROP_HAND_SLIDE[1], t);
    reach_left(p, unpose_upper(Vec3::from(DROP_HAND).lerp(own, s), p), w)
}

/// Most of the way over, the chest kept well down (so the head never rises as the body pivots from the knee to the
/// boots), the left palm on the ground ahead taking the weight (found by search: 4 cm up, 0.8 m ahead of the
/// boots); the rifle stays in the right hand.
fn drop_plant() -> PoseParams {
    let (kneel, flat) = (params(Pose::Crouch, 0.0), params(Pose::Prone, 0.0));
    let mut plant = blend(&kneel, &flat, 0.65);
    // The prone rifle-support rig adds a torso twist for aiming; the drop plant is a separate hand-braced
    // transition and must stay square to the planted palm.
    plant.lean = 1.1;
    plant.prone = 0.65;
    // The rifle in the right hand tucked to the chest and turned across (searched: clear of the ground all the
    // way into the prone rest, which has it across).
    plant.shoulder[1] = 0.6;
    plant.elbow[1] = 2.2;
    plant.rifle = 0.0;
    plant.rifle_yaw = 1.5;
    plant.twist = 0.0;
    // On both knees under him (search: knees about 20 cm up as drawn, boots on the ground), not floating.
    plant.hip = [0.9, 0.9];
    plant.knee = [1.35, 1.35];
    plant.drop = ground_drop_side(plant.hip, plant.knee, plant.side, plant.flat);
    plant.shoulder[0] = 0.9;
    plant.elbow[0] = 0.5;
    plant.reach[0] = 0.1;
    plant.abduct[0] = 0.15;
    plant.head = 0.6;
    plant
}

/// Between the kneel and the plant, still on the knee: chest down over it and the left palm on the ground just
/// ahead (search: about 15 cm up, 45 cm ahead), so he goes over onto the hand instead of rising off the knee.
fn drop_reach() -> PoseParams {
    let mut reach = params(Pose::Crouch, 0.0);
    reach.lean = 1.55;
    reach.rifle = 0.9;
    reach.shoulder[0] = 1.0;
    reach.elbow[0] = 0.1;
    reach.reach[0] = 0.1;
    reach.abduct[0] = 0.15;
    reach.head = 0.9;
    reach
}

/// The shader's literal for a fixed pose (`const` in common.wgsl), field for field in its `PP` order.
pub fn wgsl_literal(p: &PoseParams) -> String {
    let f = |x: f32| format!("{x:.6}");
    let v = |a: [f32; 2]| format!("vec2f({}, {})", f(a[0]), f(a[1]));
    format!(
        "PP({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
        f(p.lean), v(p.hip), v(p.knee), v(p.shoulder), v(p.elbow), f(p.head), f(p.rifle), f(p.prone), f(p.drop),
        v(p.reach), f(p.sling), f(p.bob), f(p.twist), f(p.look), v(p.abduct), f(p.rifle_yaw), v(p.side), f(p.flat), f(p.tilt), f(p.yaw), f(p.raise), v(p.roll), v(p.toe)
    )
}

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
    /// Combat low ready: the shooting hold pivoted down about the shoulder pocket, muzzle forward and down.
    LowReadyFwd,
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
        Carry::LowReadyFwd => ([0.65, -0.13], [0.45, 1.95], [0.36, 0.70], [0.14, 0.34], -0.60, 0.70),
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
    ground_drop_toe(hip, knee, side, flat, [0.0; 2])
}

/// `ground_drop_side` with the boots rolled by `toe` (`PoseParams::toe`). common.wgsl `ground_drop` mirrors it.
pub fn ground_drop_toe(hip: [f32; 2], knee: [f32; 2], side: [f32; 2], flat: f32, toe: [f32; 2]) -> f32 {
    let mut low = f32::MAX;
    for i in 0..2 {
        let px = if i == 0 { -HIP_X } else { HIP_X };
        for y in [HEEL_Y, TOE_Y] {
            low = low.min(sideways(foot(Vec3::new(px, y, 0.0), hip[i], knee[i], flat, 0.0, toe[i]), side[i], px).z);
        }
    }
    -low
}

/// Seams: the soldier is one mesh cut into rigid parts, so each cut is a ring of vertices that belong to triangles
/// of two parts. Posed rigidly with either part, a ring vertex lands in two places and the cut opens (gaps at the
/// ankle, seat and crotch). `bake_seams` gives every vertex near a cut the part across it and a blend weight that
/// depends only on where the vertex is: 0.5 on the cut, easing to none `SEAM_REACH` away, so all copies of a cut
/// vertex land in one place. The thighs' shared seam at the crotch goes wholly with the pelvis. The blend rides in
/// the colour code's fraction: `SEAM_SCALE` * fraction = other part * 256 + weight * 254 (254, so a cut's 0.5 is exact).
pub const SEAM_SCALE: f32 = 4096.0;
/// How far from a cut (m) its blend reaches: ankle, knee, hip, crotch, elbow, shoulder, neck. The ankle's stops short
/// of the sole, which stays flat on the ground.
pub const SEAM_REACH: [f32; 7] = [0.045, 0.07, 0.08, 0.06, 0.06, 0.07, 0.05];

/// The joint pivot (rest pose) between two parts, and which `SEAM_REACH` applies; None if they share no joint.
pub fn seam_joint(a: Part, b: Part) -> Option<(Vec3, usize)> {
    use Part::*;
    let x = |p: Part| if matches!(p, LeftThigh | LeftShin | LeftFoot) { -HIP_X } else if matches!(p, RightThigh | RightShin | RightFoot) { HIP_X } else if matches!(p, LeftArm | LeftForearm) { -SHOULDER_X } else { SHOULDER_X };
    let (a, b) = if (a as u32) < (b as u32) { (a, b) } else { (b, a) };
    Some(match (a, b) {
        (LeftShin, LeftFoot) | (RightShin, RightFoot) => (Vec3::new(x(a), 0.0, ANKLE_Z), 0),
        (LeftThigh, LeftShin) | (RightThigh, RightShin) => (Vec3::new(x(a), 0.0, KNEE_Z), 1),
        (LeftThigh, Torso) | (RightThigh, Torso) => (Vec3::new(x(a), 0.0, HIP_Z), 2),
        (LeftArm, LeftForearm) | (RightArm, RightForearm) => (Vec3::new(x(a), 0.0, ELBOW_Z), 4),
        (LeftArm, Torso) | (RightArm, Torso) => (Vec3::new(x(a), 0.0, SHOULDER_Z), 5),
        (Torso, Head) => (Vec3::new(0.0, 0.0, NECK_Z), 6),
        _ => return None,
    })
}

/// The part across the nearest cut and its blend weight, as `bake_seams` packed them into a colour code.
pub fn seam_of(code: f32) -> Option<(Part, f32)> {
    if code < 0.0 {
        return None;
    }
    let packed = ((code - code.floor()) * SEAM_SCALE).round() as u32;
    let w = (packed & 255) as f32 / 254.0;
    (w > 0.0).then(|| (crate::mesh::part_of((packed >> 8) as f32 * crate::mesh::PALETTE_SIZE as f32), w))
}

/// Works out the seam blends of a cut-up figure and packs them into its colour codes (see `SEAM_SCALE`).
pub fn bake_seams(vertices: &mut [crate::mesh::Vertex]) {
    use crate::mesh::part_of;
    use std::collections::HashMap;
    let key = |p: [f32; 3]| p.map(|c| (c * 1e4).round() as i32);
    let mut parts: HashMap<[i32; 3], (Vec3, Vec<Part>)> = HashMap::new();
    for v in vertices.iter() {
        if v.color < 0.0 {
            continue;
        }
        let e = parts.entry(key(v.pos)).or_insert((Vec3::from(v.pos), Vec::new()));
        let part = part_of(v.color);
        if !e.1.contains(&part) {
            e.1.push(part);
        }
    }
    // Cut vertices per pair of parts; a vertex both thighs share is the crotch, which the pelvis owns.
    let crotch: Vec<Vec3> = parts.values().filter(|(_, ps)| ps.contains(&Part::LeftThigh) && ps.contains(&Part::RightThigh)).map(|(p, _)| *p).collect();
    let mut cuts: HashMap<(u32, u32), (Part, Part, Vec<Vec3>)> = HashMap::new();
    for (p, ps) in parts.values() {
        if ps.contains(&Part::LeftThigh) && ps.contains(&Part::RightThigh) {
            continue;
        }
        for &a in ps {
            for &b in ps {
                if a != b && seam_joint(a, b).is_some() {
                    cuts.entry((a as u32, b as u32)).or_insert((a, b, Vec::new())).2.push(*p);
                }
            }
        }
    }
    let near = |set: &[Vec3], at: Vec3| set.iter().map(|c| c.distance(at)).fold(f32::MAX, f32::min);
    for v in vertices.iter_mut() {
        if v.color < 0.0 {
            continue;
        }
        let (at, part) = (Vec3::from(v.pos), part_of(v.color));
        // (score: distance over reach, the part across, the weight on the cut itself)
        let mut best: Option<(f32, Part, f32)> = None;
        let mut take = |score: f32, other: Part, on_cut: f32| {
            if score < 1.0 && best.map_or(true, |b| score < b.0) {
                best = Some((score, other, on_cut));
            }
        };
        for (a, b, set) in cuts.values() {
            if *a == part {
                take(near(set, at) / SEAM_REACH[seam_joint(*a, *b).unwrap().1], *b, 0.5);
            }
        }
        if !crotch.is_empty() {
            let d = near(&crotch, at) / SEAM_REACH[3];
            match part {
                Part::LeftThigh | Part::RightThigh => take(d, Part::Torso, 1.0),
                Part::Torso => take(d, Part::Torso, 0.0),
                _ => {}
            }
        }
        if let Some((score, other, on_cut)) = best {
            let w = on_cut * (1.0 - smooth(0.0, 1.0, score));
            let q = (w * 254.0).round() as u32;
            if q > 0 && other != part {
                v.color = v.color.floor() + ((other as u32) * 256 + q) as f32 / SEAM_SCALE;
            }
        }
    }
}

/// A figure vertex in the pose: its own part's motion, blended across a nearby cut as `bake_seams` packed into its
/// colour code. The blend is a turn about the joint part way from one part's turn to the other's (each part's turn
/// read off three posed points: the pivot and a step along x and y), so the skin round a bent knee, hip or shoulder
/// keeps its shape: averaging the two positions instead pinches a deeply bent joint (the rubbery look).
/// common.wgsl `pose_mesh_vertex` mirrors it.
pub fn pose_mesh_vertex(v: Vec3, code: f32, p: &PoseParams) -> Vec3 {
    let part = crate::mesh::part_of(code);
    let Some((other, w)) = seam_of(code) else { return pose_rigid(v, part, p) };
    let Some((j, _)) = seam_joint(part, other) else { return pose_rigid(v, part, p) };
    let frame = |q: Part| {
        let o = pose_rigid(j, q, p);
        let x = (pose_rigid(j + Vec3::X * 0.1, q, p) - o).normalize();
        let y = (pose_rigid(j + Vec3::Y * 0.1, q, p) - o).reject_from_normalized(x).normalize();
        (o, glam::Quat::from_mat3(&glam::Mat3::from_cols(x, y, x.cross(y))))
    };
    let ((oa, qa), (ob, qb)) = (frame(part), frame(other));
    let qb = if qa.dot(qb) < 0.0 { -qb } else { qb };
    oa.lerp(ob, w) + qa.lerp(qb, w).normalize() * (v - j)
}

/// Moves a point carried by one figure part into the pose (in the figure's frame, before heading and position).
/// Mesh vertices go through `pose_mesh_vertex`, which also closes the cuts between parts.
pub fn pose_vertex(v: Vec3, part: Part, p: &PoseParams) -> Vec3 {
    pose_rigid(v, part, p)
}

/// `pose_vertex` with the vertex carried by its own part alone.
pub fn pose_rigid(v: Vec3, part: Part, p: &PoseParams) -> Vec3 {
    let mut q = match part {
        Part::Body | Part::Sway | Part::Glow => return v,
        Part::LeftThigh => sideways(leg_roll(rot(v, 0.0, HIP_Z, p.hip[0]), p, 0), p.side[0], -HIP_X),
        Part::RightThigh => sideways(leg_roll(rot(v, 0.0, HIP_Z, p.hip[1]), p, 1), p.side[1], HIP_X),
        Part::LeftShin => sideways(leg_roll(leg(v, p.hip[0], p.knee[0]), p, 0), p.side[0], -HIP_X),
        Part::RightShin => sideways(leg_roll(leg(v, p.hip[1], p.knee[1]), p, 1), p.side[1], HIP_X),
        Part::LeftFoot => sideways(leg_roll(foot(v, p.hip[0], p.knee[0], p.flat, p.prone, p.toe[0]), p, 0), p.side[0], -HIP_X),
        Part::RightFoot => sideways(leg_roll(foot(v, p.hip[1], p.knee[1], p.flat, p.prone, p.toe[1]), p, 1), p.side[1], HIP_X),
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
    } else {
        q = turn(q, p.yaw);
    }
    q.z += p.drop + p.bob;
    if p.prone != 0.0 {
        // Positive tips him forward onto his front, negative back onto his back.
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2 * p.prone);
        // Mid-transition the bent knees would dip into the ground: lift a little, most at the halfway point.
        // Onto his front the body keeps the transition height (PRONE_LIFT_MID) until the very end, then settles
        // flat (PRONE_LIFT): hips and legs on the ground only once he is fully down.
        q.z += prone_lift(p.prone) + p.raise;
    }
    q
}

/// How far the body is lifted at prone amount `prone` (`pose_vertex`).
fn prone_lift(prone: f32) -> f32 {
    let a = prone.abs();
    let lift = if prone > 0.0 { PRONE_LIFT_MID * a - (PRONE_LIFT_MID - PRONE_LIFT) * prone_settle(a) } else { SUPINE_LIFT * a };
    lift + 0.12 * (std::f32::consts::PI * a).sin()
}

/// `pose_vertex`'s upper-body chain: a point posed in the arms' frame to the figure's frame.
pub fn pose_upper(v: Vec3, p: &PoseParams) -> Vec3 {
    let mut q = tilt(rot(turn(v, p.twist), 0.0, HIP_Z, -p.lean), p.tilt);
    q.z += p.drop + p.bob;
    if p.prone != 0.0 {
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2 * p.prone);
        q.z += prone_lift(p.prone) + p.raise;
    }
    q
}

/// The inverse of `pose_vertex`'s upper-body chain: where a point in the figure's frame (feet at the origin) is in
/// the frame the arms are posed in, before twist, lean, tilt, drop and prone. common.wgsl `unpose_upper` mirrors it.
pub fn unpose_upper(w: Vec3, p: &PoseParams) -> Vec3 {
    let mut q = w;
    if p.prone != 0.0 {
        q.z -= prone_lift(p.prone) + p.raise;
        q = rot(q, 0.0, 0.0, std::f32::consts::FRAC_PI_2 * p.prone);
    }
    q.z -= p.drop + p.bob;
    q = tilt(q, -p.tilt);
    turn(rot(q, 0.0, HIP_Z, p.lean), -p.twist)
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
    let toe = m2(a.toe, b.toe);
    let drop = if w <= 0.0 { a.drop } else if w >= 1.0 { b.drop } else { ground_drop_toe(hip, knee, side, flat, toe) };
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
        yaw: m(a.yaw, b.yaw),
        raise: m(a.raise, b.raise),
        roll: m2(a.roll, b.roll),
        toe,
    }
}

/// Seconds a pose change takes.
pub const BLEND_S: f32 = 0.35;
/// Seconds kneeling down or standing up from the knee takes: 30% quicker than other changes (Patch: Animations).
pub const KNEEL_BLEND_S: f32 = BLEND_S / 1.3;

/// Seconds the change from `a` to `b` takes: `KNEEL_BLEND_S` between standing and kneeling, else `BLEND_S`.
/// common.wgsl `blend_s` mirrors it.
pub fn blend_s(a: Pose, b: Pose) -> f32 {
    let standing = |p: Pose| matches!(p, Pose::Stand | Pose::Ready | Pose::StandAim);
    let kneeling = |p: Pose| matches!(p, Pose::Crouch | Pose::CrouchAim);
    if (standing(a) && kneeling(b)) || (kneeling(a) && standing(b)) { KNEEL_BLEND_S } else { BLEND_S }
}

/// When a pose change should count as having started (shader clock, s), so the figure does not jump. Going back
/// to `prev`, the pose it was blending away from since `since`, part way through starts the return as far along
/// as the first blend had got: the eased blend is symmetric, so the body carries on from exactly where it was.
/// Anything else starts `now`.
/// `secs` is how long the blend between the two takes (`blend_s`).
pub fn switch_start<P: PartialEq>(prev: P, since: f32, new: P, now: f32, secs: f32) -> f32 {
    let w = ((now - since) / secs).clamp(0.0, 1.0);
    if new == prev && w < 1.0 { now - (1.0 - w) * secs } else { now }
}

#[cfg(test)]
mod switch_tests {
    use super::{switch_start, BLEND_S};
    fn eased(w: f32) -> f32 { let w = w.clamp(0.0, 1.0); w * w * (3.0 - 2.0 * w) }
    #[test]
    fn turning_back_mid_blend_carries_on_from_where_the_body_was() {
        // Stand -> walk at 0, back to stand 0.1 s later: the stand weight just before equals the one just after.
        let before = 1.0 - eased(0.1 / BLEND_S);
        let start = switch_start(0u8, 0.0, 0u8, 0.1, BLEND_S);
        let after = eased((0.1 - start) / BLEND_S);
        assert!((before - after).abs() < 1e-5, "{before} vs {after}");
    }
    #[test]
    fn a_settled_or_new_pose_starts_now() {
        assert_eq!(switch_start(0u8, 0.0, 0u8, 1.0, BLEND_S), 1.0);
        assert_eq!(switch_start(0u8, 0.0, 2u8, 0.1, BLEND_S), 0.1);
    }
}

/// How far a figure banks into a turn (radians, + to the left): the faster he goes and the harder he turns, the
/// more, like a runner leaning into a bend. About 2 degrees walking, 6 jogging, 14 sprinting on a firm turn;
/// never past BANK_MAX. `speed` m/s, `turn` rad/s (+ turning left).
pub fn bank(speed: f32, turn: f32) -> f32 {
    (speed * speed * turn * 0.006).clamp(-BANK_MAX, BANK_MAX)
}
/// The steepest bank (15 degrees).
pub const BANK_MAX: f32 = 0.26;
/// How far a figure pitches with his acceleration along his facing (radians, + forward): he leans into a push
/// off from standing, and back against a hard stop or skid. `accel` m/s² (+ speeding up).
pub fn accel_pitch(accel: f32) -> f32 {
    (accel * 0.03).clamp(-PITCH_MAX, PITCH_MAX)
}
/// The furthest lean forward or back (about 11 degrees).
pub const PITCH_MAX: f32 = 0.2;
/// Bank and pitch packed into the fractional part of an instance's colour (common.wgsl `tilt_of`): each to 128
/// steps (64 upright). Upright on both leaves the colour whole (no fraction at all).
pub fn pack_tilt(roll: f32, pitch: f32) -> f32 {
    let q = |a: f32, max: f32| ((a / max).clamp(-1.0, 1.0) * 63.0).round() as i32 + 64;
    let (r, p) = (q(roll, BANK_MAX), q(pitch, PITCH_MAX));
    if r == 64 && p == 64 { 0.0 } else { (1 + r * 128 + p) as f32 / 16_385.0 }
}
/// The inverse of `pack_tilt` from a colour value (as the shader reads it): (roll, pitch).
pub fn unpack_tilt(colour: f32) -> (f32, f32) {
    let f = colour - colour.floor();
    if f < 1e-6 {
        return (0.0, 0.0);
    }
    let n = (f * 16_385.0).round() as i32 - 1;
    let (r, p) = (n / 128, n % 128);
    ((r - 64) as f32 / 63.0 * BANK_MAX, (p - 64) as f32 / 63.0 * PITCH_MAX)
}

#[cfg(test)]
mod bank_tests {
    use super::*;
    #[test]
    fn banking_grows_with_pace_and_turn_and_survives_packing() {
        let deg = |a: f32| a.to_degrees();
        assert!(deg(bank(1.7, 2.0)) < 3.0, "walking: barely");
        assert!((4.0..9.0).contains(&deg(bank(3.0, 2.0))), "jogging: a little ({})", deg(bank(3.0, 2.0)));
        assert!(deg(bank(4.6, 2.0)) >= 12.0, "sprinting: clearly");
        assert!(bank(4.6, -2.0) < 0.0 && bank(3.0, 0.0) == 0.0);
        for (r, p) in [(-0.26f32, 0.0f32), (-0.1, 0.12), (0.0, -0.2), (0.07, 0.05), (0.26, 0.2)] {
            for base in [6.0f32, 9.0, 38.0, 233.0] {
                let c = base + pack_tilt(r, p);
                let (r2, p2) = unpack_tilt(c);
                assert!((r2 - r).abs() < 0.005 && (p2 - p).abs() < 0.005, "{base}: ({r}, {p}) -> ({r2}, {p2})");
                assert_eq!(c.floor(), base, "the colour's whole part is untouched");
            }
        }
        assert_eq!(pack_tilt(0.0, 0.0), 0.0, "upright: no fraction");
        assert_eq!(unpack_tilt(14.0), (0.0, 0.0));
        assert!(accel_pitch(4.2) > 0.1 && accel_pitch(-6.0) < -0.15);
    }
}

/// A weapon as the figures carry it. Every gun is drawn in the rifle's rest frame (grip at `RIFLE_GRIP`); its
/// rounds leave from `muzzle`, wherever the pose puts the gun, so flashes and tracers start at the barrel's end
/// for any weapon given its own muzzle.
#[derive(Copy, Clone, Debug)]
pub struct Weapon {
    /// The end of the barrel, in the rest frame (mesh.rs builds the rifle's barrel out to y 0.77).
    pub muzzle: Vec3,
    /// The back of the barrel (rest frame): with `muzzle`, which way the bore points.
    pub breech: Vec3,
}
/// The issue rifle (mesh.rs `soldier`): barrel from y 0.58 to 0.77 at x 0.238, z 0.818..0.836.
pub const RIFLE: Weapon = Weapon { muzzle: Vec3::new(0.238, 0.77, 0.827), breech: Vec3::new(0.238, 0.58, 0.827) };

/// Where `w`'s muzzle is in a figure's own frame (x right, y forward, z up, metres from his feet) for a pose
/// with an optional upper-body layer, posed as common.wgsl poses the gun (layer, then the two-handed grip).
pub fn muzzle(w: Weapon, pose: Pose, upper: Option<Pose>, phase: f32, dir: f32) -> Vec3 {
    barrel(w, pose, upper, phase, dir).0
}

/// `w`'s muzzle and the unit direction its bore points, in a figure's own frame (as `muzzle`).
pub fn barrel(w: Weapon, pose: Pose, upper: Option<Pose>, phase: f32, dir: f32) -> (Vec3, Vec3) {
    barrel_with_phases(w, pose, upper, phase, phase, dir)
}

/// A barrel reference with a separate magazine-manipulation phase; gait and weapon handling need not share a clock.
pub fn barrel_with_phases(w: Weapon, pose: Pose, upper: Option<Pose>, phase: f32, reload_phase: f32, dir: f32) -> (Vec3, Vec3) {
    let base_phase = if pose == Pose::Reload { reload_phase } else { phase };
    let legs = params_dir(pose, base_phase, dir);
    let p = match upper {
        Some(u) => layer(&legs, &params_dir(u, if u == Pose::Reload { reload_phase } else { phase }, 0.0)),
        None => legs,
    };
    let p = grip(&p, two_handed(upper.unwrap_or(pose)));
    let p = if pose == Pose::Reload || upper == Some(Pose::Reload) { reload_hand(&p, reload_phase, 1.0) } else { p };
    let tip = pose_vertex(w.muzzle, Part::Rifle, &p);
    (tip, (tip - pose_vertex(w.breech, Part::Rifle, &p)).normalize_or_zero())
}

/// Recoil under fire (`k` the firing pressure, 0 to 1: it builds through a burst and eases off after). The rifle
/// is driven straight back into the shoulder with the arms and the upper body takes it, so the muzzle stays on the
/// mark; it only creeps up a little (squared, so the climb comes late in a long burst). Visual only: where rounds
/// go is the battle's. common.wgsl mirrors this.
pub fn recoil(p: &mut PoseParams, k: f32) {
    if k <= 0.0 {
        return;
    }
    p.rifle += 0.0288 * k * k;
    p.shoulder[0] -= 0.108 * k;
    p.shoulder[1] -= 0.108 * k;
    p.lean -= 0.024 * k;
}

/// A flinch from a round close by or a hit that does not put him down (`k` 0 to 1, easing off): he ducks his
/// head, hunches over, dips at the knees and tucks the rifle in. Lying down, only the head ducks. Visual only.
/// common.wgsl mirrors this.
pub fn flinch(p: &mut PoseParams, k: f32) {
    if k <= 0.0 {
        return;
    }
    p.head -= 0.35 * k;
    p.shoulder[0] -= 0.06 * k;
    p.shoulder[1] -= 0.06 * k;
    if p.prone != 0.0 {
        return;
    }
    p.lean += 0.18 * k;
    p.hip[0] += 0.2 * k;
    p.hip[1] += 0.2 * k;
    p.knee[0] += 0.35 * k;
    p.knee[1] += 0.35 * k;
    p.drop = ground_drop_side(p.hip, p.knee, p.side, p.flat);
}

/// The previous pose's id (instance d.x) carrying the flinch in its fraction (common.wgsl reads the id with
/// `u32()`).
pub fn pack_flinch(prev: Pose, k: f32) -> f32 {
    prev as u32 as f32 + k.clamp(0.0, 0.99)
}

/// An instance's pose id (b.w) carrying the shot kick in its fraction (common.wgsl reads the id with `u32()`).
pub fn pack_kick(pose: Pose, k: f32) -> f32 {
    pose as u32 as f32 + k.clamp(0.0, 0.99)
}
pub fn unpack_kick(w: f32) -> f32 {
    w.fract()
}

/// A point of a figure's own frame placed in the world (metres): banked into a turn (`bank`, + to his left)
/// and turned to his heading as common.wgsl `animate` does, then moved to where he stands (`at`, z = ground).
pub fn figure_to_world(q: Vec3, at: Vec3, heading: f32, bank: f32, pitch: f32) -> Vec3 {
    let (sb, cb) = bank.sin_cos();
    let q = Vec3::new(q.x * cb - q.z * sb, q.y, q.x * sb + q.z * cb);
    let (sp, cp) = pitch.sin_cos();
    let q = Vec3::new(q.x, q.y * cp + q.z * sp, -q.y * sp + q.z * cp);
    let (sh, ch) = heading.sin_cos();
    Vec3::new(q.x * ch - q.y * sh, q.x * sh + q.y * ch, q.z) + at
}

#[cfg(test)]
mod muzzle_tests {
    use super::*;
    #[test]
    fn the_muzzle_is_at_the_barrels_end_wherever_the_pose_puts_the_gun() {
        let aim = muzzle(RIFLE, Pose::StandAim, None, 0.0, 0.0);
        assert!(aim.y > 0.6 && (1.1..1.8).contains(&aim.z), "aiming: well ahead at about shoulder height, {aim:?}");
        let crouch = muzzle(RIFLE, Pose::CrouchAim, None, 0.0, 0.0);
        assert!(crouch.z < aim.z - 0.2, "crouched: lower, {crouch:?}");
        let prone = muzzle(RIFLE, Pose::Prone, None, 0.0, 0.0);
        assert!(prone.z < 0.5, "prone: near the ground, {prone:?}");
        let walking_aim = muzzle(RIFLE, Pose::Walk, Some(Pose::StandAim), 0.3, 0.0);
        assert!((walking_aim - aim).length() < 0.3, "aiming while walking: close to the standing aim, {walking_aim:?}");
        // Turned to face +x (heading -90 degrees): the muzzle is out along +x from where he stands.
        let w = figure_to_world(aim, Vec3::new(10.0, 20.0, 1.0), -std::f32::consts::FRAC_PI_2, 0.0, 0.0);
        assert!(w.x > 10.5 && (w.y - 20.0).abs() < 0.4 && w.z > 2.0, "{w:?}");
    }

    /// The barrel's direction (breech to muzzle) in a figure's frame.
    fn bore(pose: Pose, upper: Option<Pose>) -> Vec3 {
        barrel(RIFLE, pose, upper, 0.0, 0.0).1
    }

    #[test]
    fn an_aimed_barrel_points_where_he_faces() {
        for (pose, upper) in [(Pose::StandAim, None), (Pose::CrouchAim, None), (Pose::Walk, Some(Pose::StandAim))] {
            let b = bore(pose, upper);
            let yaw = b.x.atan2(b.y).to_degrees();
            assert!(yaw.abs() < 3.0, "{pose:?}/{upper:?}: barrel yaw {yaw:.1} degrees off his heading");
        }
    }


    #[test]
    fn moving_reload_reaches_the_magazine_and_uses_its_own_phase() {
        let legs = params(Pose::Walk, 0.2);
        let upper = params(Pose::Reload, 0.2);
        let layered = layer(&legs, &upper);
        let target = reload_target(&layered, 0.2);
        let reached = reload_hand(&layered, 0.2, 1.0);
        assert!(
            (hand(&reached, Side::Left) - target).length() < (hand(&layered, Side::Left) - target).length(),
            "the support hand leaves the handguard for the magazine path"
        );

        let early = barrel_with_phases(RIFLE, Pose::Walk, Some(Pose::Reload), 0.1, 0.15, 0.0).0;
        let late = barrel_with_phases(RIFLE, Pose::Walk, Some(Pose::Reload), 0.1, 0.5, 0.0).0;
        assert!((early - late).length() > 0.005, "reload manipulation changes the posed barrel independently of gait");
    }
}

/// How long a change of upper body (raising or lowering the rifle over moving legs) takes to blend (s).
pub const LAYER_BLEND_S: f32 = 0.3;

/// An instance's c.w for a figure (common.wgsl `fade_of`, `layer_back`): its dissolve (0 solid .. 1 gone) with,
/// while it changes upper body, the one it is leaving (`was`, None = the legs' own) and how much of it remains
/// (1 just changed .. 0 done). Settled, this is just the dissolve.
pub fn pack_fade_layer(fade: f32, was: Option<Pose>, remaining: f32) -> f32 {
    let fade = fade.clamp(0.0, 0.999);
    let r = (remaining.clamp(0.0, 1.0) * 63.0).round();
    if r <= 0.0 {
        return fade;
    }
    let code = was.map_or(0.0, |p| p as u32 as f32 + 1.0);
    code * 64.0 + r + fade
}

/// The inverse of `pack_fade_layer`: (dissolve, upper body being left as id + 1 or 0, remaining).
pub fn unpack_fade_layer(w: f32) -> (f32, f32, f32) {
    if w < 1.0 {
        return (w, 0.0, 0.0);
    }
    let whole = w.floor();
    let code = (whole / 64.0).floor();
    (w - whole, code, (whole - code * 64.0) / 63.0)
}

#[cfg(test)]
mod fade_layer_tests {
    use super::*;
    #[test]
    fn the_upper_body_being_left_rides_with_the_dissolve() {
        assert_eq!(pack_fade_layer(0.0, Some(Pose::StandAim), 0.0), 0.0, "settled: nothing extra");
        assert_eq!(pack_fade_layer(0.4, None, 0.0), 0.4, "a plain dissolve is unchanged");
        let w = pack_fade_layer(0.25, Some(Pose::StandAim), 0.5);
        let (f, code, rem) = unpack_fade_layer(w);
        assert!((f - 0.25).abs() < 1e-3 && code == Pose::StandAim as u32 as f32 + 1.0 && (rem - 0.5).abs() < 0.02, "{f} {code} {rem}");
        let (_, code, rem) = unpack_fade_layer(pack_fade_layer(0.0, None, 1.0));
        assert!(code == 0.0 && rem > 0.99, "leaving the legs' own upper body");
        let (_, code, _) = unpack_fade_layer(pack_fade_layer(0.0, Some(Pose::Ready), 0.9));
        assert_eq!(code, Pose::Ready as u32 as f32 + 1.0);
    }

    #[test]
    fn recoil_pushes_the_rifle_in_and_only_creeps_it_up() {
        let base = params(Pose::StandAim, 0.0);
        let at = |p: &PoseParams| { let p = grip(p, 1.0); pose_vertex(RIFLE.muzzle, Part::Rifle, &p) };
        let a = at(&base);
        let mut first = base;
        recoil(&mut first, 0.5);
        let mut held = base;
        recoil(&mut held, 1.0);
        let (b, c) = (at(&first), at(&held));
        assert!(c.y < a.y - 0.01, "rifle driven back into the shoulder: {a:?} -> {c:?}");
        assert!(a.y - c.y > (c.z - a.z) * 1.5, "mostly in, not up: {a:?} -> {c:?}");
        assert!(c.z > a.z && c.z - a.z < 0.05, "a slight climb, no flip: {a:?} -> {c:?}");
        assert!(b.z - a.z < (c.z - a.z) * 0.5, "the climb builds through a burst: {:?} vs {:?}", b - a, c - a);
        let mut none = base;
        recoil(&mut none, 0.0);
        assert_eq!(at(&none), a, "no kick, no change");
    }

    #[test]
    fn the_kick_rides_on_the_pose_id_without_changing_it() {
        let packed = pack_kick(Pose::StandAim, 0.7);
        assert_eq!(packed as u32, Pose::StandAim as u32);
        assert!((unpack_kick(packed) - 0.7).abs() < 0.01);
        assert_eq!(pack_kick(Pose::Ready, 0.0), Pose::Ready as u32 as f32);
        assert!(unpack_kick(pack_kick(Pose::Ready, 5.0)) < 1.0);
    }
}

#[cfg(test)]
mod drop_tests {
    use super::*;
    use crate::mesh::Part;

    #[test]
    fn the_drop_lands_exactly_on_ready_kneel_and_prone() {
        assert_eq!(drop_params(0.0), params(Pose::Ready, 0.0));
        assert_eq!(drop_params(DROP_KNEEL), params(Pose::Crouch, 0.0));
        assert_eq!(drop_params(1.0), params(Pose::Prone, 0.0));
        assert_eq!(params(Pose::Drop, 0.6), drop_params(0.6));
    }

    #[test]
    fn the_left_hand_goes_to_the_ground_ahead_and_nothing_jumps() {
        let palm = |t: f32| pose_vertex(Vec3::new(-PALM.x, PALM.y, PALM.z), Part::LeftForearm, &drop_params(t));
        let h = palm(DROP_PLANT);
        assert!((-0.02..0.15).contains(&h.z), "planted hand on the ground: {h:?}");
        assert!(h.y > 0.15, "and ahead of him: {h:?}");
        // Small steps along the drop move the head a little each, never a jump.
        let head = |t: f32| pose_vertex(Vec3::new(0.0, 0.0, 1.6), Part::Head, &drop_params(t));
        let kneel_head = head(DROP_KNEEL).z;
        for i in 35..=100 {
            let z = head(i as f32 / 100.0).z;
            assert!(z < kneel_head + 0.06, "head rises off the knee at {}: {z} vs {kneel_head}", i as f32 / 100.0);
        }
        for i in 0..100 {
            let (a, b) = (i as f32 / 100.0, (i + 1) as f32 / 100.0);
            let step = (head(b) - head(a)).length();
            assert!(step < 0.08, "head jumps {step} m between {a} and {b}");
        }
    }
}

#[cfg(test)]
mod flinch_tests {
    use super::*;
    use crate::mesh::Part;

    #[test]
    fn a_flinch_ducks_the_head_and_keeps_the_boots_on_the_ground() {
        for pose in [Pose::Ready, Pose::StandAim, Pose::Crouch] {
            let base = params(pose, 0.0);
            let mut f = base;
            flinch(&mut f, 1.0);
            let head = |p: &PoseParams| pose_vertex(Vec3::new(0.0, 0.0, 1.6), Part::Head, p);
            assert!(head(&f).z < head(&base).z - 0.05, "{pose:?} ducks: {:?} -> {:?}", head(&base), head(&f));
            let boot = |p: &PoseParams| pose_vertex(Vec3::new(-0.1, 0.05, 0.03), Part::LeftFoot, p).z;
            assert!((boot(&f) - boot(&base)).abs() < 0.05, "{pose:?} boots stay down: {} -> {}", boot(&base), boot(&f));
        }
        let mut lying = params(Pose::Prone, 0.0);
        let before = lying;
        flinch(&mut lying, 1.0);
        assert_eq!((lying.hip, lying.knee, lying.prone), (before.hip, before.knee, before.prone), "lying down only the head ducks");
        assert_eq!(pack_flinch(Pose::Ready, 0.0), Pose::Ready as u32 as f32);
        assert_eq!(pack_flinch(Pose::Ready, 0.5) as u32, Pose::Ready as u32);
    }
}

// CLIP CRAWL_BACK BEGIN
/// AN-02-S12: crawling backwards, solved by examples/fit_clip from the reference clip Moving Backward In Prone Position. Generated by scripts/clip_table.py: 24 samples over one loop (sample with `clip_at`),
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, shR, elR, reachR, abR, twist.
pub const CRAWL_BACK: [[f32; 11]; 24] = [
    [0.2645, 0.0426, 0.8719, 0.1368, -0.4950, -0.0577, 1.2028, 2.2348, -0.6000, 1.2023, 0.2202],
    [0.0779, 0.0442, 0.4143, 0.1373, -0.4051, -0.0008, 1.0541, 2.3439, -0.5965, 0.9637, 0.1368],
    [0.0139, 0.0383, 0.2016, 0.1263, -0.3133, 0.0612, 0.8051, 2.4810, -0.5699, 0.6932, 0.0676],
    [0.0049, 0.0334, 0.1367, 0.1244, -0.2545, 0.1180, 0.6169, 2.5370, -0.5292, 0.5071, 0.0380],
    [0.0136, 0.0356, 0.1115, 0.1388, -0.2262, 0.1638, 0.5764, 2.5279, -0.5347, 0.4211, 0.0716],
    [0.0385, 0.0402, 0.1260, 0.1516, -0.2236, 0.1962, 0.6146, 2.5113, -0.5789, 0.4074, 0.1310],
    [0.0970, 0.0458, 0.1855, 0.1564, -0.2268, 0.2139, 0.6597, 2.4701, -0.5546, 0.4123, 0.1400],
    [0.1977, 0.0462, 0.3705, 0.1496, -0.2422, 0.2086, 0.7048, 2.4070, -0.4099, 0.4149, 0.0874],
    [0.3029, 0.0480, 0.7048, 0.1337, -0.3092, 0.1760, 0.7887, 2.3258, -0.2238, 0.4348, 0.0228],
    [0.3853, 0.0541, 1.0213, 0.1257, -0.3929, 0.1253, 0.9274, 2.2093, -0.0973, 0.4740, -0.0223],
    [0.4462, 0.0396, 1.2252, 0.1305, -0.4529, 0.0718, 1.0746, 2.1256, -0.0694, 0.5288, -0.0178],
    [0.5029, 0.0128, 1.3762, 0.1383, -0.5068, 0.0205, 1.2100, 2.1335, -0.1293, 0.5897, 0.0342],
    [0.5655, 0.0014, 1.5169, 0.1405, -0.5668, -0.0326, 1.3648, 2.1619, -0.2402, 0.6514, 0.0858],
    [0.6274, -0.0002, 1.6550, 0.1363, -0.6294, -0.0805, 1.5464, 2.1421, -0.3829, 0.7294, 0.1255],
    [0.6848, 0.0129, 1.7848, 0.1297, -0.6895, -0.1146, 1.6960, 2.0836, -0.5195, 0.8161, 0.1737],
    [0.7392, 0.0474, 1.9018, 0.1235, -0.7469, -0.1327, 1.7511, 2.0163, -0.5882, 0.8878, 0.2340],
    [0.7940, 0.0567, 2.0075, 0.1150, -0.8041, -0.1356, 1.7440, 1.9626, -0.6000, 0.9352, 0.2872],
    [0.8538, 0.0231, 2.1044, 0.1081, -0.8633, -0.1296, 1.7409, 1.9387, -0.6000, 0.9466, 0.3081],
    [0.9073, -0.0062, 2.1722, 0.1070, -0.9142, -0.1249, 1.7279, 1.9445, -0.6000, 0.9427, 0.3066],
    [0.9256, -0.0121, 2.1737, 0.1093, -0.9305, -0.1256, 1.6274, 1.9770, -0.6000, 1.0049, 0.3162],
    [0.8915, -0.0035, 2.0992, 0.1129, -0.8965, -0.1284, 1.4339, 2.0568, -0.6000, 1.1531, 0.3394],
    [0.8083, 0.0158, 1.9595, 0.1164, -0.8151, -0.1275, 1.2599, 2.1583, -0.6000, 1.2871, 0.3608],
    [0.6854, 0.0325, 1.7343, 0.1209, -0.6938, -0.1184, 1.2016, 2.2024, -0.6000, 1.3327, 0.3585],
    [0.5051, 0.0389, 1.3759, 0.1283, -0.5780, -0.0970, 1.2218, 2.2025, -0.6000, 1.3113, 0.3058],
];
// CLIP CRAWL_BACK END

// CLIP REGISTRY BEGIN
// Generated by scripts/clip_table.py from the tables above: each table's offset in the GPU clip buffer.
pub const CRAWL_BACK_AT: u32 = 0;
pub const WALK_F_AT: u32 = 264;
pub const WALK_FL_AT: u32 = 792;
pub const WALK_L_AT: u32 = 1320;
pub const WALK_BL_AT: u32 = 1848;
pub const WALK_B_AT: u32 = 2376;
pub const WALK_BR_AT: u32 = 2904;
pub const WALK_R_AT: u32 = 3432;
pub const WALK_FR_AT: u32 = 3960;
pub const RUN_F_AT: u32 = 4488;
pub const RUN_FL_AT: u32 = 5016;
pub const RUN_L_AT: u32 = 5544;
pub const RUN_BL_AT: u32 = 6072;
pub const RUN_B_AT: u32 = 6600;
pub const RUN_BR_AT: u32 = 7128;
pub const RUN_R_AT: u32 = 7656;
pub const RUN_FR_AT: u32 = 8184;
pub const SPRINT_F_AT: u32 = 8712;
pub const SPRINT_FL_AT: u32 = 9240;
pub const SPRINT_L_AT: u32 = 9768;
pub const SPRINT_BL_AT: u32 = 10296;
pub const SPRINT_B_AT: u32 = 10824;
pub const SPRINT_BR_AT: u32 = 11352;
pub const SPRINT_R_AT: u32 = 11880;
pub const SPRINT_FR_AT: u32 = 12408;
pub const HIGH_CRAWL_AT: u32 = 12936;
/// Where the gaits' boot roll starts (after the tables; 24 loops of 48 rows: roll left, right, turn left, right).
pub const GAIT_TOE_AT: u32 = 13608;
pub const GAIT_TOE_FLOATS: u32 = 24 * 48 * 4;
/// Floats in the clip buffer.
pub const CLIP_FLOATS: u32 = 13608 + GAIT_TOE_FLOATS;
/// Every baked clip table back to back, rows in order (uploaded once; common.wgsl reads it as `clips`).
pub fn clip_buffer() -> Vec<f32> {
    let mut v = Vec::with_capacity(CLIP_FLOATS as usize);
    v.extend(CRAWL_BACK.iter().flatten());
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint] {
        for l in gait_loops(pose).expect("a baked gait") {
            v.extend(l.iter().flatten());
        }
    }
    v.extend(HIGH_CRAWL.iter().flatten());
    // Patch: Animations 2: each gait loop's boot roll, in the same order as the loops (common.wgsl GAIT_TOE_AT).
    for pose in [Pose::Walk, Pose::Run, Pose::Sprint] {
        for l in gait_toes(pose).expect("a baked gait") {
            v.extend(l.iter().flatten());
        }
    }
    v
}
// CLIP REGISTRY END

// CLIP WALK_F BEGIN
/// AN-02-S13: Walk, forward, from the Pro Rifle Pack clip "walk forward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_F: [[f32; 11]; 48] = [
    [0.7197, 0.0422, 0.8313, 0.8088, -0.3762, 0.3955, -0.7959, -0.0167, -0.0163, 0.0052, -0.1606],
    [0.7340, 0.0188, 0.9125, 0.8291, -0.3416, 0.4313, -0.7885, -0.0171, -0.0139, 0.0112, -0.1584],
    [0.7463, 0.0071, 0.9912, 0.8601, -0.3076, 0.4587, -0.7806, -0.0177, -0.0118, 0.0173, -0.1584],
    [0.7577, 0.0116, 1.0665, 0.8904, -0.2727, 0.4672, -0.7679, -0.0172, -0.0099, 0.0229, -0.1603],
    [0.7603, 0.0355, 1.1269, 0.9414, -0.2369, 0.4640, -0.7533, -0.0160, -0.0079, 0.0280, -0.1612],
    [0.7378, 0.1211, 1.1404, 1.0793, -0.1977, 0.4320, -0.7344, -0.0131, -0.0031, 0.0318, -0.1536],
    [0.7129, 0.2082, 1.1523, 1.2168, -0.1604, 0.4008, -0.7164, -0.0100, 0.0017, 0.0352, -0.1474],
    [0.6869, 0.2946, 1.1644, 1.3520, -0.1247, 0.3699, -0.6977, -0.0060, 0.0063, 0.0389, -0.1430],
    [0.6473, 0.3679, 1.1508, 1.3881, -0.0898, 0.3134, -0.6773, -0.0019, 0.0109, 0.0405, -0.1345],
    [0.6060, 0.4428, 1.1347, 1.4068, -0.0569, 0.2526, -0.6554, 0.0027, 0.0162, 0.0406, -0.1272],
    [0.5625, 0.5221, 1.1162, 1.4039, -0.0262, 0.1861, -0.6338, 0.0071, 0.0214, 0.0393, -0.1210],
    [0.5174, 0.5891, 1.0958, 1.3656, 0.0024, 0.1234, -0.6121, 0.0121, 0.0264, 0.0371, -0.1160],
    [0.4703, 0.6508, 1.0729, 1.3174, 0.0291, 0.0637, -0.5907, 0.0166, 0.0320, 0.0337, -0.1120],
    [0.4219, 0.7055, 1.0485, 1.2664, 0.0538, 0.0149, -0.5696, 0.0212, 0.0387, 0.0287, -0.1092],
    [0.3747, 0.7501, 1.0289, 1.2064, 0.0772, -0.0260, -0.5511, 0.0238, 0.0432, 0.0236, -0.1089],
    [0.3283, 0.7883, 1.0128, 1.1403, 0.0996, -0.0610, -0.5339, 0.0253, 0.0466, 0.0174, -0.1107],
    [0.2815, 0.8261, 0.9972, 1.0737, 0.1208, -0.0968, -0.5168, 0.0266, 0.0499, 0.0114, -0.1142],
    [0.2342, 0.8634, 0.9823, 1.0072, 0.1411, -0.1338, -0.4998, 0.0280, 0.0542, 0.0054, -0.1192],
    [0.1862, 0.9003, 0.9678, 0.9408, 0.1606, -0.1725, -0.4828, 0.0294, 0.0571, -0.0007, -0.1259],
    [0.1332, 0.9108, 0.9420, 0.8426, 0.1802, -0.2116, -0.4704, 0.0282, 0.0571, -0.0070, -0.1321],
    [0.0820, 0.9107, 0.9217, 0.7606, 0.2016, -0.2430, -0.4631, 0.0262, 0.0546, -0.0125, -0.1413],
    [0.0400, 0.8975, 0.9250, 0.7791, 0.2309, -0.2423, -0.4703, 0.0231, 0.0477, -0.0136, -0.1576],
    [-0.0057, 0.8786, 0.9193, 0.6547, 0.2622, -0.3499, -0.4777, 0.0203, 0.0412, -0.0150, -0.1743],
    [-0.0549, 0.8657, 0.9047, 0.7129, 0.2952, -0.3247, -0.4849, 0.0173, 0.0347, -0.0159, -0.1902],
    [0.0857, 0.8533, 1.0712, 0.7705, 0.1944, -0.3008, -0.4925, 0.0145, 0.0283, -0.0175, -0.1781],
    [0.0930, 0.8401, 1.1153, 0.8254, 0.1920, -0.2774, -0.4995, 0.0116, 0.0215, -0.0189, -0.1680],
    [0.0988, 0.8256, 1.1559, 0.8774, 0.1902, -0.2547, -0.5071, 0.0090, 0.0147, -0.0200, -0.1597],
    [0.1048, 0.8099, 1.1971, 0.9266, 0.1880, -0.2319, -0.5143, 0.0061, 0.0080, -0.0214, -0.1529],
    [0.1103, 0.7923, 1.2380, 0.9722, 0.1859, -0.2091, -0.5212, 0.0033, 0.0016, -0.0229, -0.1474],
    [0.1162, 0.7730, 1.2788, 1.0145, 0.1836, -0.1864, -0.5288, 0.0005, -0.0050, -0.0242, -0.1432],
    [0.1687, 0.7470, 1.3339, 1.0447, 0.1585, -0.1652, -0.5449, -0.0024, -0.0136, -0.0241, -0.1386],
    [0.2523, 0.7196, 1.3990, 1.0713, 0.1186, -0.1440, -0.5662, -0.0057, -0.0239, -0.0228, -0.1349],
    [0.3493, 0.6828, 1.4501, 1.0780, 0.0674, -0.1208, -0.5877, -0.0089, -0.0344, -0.0218, -0.1286],
    [0.4402, 0.6458, 1.4875, 1.0824, 0.0179, -0.0962, -0.6084, -0.0116, -0.0447, -0.0203, -0.1233],
    [0.5237, 0.6066, 1.5041, 1.0797, -0.0302, -0.0701, -0.6279, -0.0148, -0.0540, -0.0192, -0.1180],
    [0.6050, 0.5670, 1.4935, 1.0742, -0.0805, -0.0428, -0.6449, -0.0153, -0.0550, -0.0171, -0.1136],
    [0.6729, 0.5263, 1.4622, 1.0641, -0.1301, -0.0145, -0.6612, -0.0155, -0.0523, -0.0151, -0.1099],
    [0.6907, 0.4831, 1.3738, 1.0452, -0.1676, 0.0147, -0.6796, -0.0154, -0.0486, -0.0154, -0.1059],
    [0.7076, 0.4415, 1.2840, 1.0268, -0.2076, 0.0450, -0.6977, -0.0153, -0.0459, -0.0155, -0.1035],
    [0.7239, 0.4008, 1.1935, 1.0073, -0.2502, 0.0763, -0.7161, -0.0149, -0.0430, -0.0156, -0.1024],
    [0.7394, 0.3615, 1.1030, 0.9876, -0.2947, 0.1088, -0.7344, -0.0150, -0.0406, -0.0158, -0.1029],
    [0.7546, 0.3231, 1.0130, 0.9670, -0.3411, 0.1425, -0.7526, -0.0147, -0.0367, -0.0158, -0.1048],
    [0.7689, 0.2860, 0.9233, 0.9458, -0.3896, 0.1776, -0.7712, -0.0141, -0.0340, -0.0160, -0.1081],
    [0.7493, 0.2424, 0.8469, 0.9104, -0.4137, 0.2123, -0.7843, -0.0140, -0.0298, -0.0149, -0.1105],
    [0.7241, 0.1982, 0.7870, 0.8738, -0.4273, 0.2473, -0.7941, -0.0137, -0.0262, -0.0129, -0.1146],
    [0.7140, 0.1559, 0.7848, 0.8466, -0.4213, 0.2825, -0.7972, -0.0144, -0.0237, -0.0090, -0.1223],
    [0.7108, 0.1140, 0.6982, 0.8197, -0.4486, 0.3186, -0.8005, -0.0154, -0.0209, -0.0051, -0.1319],
    [0.7044, 0.0724, 0.7499, 0.7929, -0.4120, 0.3560, -0.8036, -0.0159, -0.0182, -0.0011, -0.1435],
];
// CLIP WALK_F END

// CLIP WALK_FL BEGIN
/// AN-02-S13: Walk, forward left, from the Pro Rifle Pack clip "walk forward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_FL: [[f32; 11]; 48] = [
    [0.7263, -0.0344, 0.7837, 0.7092, -0.3451, 0.4407, -0.0202, -0.0156, -0.0159, 0.0101, -0.1522],
    [0.7469, -0.0531, 0.8735, 0.7232, -0.3108, 0.4779, -0.0119, -0.0114, -0.0123, 0.0143, -0.1515],
    [0.7674, -0.0668, 0.9632, 0.7409, -0.2769, 0.5113, -0.0022, -0.0064, -0.0085, 0.0182, -0.1536],
    [0.7814, -0.0751, 1.0406, 0.7549, -0.2416, 0.5274, 0.0112, -0.0021, -0.0049, 0.0213, -0.1560],
    [0.7813, -0.0557, 1.0936, 0.8191, -0.2047, 0.5316, 0.0262, 0.0036, 0.0004, 0.0232, -0.1554],
    [0.7636, 0.0075, 1.1157, 0.9616, -0.1665, 0.5197, 0.0432, 0.0096, 0.0073, 0.0237, -0.1499],
    [0.7387, 0.0973, 1.1257, 1.1040, -0.1289, 0.4851, 0.0621, 0.0145, 0.0123, 0.0227, -0.1434],
    [0.7093, 0.1930, 1.1291, 1.2306, -0.0926, 0.4416, 0.0822, 0.0193, 0.0168, 0.0213, -0.1371],
    [0.6701, 0.2872, 1.1147, 1.3039, -0.0577, 0.3785, 0.1039, 0.0231, 0.0224, 0.0185, -0.1285],
    [0.6284, 0.3772, 1.0971, 1.3434, -0.0250, 0.3131, 0.1259, 0.0259, 0.0271, 0.0146, -0.1209],
    [0.5853, 0.4655, 1.0784, 1.3605, 0.0056, 0.2468, 0.1479, 0.0274, 0.0312, 0.0099, -0.1146],
    [0.5395, 0.5453, 1.0553, 1.3387, 0.0340, 0.1800, 0.1701, 0.0279, 0.0358, 0.0051, -0.1089],
    [0.4924, 0.6225, 1.0313, 1.3116, 0.0604, 0.1199, 0.1917, 0.0279, 0.0398, -0.0005, -0.1045],
    [0.4449, 0.6961, 1.0077, 1.2767, 0.0850, 0.0690, 0.2132, 0.0277, 0.0456, -0.0072, -0.1015],
    [0.4013, 0.7220, 0.9950, 1.2407, 0.1089, 0.0411, 0.2232, 0.0255, 0.0455, -0.0093, -0.1022],
    [0.3581, 0.7355, 0.9842, 1.2040, 0.1326, 0.0187, 0.2299, 0.0227, 0.0429, -0.0104, -0.1046],
    [0.3145, 0.7484, 0.9730, 1.1666, 0.1561, -0.0040, 0.2368, 0.0197, 0.0404, -0.0111, -0.1085],
    [0.2704, 0.7613, 0.9615, 1.1292, 0.1795, -0.0270, 0.2440, 0.0165, 0.0377, -0.0121, -0.1138],
    [0.2256, 0.7734, 0.9490, 1.0915, 0.2028, -0.0503, 0.2510, 0.0135, 0.0348, -0.0130, -0.1204],
    [0.1802, 0.8571, 0.9360, 0.6168, 0.2264, -0.3553, 0.2577, 0.0106, 0.0327, -0.0135, -0.1286],
    [0.1337, 0.8468, 0.9216, 0.6710, 0.2501, -0.3202, 0.2648, 0.0076, 0.0301, -0.0143, -0.1381],
    [0.0865, 0.8496, 0.9065, 0.7481, 0.2742, -0.2894, 0.2718, 0.0048, 0.0283, -0.0152, -0.1491],
    [0.0380, 0.8331, 0.8897, 0.7915, 0.2989, -0.2583, 0.2786, 0.0019, 0.0253, -0.0157, -0.1616],
    [-0.0122, 0.8258, 0.8703, 0.8515, 0.3242, -0.2301, 0.2856, -0.0010, 0.0227, -0.0164, -0.1550],
    [-0.0657, 0.8182, 0.8926, 0.9115, 0.2800, -0.2034, 0.2926, -0.0042, 0.0205, -0.0171, -0.1499],
    [-0.1105, 0.8115, 0.8829, 0.9741, 0.2958, -0.1781, 0.2997, -0.0070, 0.0177, -0.0177, -0.1476],
    [-0.1549, 0.8008, 0.8730, 1.0309, 0.3124, -0.1536, 0.3065, -0.0104, 0.0152, -0.0186, -0.1462],
    [-0.0828, 0.7965, 1.0131, 1.1004, 0.2854, -0.1372, 0.2899, -0.0084, 0.0056, -0.0146, -0.1504],
    [0.0021, 0.7697, 1.1488, 1.1287, 0.2527, -0.1180, 0.2722, -0.0065, -0.0030, -0.0112, -0.1474],
    [0.1043, 0.7425, 1.2740, 1.1548, 0.2135, -0.0981, 0.2519, -0.0050, -0.0107, -0.0077, -0.1455],
    [0.2024, 0.7081, 1.3601, 1.1659, 0.1725, -0.0765, 0.2308, -0.0043, -0.0182, -0.0049, -0.1417],
    [0.2974, 0.6716, 1.4295, 1.1708, 0.1314, -0.0534, 0.2095, -0.0034, -0.0251, -0.0023, -0.1380],
    [0.3866, 0.6299, 1.4734, 1.1627, 0.0873, -0.0287, 0.1878, -0.0035, -0.0319, 0.0007, -0.1329],
    [0.4677, 0.5863, 1.4995, 1.1477, 0.0426, -0.0027, 0.1673, -0.0044, -0.0385, 0.0030, -0.1277],
    [0.5420, 0.5422, 1.5127, 1.1291, -0.0037, 0.0244, 0.1483, -0.0058, -0.0444, 0.0047, -0.1232],
    [0.5696, 0.4957, 1.4068, 1.1020, -0.0480, 0.0525, 0.1301, -0.0078, -0.0429, 0.0042, -0.1185],
    [0.5958, 0.4502, 1.2973, 1.0743, -0.0949, 0.0815, 0.1124, -0.0098, -0.0418, 0.0039, -0.1151],
    [0.7209, 0.4059, 1.0094, 1.0454, -0.2642, 0.1115, 0.0941, -0.0120, -0.0401, 0.0031, -0.1130],
    [0.6791, 0.3633, 1.0109, 1.0170, -0.2352, 0.1426, 0.0760, -0.0140, -0.0383, 0.0024, -0.1126],
    [0.6435, 0.3214, 1.0219, 0.9872, -0.2065, 0.1748, 0.0580, -0.0161, -0.0373, 0.0020, -0.1135],
    [0.6145, 0.2811, 1.0438, 0.9575, -0.1778, 0.2082, 0.0400, -0.0180, -0.0355, 0.0009, -0.1159],
    [0.7195, 0.2418, 0.7426, 0.9267, -0.3636, 0.2429, 0.0218, -0.0202, -0.0343, 0.0001, -0.1198],
    [0.7401, 0.2036, 0.6301, 0.8955, -0.4252, 0.2788, 0.0036, -0.0214, -0.0326, -0.0007, -0.1252],
    [0.6974, 0.1543, 0.5398, 0.8408, -0.4393, 0.3130, -0.0082, -0.0225, -0.0291, -0.0015, -0.1283],
    [0.6701, 0.1024, 0.4943, 0.7790, -0.4474, 0.3473, -0.0184, -0.0227, -0.0264, -0.0013, -0.1323],
    [0.6686, 0.0449, 0.5166, 0.7065, -0.4443, 0.3809, -0.0250, -0.0229, -0.0244, 0.0002, -0.1369],
    [0.6868, -0.0010, 0.5997, 0.6673, -0.4134, 0.4170, -0.0266, -0.0216, -0.0225, 0.0027, -0.1492],
    [0.7076, -0.0417, 0.6954, 0.6446, -0.3795, 0.4552, -0.0252, -0.0196, -0.0193, 0.0063, -0.1563],
];
// CLIP WALK_FL END

// CLIP WALK_L BEGIN
/// AN-02-S13: Walk, left, from the Pro Rifle Pack clip "walk left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_L: [[f32; 11]; 48] = [
    [0.3029, 0.7412, 1.1037, 0.8739, -0.3278, 0.5641, -0.7040, 0.0198, 0.0046, 0.0021, -0.1728],
    [0.3302, 0.7514, 1.1143, 0.9393, -0.2827, 0.5311, -0.7258, 0.0177, 0.0020, -0.0009, -0.1619],
    [0.3590, 0.7617, 1.1238, 1.0044, -0.2397, 0.4984, -0.7476, 0.0161, -0.0009, -0.0036, -0.1526],
    [0.3853, 0.7428, 1.1231, 1.0666, -0.1980, 0.4326, -0.7690, 0.0140, -0.0015, -0.0069, -0.1430],
    [0.4116, 0.6930, 1.1188, 1.1963, -0.1583, 0.3138, -0.7903, 0.0125, -0.0020, -0.0099, -0.1344],
    [0.4392, 0.7072, 1.1141, 1.1336, -0.1207, 0.3428, -0.8114, 0.0105, -0.0016, -0.0133, -0.1276],
    [0.4670, 0.6515, 1.1072, 1.2668, -0.0850, 0.2340, -0.8326, 0.0090, -0.0018, -0.0163, -0.1220],
    [0.4982, 0.6181, 1.1048, 1.3390, -0.0512, 0.1767, -0.8539, 0.0077, -0.0024, -0.0197, -0.1190],
    [0.5296, 0.5831, 1.1002, 1.4111, -0.0191, 0.1240, -0.8753, 0.0064, -0.0024, -0.0225, -0.1171],
    [0.5642, 0.5406, 1.0998, 1.4522, 0.0113, 0.0789, -0.8971, 0.0043, -0.0035, -0.0246, -0.1177],
    [0.6032, 0.4881, 1.1057, 1.4459, 0.0402, 0.0382, -0.9201, 0.0015, -0.0058, -0.0254, -0.1211],
    [0.6432, 0.4347, 1.1106, 1.4386, 0.0677, -0.0024, -0.9431, -0.0015, -0.0082, -0.0259, -0.1259],
    [0.6840, 0.3813, 1.1140, 1.4319, 0.0938, -0.0433, -0.9662, -0.0041, -0.0108, -0.0268, -0.1319],
    [0.7272, 0.3279, 1.1188, 1.4251, 0.1188, -0.0842, -0.9894, -0.0072, -0.0129, -0.0277, -0.1399],
    [0.7617, 0.2862, 1.1044, 1.3982, 0.1436, -0.1140, -1.0049, -0.0104, -0.0165, -0.0265, -0.1455],
    [0.7902, 0.2524, 1.0776, 1.3581, 0.1686, -0.1355, -1.0150, -0.0137, -0.0205, -0.0240, -0.1503],
    [0.8189, 0.2183, 1.0504, 1.3182, 0.1934, -0.1566, -1.0251, -0.0176, -0.0246, -0.0218, -0.1569],
    [0.8484, 0.1841, 1.0237, 1.2788, 0.2181, -0.1771, -1.0356, -0.0212, -0.0289, -0.0193, -0.1654],
    [0.8778, 0.1494, 0.9959, 1.2398, 0.2428, -0.1970, -1.0460, -0.0249, -0.0329, -0.0171, -0.1756],
    [0.9077, 0.1148, 0.9676, 1.2012, 0.2680, -0.2165, -1.0564, -0.0286, -0.0371, -0.0150, -0.1878],
    [0.9374, 0.0801, 0.9378, 1.1633, 0.2937, -0.2355, -1.0666, -0.0323, -0.0414, -0.0129, -0.2018],
    [0.9671, 0.0854, 0.9064, 1.1496, 0.3205, -0.2545, -1.0764, -0.0361, -0.0456, -0.0105, -0.2166],
    [0.9849, 0.1376, 0.8620, 1.1585, 0.3646, -0.2305, -1.0594, -0.0367, -0.0463, -0.0075, -0.2014],
    [1.0311, 0.1803, 0.9332, 1.1413, 0.2685, -0.2112, -1.0244, -0.0353, -0.0454, -0.0037, -0.1829],
    [1.0275, 0.2204, 0.9817, 1.1245, 0.2457, -0.1916, -0.9857, -0.0327, -0.0349, 0.0006, -0.1666],
    [1.0168, 0.2597, 1.0417, 1.1133, 0.2213, -0.1704, -0.9464, -0.0296, -0.0236, 0.0050, -0.1533],
    [0.9961, 0.3001, 1.1204, 1.1118, 0.1955, -0.1478, -0.9062, -0.0260, -0.0119, 0.0098, -0.1438],
    [0.9596, 0.3353, 1.1930, 1.1040, 0.1573, -0.1224, -0.8662, -0.0225, -0.0002, 0.0145, -0.1344],
    [0.9156, 0.3682, 1.2593, 1.0969, 0.1133, -0.0949, -0.8265, -0.0187, 0.0127, 0.0195, -0.1268],
    [0.8589, 0.3956, 1.3092, 1.0833, 0.0527, -0.0648, -0.7887, -0.0143, 0.0257, 0.0233, -0.1193],
    [0.7988, 0.4247, 1.3441, 1.0763, -0.0138, -0.0318, -0.7649, -0.0099, 0.0305, 0.0245, -0.1145],
    [0.7359, 0.4545, 1.3697, 1.0719, -0.0837, 0.0029, -0.7509, -0.0059, 0.0298, 0.0240, -0.1119],
    [0.6717, 0.4828, 1.3946, 1.0669, -0.1524, 0.0383, -0.7365, -0.0018, 0.0281, 0.0233, -0.1107],
    [0.6060, 0.5115, 1.3920, 1.0650, -0.2208, 0.0746, -0.7222, 0.0022, 0.0282, 0.0232, -0.1119],
    [0.5377, 0.5448, 1.3483, 1.0748, -0.2903, 0.1122, -0.7086, 0.0064, 0.0278, 0.0229, -0.1171],
    [0.4693, 0.5767, 1.3042, 1.0838, -0.3596, 0.1513, -0.6942, 0.0098, 0.0275, 0.0230, -0.1239],
    [0.4068, 0.6080, 1.2584, 1.0930, -0.4238, 0.1919, -0.6803, 0.0137, 0.0278, 0.0224, -0.1325],
    [0.3682, 0.6358, 1.2052, 1.0969, -0.4672, 0.2338, -0.6667, 0.0168, 0.0270, 0.0211, -0.1419],
    [0.3294, 0.6640, 1.1518, 1.1032, -0.5100, 0.2777, -0.6532, 0.0201, 0.0275, 0.0199, -0.1538],
    [0.2904, 0.6905, 1.0978, 1.1074, -0.5522, 0.3231, -0.6404, 0.0239, 0.0281, 0.0187, -0.1672],
    [0.2520, 0.7156, 1.0440, 1.1114, -0.5942, 0.3708, -0.6268, 0.0269, 0.0283, 0.0174, -0.1829],
    [0.2289, 0.7287, 1.0130, 1.0936, -0.6196, 0.4176, -0.6141, 0.0295, 0.0289, 0.0157, -0.1957],
    [0.2280, 0.7258, 1.0173, 1.0460, -0.6215, 0.4612, -0.6032, 0.0302, 0.0300, 0.0127, -0.2038],
    [0.2216, 0.7134, 1.0221, 0.9676, -0.5921, 0.4984, -0.6053, 0.0302, 0.0214, 0.0105, -0.2076],
    [0.2170, 0.7046, 1.0308, 0.8882, -0.5492, 0.5346, -0.6143, 0.0294, 0.0110, 0.0087, -0.2143],
    [0.2297, 0.7195, 1.0653, 0.8338, -0.4754, 0.5722, -0.6379, 0.0277, 0.0061, 0.0078, -0.2172],
    [0.2524, 0.7210, 1.0793, 0.7976, -0.4239, 0.6012, -0.6603, 0.0245, 0.0056, 0.0063, -0.2004],
    [0.2767, 0.7318, 1.0918, 0.8096, -0.3747, 0.5968, -0.6824, 0.0220, 0.0073, 0.0051, -0.1856],
];
// CLIP WALK_L END

// CLIP WALK_BL BEGIN
/// AN-02-S13: Walk, backward left, from the Pro Rifle Pack clip "walk backward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_BL: [[f32; 11]; 48] = [
    [-0.0682, 0.7629, 0.7065, 0.8606, 0.0235, 0.0822, -0.7964, -0.0095, -0.0379, 0.0250, -0.1082],
    [-0.0072, 0.7551, 0.7312, 0.9905, 0.0313, 0.0729, -0.8202, -0.0114, -0.0379, 0.0217, -0.0981],
    [0.0547, 0.7382, 0.7589, 1.1094, 0.0366, 0.0698, -0.8443, -0.0129, -0.0379, 0.0177, -0.0904],
    [0.1150, 0.7010, 0.7840, 1.1891, 0.0399, 0.0722, -0.8706, -0.0138, -0.0371, 0.0132, -0.0842],
    [0.1771, 0.6506, 0.8144, 1.2450, 0.0411, 0.0774, -0.8989, -0.0141, -0.0278, 0.0083, -0.0808],
    [0.2407, 0.5912, 0.8484, 1.2851, 0.0398, 0.0858, -0.9281, -0.0138, -0.0178, 0.0035, -0.0799],
    [0.2984, 0.5107, 0.8691, 1.2473, 0.0353, 0.0982, -0.9568, -0.0133, -0.0135, -0.0005, -0.0785],
    [0.3555, 0.4285, 0.8890, 1.2068, 0.0281, 0.1123, -0.9852, -0.0119, -0.0089, -0.0042, -0.0787],
    [0.4130, 0.3461, 0.9101, 1.1652, 0.0182, 0.1286, -1.0137, -0.0106, -0.0045, -0.0077, -0.0808],
    [0.4702, 0.2637, 0.9312, 1.1217, 0.0053, 0.1473, -1.0422, -0.0092, -0.0008, -0.0111, -0.0847],
    [0.5274, 0.1820, 0.9526, 1.0766, -0.0105, 0.1687, -1.0706, -0.0068, 0.0038, -0.0140, -0.0905],
    [0.5844, 0.1012, 0.9741, 1.0294, -0.0295, 0.1931, -1.0993, -0.0042, 0.0084, -0.0165, -0.0982],
    [0.6409, 0.0221, 0.9951, 0.9799, -0.0518, 0.2207, -1.1278, -0.0015, 0.0130, -0.0187, -0.1077],
    [0.6964, -0.0551, 1.0148, 0.9272, -0.0776, 0.2515, -1.1566, 0.0013, 0.0169, -0.0207, -0.1189],
    [0.7311, -0.1242, 0.9912, 0.8363, -0.1032, 0.2779, -1.1744, 0.0033, 0.0142, -0.0205, -0.1240],
    [0.7508, -0.1683, 0.9373, 0.7641, -0.1271, 0.2887, -1.1832, 0.0047, 0.0108, -0.0183, -0.1256],
    [0.7490, -0.1745, 0.8386, 0.7230, -0.1458, 0.2754, -1.1784, 0.0058, 0.0094, -0.0140, -0.1220],
    [0.7465, -0.1804, 0.7388, 0.6826, -0.1644, 0.2617, -1.1733, 0.0069, 0.0066, -0.0093, -0.1221],
    [0.7436, -0.1859, 0.6379, 0.6417, -0.1835, 0.2485, -1.1686, 0.0083, 0.0048, -0.0049, -0.1258],
    [0.7423, -0.3320, 0.5384, 0.5075, -0.2031, 0.3165, -1.1631, 0.0097, 0.0028, -0.0005, -0.1339],
    [0.7429, -0.2825, 0.4403, 0.5042, -0.2243, 0.2820, -1.1584, 0.0112, 0.0005, 0.0041, -0.1465],
    [0.7469, -0.2375, 0.3451, 0.4933, -0.2473, 0.2508, -1.1534, 0.0124, -0.0015, 0.0085, -0.1317],
    [0.8071, -0.1715, 0.3270, 0.5646, -0.2530, 0.2117, -1.1165, 0.0186, 0.0132, 0.0147, -0.1206],
    [0.7760, -0.1079, 0.6313, 0.6276, -0.2105, 0.1761, -1.0778, 0.0255, 0.0283, 0.0192, -0.1115],
    [0.8006, -0.0513, 0.7395, 0.6715, -0.1898, 0.1431, -1.0359, 0.0326, 0.0436, 0.0214, -0.1023],
    [0.7992, 0.0056, 0.8336, 0.7093, -0.1709, 0.1160, -0.9982, 0.0352, 0.0486, 0.0182, -0.0944],
    [0.7906, 0.0624, 0.9243, 0.7430, -0.1551, 0.0935, -0.9622, 0.0358, 0.0514, 0.0137, -0.0879],
    [0.7823, 0.1192, 1.0147, 0.7748, -0.1412, 0.0748, -0.9257, 0.0363, 0.0539, 0.0096, -0.0829],
    [0.7581, 0.1808, 1.0914, 0.8125, -0.1303, 0.0613, -0.8960, 0.0337, 0.0480, 0.0048, -0.0807],
    [0.7242, 0.2456, 1.1598, 0.8534, -0.1202, 0.0518, -0.8713, 0.0283, 0.0360, -0.0009, -0.0808],
    [0.6903, 0.3105, 1.2283, 0.8933, -0.1101, 0.0447, -0.8459, 0.0236, 0.0243, -0.0058, -0.0828],
    [0.6469, 0.3750, 1.2857, 0.9307, -0.1014, 0.0402, -0.8208, 0.0174, 0.0111, -0.0100, -0.0862],
    [0.5883, 0.4381, 1.3266, 0.9625, -0.0945, 0.0383, -0.7964, 0.0102, -0.0010, -0.0129, -0.0906],
    [0.5134, 0.4895, 1.3216, 0.9660, -0.0867, 0.0385, -0.7780, 0.0039, -0.0036, -0.0137, -0.0917],
    [0.4335, 0.5389, 1.3043, 0.9644, -0.0772, 0.0406, -0.7605, -0.0014, -0.0026, -0.0139, -0.0936],
    [0.3526, 0.5865, 1.2860, 0.9591, -0.0685, 0.0445, -0.7433, -0.0068, -0.0015, -0.0136, -0.0967],
    [0.2717, 0.6350, 1.2676, 0.9553, -0.0609, 0.0504, -0.7260, -0.0122, -0.0011, -0.0134, -0.1019],
    [0.1908, 0.6840, 1.2488, 0.9519, -0.0538, 0.0583, -0.7090, -0.0175, 0.0006, -0.0126, -0.1092],
    [0.1101, 0.7328, 1.2293, 0.9477, -0.0482, 0.0684, -0.6920, -0.0228, 0.0014, -0.0112, -0.1182],
    [0.0265, 0.7674, 1.1775, 0.9154, -0.0450, 0.0789, -0.6786, -0.0263, -0.0003, -0.0097, -0.1242],
    [-0.0628, 0.7810, 1.0769, 0.8430, -0.0442, 0.0878, -0.6715, -0.0284, -0.0071, -0.0066, -0.1253],
    [-0.1376, 0.7679, 0.9642, 0.7210, -0.0422, 0.0945, -0.6696, -0.0279, -0.0133, -0.0038, -0.1217],
    [-0.1945, 0.7533, 0.8609, 0.5939, -0.0378, 0.1000, -0.6714, -0.0261, -0.0207, 0.0002, -0.1222],
    [-0.1950, 0.7656, 0.8029, 0.5079, -0.0228, 0.0991, -0.6868, -0.0202, -0.0268, 0.0066, -0.1332],
    [-0.1947, 0.7791, 0.7459, 0.4218, -0.0079, 0.0972, -0.7020, -0.0146, -0.0334, 0.0135, -0.1483],
    [-0.2424, 0.7958, 0.6618, 0.3372, -0.0220, 0.0937, -0.7176, -0.0089, -0.0379, 0.0200, -0.1556],
    [-0.1839, 0.8476, 0.6761, 0.3089, -0.0032, 0.0805, -0.7438, -0.0092, -0.0379, 0.0218, -0.1374],
    [-0.1257, 0.9006, 0.6914, 0.2769, 0.0119, 0.0626, -0.7702, -0.0096, -0.0379, 0.0231, -0.1217],
];
// CLIP WALK_BL END

// CLIP WALK_B BEGIN
/// AN-02-S13: Walk, backward, from the Pro Rifle Pack clip "walk backward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_B: [[f32; 11]; 48] = [
    [-0.2902, 0.8757, 0.4573, 0.7079, 0.3692, -0.2609, -0.4267, 0.0218, 0.0412, -0.0326, -0.1689],
    [-0.2071, 0.8735, 0.5545, 0.6201, 0.3425, -0.2832, -0.4225, 0.0243, 0.0471, -0.0349, -0.1572],
    [-0.1098, 0.9023, 0.6877, 0.5859, 0.3235, -0.3153, -0.4242, 0.0287, 0.0501, -0.0337, -0.1552],
    [-0.0238, 0.9478, 0.7935, 0.6753, 0.3051, -0.2049, -0.4277, 0.0327, 0.0527, -0.0316, -0.1537],
    [0.0478, 0.9685, 0.8610, 0.7724, 0.2859, -0.1679, -0.4345, 0.0359, 0.0542, -0.0285, -0.1487],
    [0.1056, 0.9526, 0.8914, 0.8601, 0.2665, -0.1335, -0.4477, 0.0363, 0.0535, -0.0237, -0.1394],
    [0.1587, 0.9070, 0.9109, 1.0173, 0.2467, -0.0663, -0.4624, 0.0358, 0.0511, -0.0181, -0.1299],
    [0.2112, 0.9208, 0.9307, 0.9593, 0.2265, -0.0978, -0.4771, 0.0350, 0.0493, -0.0126, -0.1221],
    [0.2630, 0.8742, 0.9502, 1.1131, 0.2056, -0.0288, -0.4913, 0.0343, 0.0473, -0.0072, -0.1160],
    [0.3144, 0.8484, 0.9703, 1.1980, 0.1844, 0.0074, -0.5062, 0.0338, 0.0453, -0.0020, -0.1116],
    [0.3686, 0.7790, 0.9975, 1.2488, 0.1632, 0.0516, -0.5269, 0.0281, 0.0400, 0.0030, -0.1103],
    [0.4222, 0.7085, 1.0247, 1.2989, 0.1404, 0.0947, -0.5473, 0.0223, 0.0339, 0.0077, -0.1106],
    [0.4751, 0.6382, 1.0515, 1.3492, 0.1161, 0.1365, -0.5682, 0.0165, 0.0283, 0.0119, -0.1123],
    [0.5285, 0.5565, 1.0805, 1.3777, 0.0901, 0.1970, -0.5900, 0.0101, 0.0230, 0.0161, -0.1161],
    [0.5744, 0.4676, 1.0948, 1.3790, 0.0617, 0.2608, -0.6116, 0.0037, 0.0181, 0.0190, -0.1185],
    [0.6191, 0.3627, 1.1085, 1.2946, 0.0314, 0.3188, -0.6333, -0.0021, 0.0142, 0.0198, -0.1224],
    [0.6691, 0.2579, 1.1343, 1.2071, -0.0010, 0.3774, -0.6544, -0.0078, 0.0098, 0.0206, -0.1304],
    [0.7131, 0.1544, 1.1496, 1.1180, -0.0359, 0.4374, -0.6761, -0.0135, 0.0060, 0.0210, -0.1380],
    [0.7569, 0.0526, 1.1659, 1.0262, -0.0733, 0.4990, -0.6975, -0.0190, 0.0024, 0.0211, -0.1476],
    [0.7876, -0.0068, 1.1585, 0.9555, -0.1120, 0.5332, -0.7162, -0.0228, -0.0008, 0.0205, -0.1537],
    [0.7997, -0.0052, 1.1177, 0.9198, -0.1501, 0.5229, -0.7317, -0.0246, -0.0038, 0.0188, -0.1543],
    [0.8110, -0.0031, 1.0781, 0.8861, -0.1893, 0.5119, -0.7467, -0.0263, -0.0056, 0.0168, -0.1572],
    [0.8148, 0.0026, 1.0270, 0.8577, -0.2288, 0.4962, -0.7610, -0.0272, -0.0084, 0.0143, -0.1596],
    [0.7933, 0.0226, 0.9306, 0.8497, -0.2646, 0.4636, -0.7713, -0.0265, -0.0099, 0.0117, -0.1550],
    [0.7786, 0.0799, 0.8442, 0.8726, -0.3001, 0.4078, -0.7768, -0.0251, -0.0121, 0.0089, -0.1557],
    [0.7666, 0.1167, 0.7614, 0.8743, -0.3358, 0.3672, -0.7796, -0.0236, -0.0147, 0.0071, -0.1571],
    [0.7532, 0.1522, 0.6761, 0.8740, -0.3726, 0.3281, -0.7822, -0.0223, -0.0168, 0.0048, -0.1401],
    [0.7444, 0.1913, 0.5954, 0.8874, -0.4099, 0.2908, -0.7813, -0.0213, -0.0202, 0.0031, -0.1279],
    [0.7382, 0.2376, 0.6271, 0.9238, -0.3941, 0.2551, -0.7745, -0.0203, -0.0239, 0.0019, -0.1220],
    [0.7626, 0.2844, 0.6743, 0.9634, -0.3844, 0.2197, -0.7632, -0.0196, -0.0283, 0.0021, -0.1188],
    [0.7854, 0.3304, 0.7486, 1.0003, -0.3640, 0.1848, -0.7493, -0.0193, -0.0328, 0.0023, -0.1170],
    [0.7764, 0.3695, 0.8638, 1.0215, -0.3128, 0.1497, -0.7308, -0.0190, -0.0367, 0.0041, -0.1139],
    [0.7436, 0.4094, 1.0487, 1.0413, -0.2246, 0.1159, -0.7118, -0.0186, -0.0409, 0.0056, -0.1123],
    [0.7823, 0.4500, 1.0361, 1.0597, -0.2546, 0.0833, -0.6927, -0.0180, -0.0449, 0.0072, -0.1120],
    [0.7453, 0.4918, 1.2198, 1.0770, -0.1720, 0.0521, -0.6737, -0.0171, -0.0486, 0.0087, -0.1132],
    [0.7339, 0.5346, 1.3398, 1.0932, -0.1293, 0.0220, -0.6554, -0.0166, -0.0530, 0.0101, -0.1158],
    [0.7218, 0.5774, 1.4575, 1.1064, -0.0884, -0.0069, -0.6364, -0.0159, -0.0565, 0.0116, -0.1194],
    [0.6427, 0.6203, 1.4973, 1.1165, -0.0299, -0.0349, -0.6190, -0.0148, -0.0603, 0.0105, -0.1239],
    [0.5498, 0.6640, 1.5102, 1.1255, 0.0310, -0.0619, -0.6012, -0.0133, -0.0606, 0.0086, -0.1299],
    [0.4675, 0.7038, 1.4953, 1.1244, 0.0832, -0.0875, -0.5821, -0.0094, -0.0527, 0.0067, -0.1353],
    [0.3735, 0.7400, 1.4492, 1.1141, 0.1299, -0.1114, -0.5616, -0.0051, -0.0428, 0.0037, -0.1404],
    [0.2712, 0.7735, 1.3819, 1.0969, 0.1751, -0.1334, -0.5395, -0.0010, -0.0322, -0.0006, -0.1455],
    [0.1695, 0.8094, 1.3150, 1.0821, 0.2223, -0.1545, -0.5182, 0.0029, -0.0216, -0.0049, -0.1529],
    [0.0627, 0.8405, 1.2181, 1.0567, 0.2659, -0.1740, -0.4974, 0.0066, -0.0113, -0.0096, -0.1596],
    [-0.0518, 0.8624, 1.0763, 1.0138, 0.3061, -0.1918, -0.4777, 0.0104, 0.0007, -0.0149, -0.1644],
    [-0.1468, 0.8714, 0.9125, 0.9478, 0.3352, -0.2080, -0.4603, 0.0139, 0.0126, -0.0204, -0.1664],
    [-0.2410, 0.8767, 0.7260, 0.8749, 0.3623, -0.2238, -0.4447, 0.0169, 0.0238, -0.0253, -0.1695],
    [-0.3269, 0.8774, 0.4751, 0.7938, 0.4077, -0.2411, -0.4336, 0.0193, 0.0339, -0.0297, -0.1739],
];
// CLIP WALK_B END

// CLIP WALK_BR BEGIN
/// AN-02-S13: Walk, backward right, from the Pro Rifle Pack clip "walk backward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_BR: [[f32; 11]; 48] = [
    [-0.0987, 0.8585, 0.7511, 0.6434, 0.3485, -0.3266, 0.3119, -0.0104, 0.0364, -0.0270, -0.1746],
    [-0.0310, 0.8769, 0.8111, 0.5930, 0.3222, -0.3598, 0.3111, -0.0071, 0.0416, -0.0297, -0.1643],
    [0.0373, 0.9167, 0.8721, 0.6923, 0.2980, -0.2406, 0.3085, -0.0030, 0.0461, -0.0321, -0.1569],
    [0.1041, 0.9335, 0.9279, 0.7741, 0.2758, -0.2036, 0.3018, 0.0015, 0.0492, -0.0335, -0.1511],
    [0.1619, 0.9241, 0.9596, 0.8514, 0.2535, -0.1669, 0.2904, 0.0061, 0.0510, -0.0317, -0.1429],
    [0.2109, 0.8565, 0.9695, 1.0589, 0.2304, -0.0610, 0.2763, 0.0109, 0.0512, -0.0287, -0.1324],
    [0.2617, 0.8720, 0.9860, 1.0045, 0.2075, -0.0930, 0.2617, 0.0156, 0.0521, -0.0246, -0.1250],
    [0.3094, 0.8872, 0.9968, 0.9514, 0.1837, -0.1266, 0.2475, 0.0198, 0.0526, -0.0217, -0.1181],
    [0.3564, 0.8170, 1.0075, 1.1513, 0.1594, -0.0179, 0.2332, 0.0247, 0.0527, -0.0180, -0.1130],
    [0.4075, 0.7600, 1.0295, 1.2056, 0.1351, 0.0278, 0.2139, 0.0269, 0.0486, -0.0117, -0.1117],
    [0.4543, 0.6939, 1.0431, 1.2531, 0.1092, 0.0734, 0.1937, 0.0281, 0.0437, -0.0046, -0.1104],
    [0.5048, 0.6277, 1.0670, 1.3007, 0.0820, 0.1178, 0.1729, 0.0292, 0.0381, 0.0025, -0.1127],
    [0.5527, 0.5549, 1.0865, 1.3348, 0.0529, 0.1759, 0.1518, 0.0296, 0.0333, 0.0090, -0.1158],
    [0.6001, 0.4773, 1.1067, 1.3588, 0.0217, 0.2435, 0.1304, 0.0290, 0.0284, 0.0149, -0.1207],
    [0.6385, 0.3841, 1.1102, 1.2879, -0.0116, 0.2980, 0.1092, 0.0256, 0.0234, 0.0181, -0.1238],
    [0.6758, 0.2909, 1.1140, 1.2143, -0.0468, 0.3535, 0.0880, 0.0220, 0.0191, 0.0217, -0.1286],
    [0.7127, 0.1994, 1.1191, 1.1392, -0.0843, 0.4109, 0.0670, 0.0185, 0.0144, 0.0249, -0.1354],
    [0.7481, 0.1096, 1.1236, 1.0615, -0.1240, 0.4698, 0.0459, 0.0151, 0.0100, 0.0281, -0.1440],
    [0.7792, 0.0264, 1.1221, 0.9683, -0.1658, 0.5213, 0.0255, 0.0108, 0.0048, 0.0304, -0.1531],
    [0.7967, -0.0370, 1.0960, 0.8265, -0.2074, 0.5338, 0.0088, 0.0053, -0.0025, 0.0296, -0.1588],
    [0.7939, -0.0568, 1.0338, 0.7568, -0.2468, 0.5246, -0.0050, 0.0003, -0.0073, 0.0273, -0.1588],
    [0.7778, -0.0468, 0.9489, 0.7381, -0.2847, 0.5004, -0.0170, -0.0041, -0.0122, 0.0229, -0.1566],
    [0.7613, -0.0374, 0.8658, 0.7199, -0.3237, 0.4757, -0.0292, -0.0088, -0.0161, 0.0186, -0.1576],
    [0.7445, 0.0174, 0.7810, 0.7355, -0.3625, 0.4207, -0.0379, -0.0125, -0.0205, 0.0142, -0.1613],
    [0.7262, 0.0474, 0.6911, 0.7225, -0.4010, 0.3783, -0.0432, -0.0161, -0.0246, 0.0088, -0.1391],
    [0.7129, 0.0917, 0.6037, 0.7535, -0.4389, 0.3395, -0.0417, -0.0180, -0.0274, 0.0058, -0.1265],
    [0.6956, 0.1406, 0.5914, 0.7969, -0.4442, 0.3023, -0.0385, -0.0187, -0.0295, 0.0031, -0.1182],
    [0.6894, 0.1901, 0.5475, 0.8412, -0.4541, 0.2660, -0.0352, -0.0195, -0.0320, 0.0006, -0.1123],
    [0.7097, 0.2391, 0.5765, 0.8873, -0.4491, 0.2296, -0.0268, -0.0195, -0.0343, -0.0002, -0.1088],
    [0.7492, 0.2884, 0.6584, 0.9349, -0.4355, 0.1935, -0.0147, -0.0187, -0.0375, 0.0001, -0.1077],
    [0.7823, 0.3286, 0.8095, 0.9617, -0.3964, 0.1567, 0.0023, -0.0168, -0.0397, 0.0027, -0.1048],
    [0.8159, 0.3691, 0.9651, 0.9865, -0.3586, 0.1210, 0.0199, -0.0150, -0.0423, 0.0049, -0.1032],
    [0.8498, 0.4107, 1.1229, 1.0102, -0.3236, 0.0863, 0.0374, -0.0132, -0.0445, 0.0074, -0.1032],
    [0.8389, 0.4506, 1.2643, 1.0274, -0.2661, 0.0526, 0.0564, -0.0109, -0.0460, 0.0092, -0.1037],
    [0.8038, 0.4906, 1.3842, 1.0416, -0.2047, 0.0201, 0.0757, -0.0092, -0.0469, 0.0107, -0.1052],
    [0.7220, 0.5311, 1.4334, 1.0535, -0.1350, -0.0113, 0.0937, -0.0089, -0.0484, 0.0101, -0.1079],
    [0.6373, 0.5725, 1.4808, 1.0641, -0.0684, -0.0417, 0.1117, -0.0083, -0.0500, 0.0094, -0.1120],
    [0.5516, 0.6149, 1.5272, 1.0740, -0.0045, -0.0713, 0.1289, -0.0078, -0.0522, 0.0090, -0.1177],
    [0.4740, 0.6514, 1.5029, 1.0695, 0.0453, -0.0991, 0.1496, -0.0065, -0.0445, 0.0060, -0.1220],
    [0.3941, 0.6907, 1.4771, 1.0680, 0.0943, -0.1260, 0.1701, -0.0056, -0.0368, 0.0034, -0.1286],
    [0.3127, 0.7325, 1.4508, 1.0685, 0.1428, -0.1519, 0.1907, -0.0043, -0.0302, 0.0006, -0.1374],
    [0.2215, 0.7642, 1.3885, 1.0476, 0.1883, -0.1757, 0.2120, -0.0049, -0.0227, -0.0026, -0.1433],
    [0.1270, 0.7922, 1.3085, 1.0183, 0.2337, -0.1978, 0.2333, -0.0057, -0.0146, -0.0054, -0.1491],
    [0.0296, 0.8120, 1.1917, 0.9727, 0.2766, -0.2183, 0.2530, -0.0070, -0.0061, -0.0094, -0.1535],
    [-0.0495, 0.8222, 1.0676, 0.9093, 0.3109, -0.2372, 0.2713, -0.0080, 0.0037, -0.0131, -0.1564],
    [-0.1164, 0.8258, 0.9386, 0.8336, 0.3370, -0.2552, 0.2880, -0.0098, 0.0139, -0.0172, -0.1594],
    [-0.1776, 0.8342, 0.7790, 0.7664, 0.3658, -0.2764, 0.2988, -0.0106, 0.0226, -0.0207, -0.1669],
    [-0.1601, 0.8448, 0.7058, 0.7025, 0.3808, -0.2997, 0.3070, -0.0110, 0.0303, -0.0239, -0.1777],
];
// CLIP WALK_BR END

// CLIP WALK_R BEGIN
/// AN-02-S13: Walk, right, from the Pro Rifle Pack clip "walk right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_R: [[f32; 11]; 48] = [
    [0.8243, -0.0475, 0.6454, 0.9090, 0.3356, -0.2410, -1.0928, -0.0320, -0.0091, -0.0197, -0.1802],
    [0.7926, -0.0996, 0.6600, 0.8795, 0.2893, -0.2584, -1.1222, -0.0310, -0.0103, -0.0234, -0.1597],
    [0.7851, -0.0883, 0.7243, 0.9177, 0.2610, -0.2449, -1.1304, -0.0292, -0.0100, -0.0259, -0.1492],
    [0.7821, -0.0556, 0.7991, 0.9738, 0.2408, -0.2245, -1.1262, -0.0261, -0.0076, -0.0280, -0.1440],
    [0.7579, 0.0142, 0.8388, 1.0488, 0.2293, -0.1936, -1.0954, -0.0207, -0.0048, -0.0305, -0.1362],
    [0.7253, 0.0761, 0.8609, 1.1159, 0.2141, -0.1657, -1.0670, -0.0160, -0.0014, -0.0326, -0.1268],
    [0.6869, 0.1328, 0.8697, 1.1763, 0.1963, -0.1410, -1.0400, -0.0116, 0.0032, -0.0340, -0.1164],
    [0.6410, 0.1906, 0.8610, 1.2059, 0.1756, -0.1077, -1.0156, -0.0080, 0.0055, -0.0334, -0.1045],
    [0.5959, 0.2481, 0.8507, 1.2348, 0.1538, -0.0747, -0.9910, -0.0043, 0.0086, -0.0332, -0.0939],
    [0.5523, 0.3048, 0.8403, 1.2628, 0.1309, -0.0417, -0.9665, -0.0008, 0.0117, -0.0334, -0.0848],
    [0.5172, 0.3572, 0.8440, 1.2577, 0.1070, -0.0034, -0.9439, 0.0011, 0.0118, -0.0312, -0.0794],
    [0.4850, 0.4081, 0.8510, 1.2448, 0.0818, 0.0371, -0.9214, 0.0026, 0.0114, -0.0285, -0.0760],
    [0.4536, 0.4582, 0.8573, 1.2325, 0.0552, 0.0789, -0.8990, 0.0045, 0.0106, -0.0259, -0.0738],
    [0.4237, 0.5075, 0.8642, 1.2214, 0.0272, 0.1219, -0.8770, 0.0063, 0.0100, -0.0231, -0.0732],
    [0.3945, 0.5549, 0.8700, 1.2107, -0.0022, 0.1657, -0.8547, 0.0078, 0.0096, -0.0205, -0.0739],
    [0.3701, 0.5939, 0.8840, 1.1473, -0.0333, 0.2248, -0.8336, 0.0090, 0.0072, -0.0163, -0.0773],
    [0.3464, 0.6141, 0.8974, 1.0820, -0.0661, 0.2768, -0.8126, 0.0105, 0.0049, -0.0126, -0.0822],
    [0.3227, 0.6095, 0.9078, 1.0131, -0.1007, 0.3121, -0.7910, 0.0118, 0.0033, -0.0091, -0.0881],
    [0.2999, 0.6044, 0.9177, 0.9439, -0.1371, 0.3487, -0.7695, 0.0137, 0.0007, -0.0053, -0.0955],
    [0.2779, 0.5992, 0.9261, 0.8747, -0.1755, 0.3860, -0.7478, 0.0151, -0.0019, -0.0015, -0.1043],
    [0.2561, 0.6188, 0.9323, 0.3994, -0.2158, 0.6403, -0.7264, 0.0169, -0.0033, 0.0023, -0.1144],
    [0.2352, 0.6023, 0.9369, 0.4557, -0.2582, 0.6003, -0.7049, 0.0187, -0.0054, 0.0062, -0.1260],
    [0.2148, 0.5938, 0.9386, 0.5242, -0.3026, 0.5638, -0.6831, 0.0207, -0.0080, 0.0099, -0.1390],
    [0.1945, 0.5903, 0.9374, 0.5989, -0.3489, 0.5294, -0.6618, 0.0227, -0.0101, 0.0136, -0.1533],
    [0.1758, 0.5860, 0.9351, 0.6705, -0.3975, 0.4958, -0.6398, 0.0246, -0.0111, 0.0163, -0.1688],
    [0.1967, 0.5709, 0.9637, 0.7209, -0.4266, 0.4609, -0.6158, 0.0268, -0.0098, 0.0182, -0.1552],
    [0.1858, 0.5795, 0.9494, 0.7918, -0.4532, 0.4243, -0.6133, 0.0270, -0.0074, 0.0198, -0.1482],
    [0.1725, 0.5888, 0.9299, 0.8594, -0.4739, 0.3872, -0.6159, 0.0270, -0.0041, 0.0219, -0.1433],
    [0.1595, 0.5997, 0.9106, 0.9314, -0.4947, 0.3510, -0.6187, 0.0272, -0.0011, 0.0235, -0.1417],
    [0.1621, 0.5933, 0.9291, 0.9649, -0.4992, 0.3109, -0.6276, 0.0263, 0.0008, 0.0250, -0.1349],
    [0.1756, 0.5738, 0.9762, 0.9711, -0.4924, 0.2692, -0.6401, 0.0240, 0.0025, 0.0259, -0.1247],
    [0.2256, 0.5400, 1.0516, 0.9493, -0.4523, 0.2271, -0.6535, 0.0205, 0.0037, 0.0264, -0.1110],
    [0.2757, 0.5049, 1.1282, 0.9258, -0.4117, 0.1867, -0.6673, 0.0172, 0.0046, 0.0269, -0.0989],
    [0.3256, 0.4687, 1.2046, 0.9019, -0.3709, 0.1481, -0.6806, 0.0136, 0.0056, 0.0270, -0.0887],
    [0.3866, 0.4341, 1.2458, 0.8823, -0.3115, 0.1111, -0.6947, 0.0096, 0.0055, 0.0272, -0.0810],
    [0.4494, 0.3994, 1.2677, 0.8641, -0.2487, 0.0756, -0.7090, 0.0051, 0.0048, 0.0272, -0.0751],
    [0.5035, 0.3639, 1.2491, 0.8457, -0.1876, 0.0413, -0.7232, 0.0001, 0.0033, 0.0278, -0.0709],
    [0.5583, 0.3320, 1.2214, 0.8370, -0.1280, 0.0082, -0.7377, -0.0047, 0.0027, 0.0279, -0.0698],
    [0.6140, 0.3020, 1.1871, 0.8345, -0.0696, -0.0241, -0.7519, -0.0093, 0.0026, 0.0282, -0.0712],
    [0.6754, 0.2753, 1.1506, 0.8421, -0.0116, -0.0559, -0.7658, -0.0135, 0.0013, 0.0285, -0.0757],
    [0.7366, 0.2484, 1.1178, 0.8503, 0.0473, -0.0858, -0.7896, -0.0174, 0.0000, 0.0269, -0.0817],
    [0.7957, 0.2226, 1.0882, 0.8644, 0.1078, -0.1123, -0.8275, -0.0203, -0.0010, 0.0206, -0.0900],
    [0.8355, 0.1894, 1.0336, 0.8667, 0.1577, -0.1362, -0.8669, -0.0231, -0.0020, 0.0144, -0.0976],
    [0.8650, 0.1531, 0.9670, 0.8673, 0.2034, -0.1578, -0.9070, -0.0252, -0.0023, 0.0076, -0.1063],
    [0.8761, 0.1128, 0.8829, 0.8636, 0.2376, -0.1770, -0.9469, -0.0272, -0.0033, 0.0010, -0.1156],
    [0.8743, 0.0751, 0.8011, 0.8723, 0.2659, -0.1948, -0.9859, -0.0288, -0.0054, -0.0049, -0.1286],
    [0.8639, 0.0375, 0.7209, 0.8872, 0.2901, -0.2110, -1.0242, -0.0303, -0.0072, -0.0104, -0.1444],
    [0.8368, 0.0003, 0.6005, 0.9099, 0.3871, -0.2271, -1.0597, -0.0309, -0.0084, -0.0152, -0.1636],
];
// CLIP WALK_R END

// CLIP WALK_FR BEGIN
/// AN-02-S13: Walk, forward right, from the Pro Rifle Pack clip "walk forward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const WALK_FR: [[f32; 11]; 48] = [
    [0.9656, -0.3101, 0.8354, 0.7366, 0.0837, -0.0532, -0.7537, 0.0005, 0.0253, -0.0207, -0.1867],
    [0.9657, -0.3429, 0.9249, 0.7137, 0.0876, -0.0608, -0.7435, -0.0008, 0.0279, -0.0219, -0.1841],
    [0.9619, -0.3491, 1.0077, 0.7187, 0.0905, -0.0693, -0.7333, -0.0027, 0.0290, -0.0220, -0.1830],
    [0.9363, -0.2662, 1.0544, 0.8525, 0.0920, -0.0700, -0.7222, -0.0035, 0.0263, -0.0196, -0.1766],
    [0.9106, -0.1819, 1.1011, 0.9897, 0.0928, -0.0717, -0.7106, -0.0042, 0.0234, -0.0173, -0.1723],
    [0.8848, -0.0978, 1.1479, 1.1263, 0.0925, -0.0739, -0.6992, -0.0047, 0.0214, -0.0146, -0.1699],
    [0.8396, 0.0539, 1.1607, 1.2385, 0.0909, -0.0698, -0.6855, -0.0045, 0.0148, -0.0105, -0.1617],
    [0.7921, 0.2102, 1.1683, 1.3294, 0.0881, -0.0624, -0.6715, -0.0042, 0.0094, -0.0058, -0.1541],
    [0.7409, 0.3740, 1.1677, 1.3886, 0.0842, -0.0506, -0.6563, -0.0037, 0.0045, -0.0008, -0.1464],
    [0.6865, 0.5170, 1.1595, 1.3856, 0.0787, -0.0372, -0.6415, -0.0032, 0.0006, 0.0045, -0.1386],
    [0.6319, 0.6436, 1.1491, 1.3528, 0.0720, -0.0233, -0.6267, -0.0022, -0.0017, 0.0091, -0.1321],
    [0.5766, 0.7274, 1.1357, 1.2681, 0.0641, -0.0081, -0.6116, -0.0019, -0.0026, 0.0122, -0.1266],
    [0.5228, 0.7814, 1.1240, 1.1743, 0.0551, 0.0059, -0.5974, -0.0015, -0.0021, 0.0142, -0.1230],
    [0.4700, 0.8152, 1.1132, 1.0751, 0.0448, 0.0220, -0.5832, -0.0017, -0.0015, 0.0155, -0.1212],
    [0.4135, 0.8392, 1.0938, 1.0226, 0.0336, 0.0303, -0.5732, -0.0024, -0.0006, 0.0148, -0.1193],
    [0.3578, 0.8631, 1.0753, 0.9702, 0.0216, 0.0404, -0.5637, -0.0033, 0.0010, 0.0136, -0.1192],
    [0.3035, 0.8869, 1.0590, 0.9184, 0.0088, 0.0526, -0.5540, -0.0044, 0.0020, 0.0128, -0.1212],
    [0.2501, 0.9099, 1.0445, 0.8659, -0.0052, 0.0671, -0.5443, -0.0057, 0.0035, 0.0117, -0.1253],
    [0.1964, 0.9274, 1.0296, 0.8174, -0.0199, 0.0813, -0.5358, -0.0068, 0.0048, 0.0106, -0.1310],
    [0.1383, 0.9240, 1.0056, 0.7820, -0.0338, 0.0863, -0.5331, -0.0082, 0.0049, 0.0081, -0.1368],
    [0.0905, 0.9206, 1.0149, 0.7909, -0.0458, 0.0800, -0.5376, -0.0088, 0.0030, 0.0056, -0.1516],
    [0.0442, 0.9414, 1.0367, 0.6769, -0.0558, 0.1001, -0.5476, -0.0088, 0.0001, 0.0037, -0.1715],
    [-0.0084, 0.9343, 1.0461, 0.7509, -0.0657, 0.0795, -0.5569, -0.0085, -0.0025, 0.0015, -0.1788],
    [-0.0668, 0.9278, 1.0435, 0.8252, -0.0748, 0.0604, -0.5669, -0.0084, -0.0056, -0.0004, -0.1714],
    [0.1856, 0.9201, 1.2470, 0.8972, -0.0438, 0.0431, -0.5764, -0.0080, -0.0081, -0.0028, -0.1663],
    [0.2143, 0.9105, 1.3110, 0.9663, -0.0412, 0.0270, -0.5862, -0.0078, -0.0116, -0.0050, -0.1632],
    [0.2410, 0.8985, 1.3716, 1.0321, -0.0391, 0.0122, -0.5956, -0.0079, -0.0146, -0.0074, -0.1618],
    [0.2676, 0.8841, 1.4310, 1.0949, -0.0369, -0.0014, -0.6047, -0.0072, -0.0176, -0.0092, -0.1620],
    [0.3211, 0.8455, 1.4563, 1.1152, -0.0285, -0.0158, -0.6195, -0.0058, -0.0191, -0.0088, -0.1550],
    [0.3924, 0.7917, 1.4579, 1.1076, -0.0167, -0.0287, -0.6361, -0.0036, -0.0188, -0.0063, -0.1437],
    [0.4643, 0.7380, 1.4596, 1.0992, -0.0057, -0.0397, -0.6530, -0.0015, -0.0189, -0.0041, -0.1341],
    [0.5358, 0.6850, 1.4607, 1.0908, 0.0049, -0.0490, -0.6702, 0.0001, -0.0193, -0.0014, -0.1261],
    [0.6074, 0.6329, 1.4623, 1.0835, 0.0150, -0.0564, -0.6872, 0.0022, -0.0185, 0.0012, -0.1200],
    [0.6793, 0.5708, 1.4042, 1.0552, 0.0222, -0.0617, -0.7030, 0.0044, -0.0178, 0.0041, -0.1112],
    [0.7437, 0.5081, 1.3290, 1.0240, 0.0256, -0.0655, -0.7180, 0.0062, -0.0158, 0.0073, -0.1035],
    [0.7782, 0.4507, 1.2446, 1.0030, 0.0215, -0.0681, -0.7296, 0.0082, -0.0142, 0.0087, -0.0995],
    [0.8123, 0.3937, 1.1593, 0.9821, 0.0170, -0.0698, -0.7414, 0.0100, -0.0131, 0.0099, -0.0971],
    [0.8463, 0.3371, 1.0737, 0.9614, 0.0122, -0.0704, -0.7534, 0.0118, -0.0122, 0.0113, -0.0963],
    [0.8807, 0.2806, 0.9888, 0.9405, 0.0073, -0.0701, -0.7652, 0.0134, -0.0114, 0.0124, -0.0971],
    [0.9081, 0.2235, 0.9217, 0.9182, 0.0020, -0.0695, -0.7739, 0.0145, -0.0092, 0.0120, -0.0993],
    [0.9252, 0.1658, 0.8786, 0.8954, -0.0042, -0.0694, -0.7781, 0.0137, -0.0065, 0.0094, -0.1031],
    [0.9426, 0.1083, 0.8364, 0.8734, -0.0107, -0.0690, -0.7822, 0.0132, -0.0041, 0.0067, -0.1088],
    [0.9566, 0.0507, 0.8000, 0.8520, -0.0147, -0.0690, -0.7849, 0.0121, -0.0009, 0.0038, -0.1164],
    [0.9564, -0.0089, 0.7853, 0.8276, -0.0054, -0.0710, -0.7822, 0.0105, 0.0038, -0.0005, -0.1256],
    [0.9564, -0.0683, 0.7711, 0.8052, 0.0041, -0.0739, -0.7793, 0.0086, 0.0082, -0.0053, -0.1372],
    [0.9774, -0.1283, 0.5771, 0.7834, 0.0744, -0.0774, -0.7761, 0.0069, 0.0131, -0.0097, -0.1510],
    [0.9685, -0.1872, 0.6572, 0.7683, 0.0776, -0.0844, -0.7689, 0.0049, 0.0174, -0.0133, -0.1684],
    [0.9685, -0.2579, 0.7496, 0.7484, 0.0811, -0.0563, -0.7611, 0.0027, 0.0218, -0.0168, -0.1933],
];
// CLIP WALK_FR END

// CLIP RUN_F BEGIN
/// AN-02-S13: Run, forward, from the Pro Rifle Pack clip "run forward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_F: [[f32; 11]; 48] = [
    [0.6549, 0.1532, 0.4018, 1.2643, -0.2920, 0.2879, -0.5944, 0.0607, 0.0213, -0.0775, -0.1319],
    [0.6739, 0.2250, 0.5360, 1.3333, -0.2622, 0.2789, -0.6050, 0.0551, 0.0074, -0.0716, -0.1222],
    [0.6947, 0.2968, 0.6699, 1.4017, -0.2336, 0.2695, -0.6151, 0.0491, -0.0065, -0.0658, -0.1187],
    [0.7160, 0.3688, 0.8032, 1.4704, -0.2055, 0.2601, -0.6254, 0.0432, -0.0197, -0.0603, -0.1209],
    [0.7385, 0.4409, 0.9383, 1.5390, -0.1775, 0.2504, -0.6361, 0.0373, -0.0332, -0.0547, -0.1289],
    [0.7613, 0.5126, 1.0746, 1.6074, -0.1485, 0.2404, -0.6461, 0.0309, -0.0466, -0.0494, -0.1424],
    [0.7682, 0.5792, 1.1827, 1.6572, -0.1163, 0.2269, -0.6546, 0.0245, -0.0595, -0.0428, -0.1546],
    [0.7146, 0.6265, 1.1747, 1.6318, -0.0778, 0.1980, -0.6569, 0.0169, -0.0720, -0.0325, -0.1440],
    [0.6608, 0.6735, 1.1643, 1.6051, -0.0402, 0.1685, -0.6592, 0.0093, -0.0845, -0.0225, -0.1355],
    [0.6083, 0.7206, 1.1553, 1.5783, -0.0032, 0.1384, -0.6616, 0.0017, -0.0974, -0.0125, -0.1301],
    [0.5572, 0.7676, 1.1480, 1.5514, 0.0335, 0.1077, -0.6640, -0.0055, -0.1097, -0.0029, -0.1275],
    [0.5075, 0.8144, 1.1426, 1.5248, 0.0704, 0.0766, -0.6662, -0.0135, -0.1219, 0.0072, -0.1279],
    [0.4590, 0.8608, 1.1396, 1.4976, 0.1078, 0.0448, -0.6688, -0.0207, -0.1348, 0.0167, -0.1314],
    [0.3909, 0.8559, 1.0897, 1.4010, 0.1431, 0.0060, -0.6643, -0.0255, -0.1402, 0.0267, -0.1273],
    [0.3173, 0.8385, 1.0252, 1.2842, 0.1768, -0.0356, -0.6585, -0.0290, -0.1436, 0.0368, -0.1231],
    [0.2443, 0.8219, 0.9606, 1.1671, 0.2100, -0.0773, -0.6527, -0.0328, -0.1472, 0.0470, -0.1217],
    [0.1715, 0.8065, 0.8956, 1.0498, 0.2431, -0.1181, -0.6470, -0.0365, -0.1507, 0.0571, -0.1232],
    [0.0992, 0.7923, 0.8315, 0.9334, 0.2764, -0.1577, -0.6415, -0.0403, -0.1548, 0.0672, -0.1279],
    [0.0268, 0.7799, 0.7677, 0.8191, 0.3103, -0.1954, -0.6358, -0.0440, -0.1584, 0.0773, -0.1358],
    [0.0057, 0.7534, 0.7425, 0.7081, 0.3748, -0.2234, -0.6271, -0.0459, -0.1537, 0.0854, -0.0840],
    [0.0459, 0.7050, 0.8623, 0.5994, 0.3568, -0.2400, -0.6137, -0.0454, -0.1356, 0.0889, -0.0833],
    [0.0888, 0.6569, 0.9876, 0.4907, 0.3378, -0.2552, -0.6004, -0.0452, -0.1178, 0.0924, -0.0826],
    [0.1378, 0.5978, 1.1243, 0.3277, 0.3098, -0.4085, -0.5865, -0.0449, -0.0986, 0.0940, -0.1517],
    [0.2106, 0.5887, 1.3072, 0.3943, 0.2473, -0.3629, -0.5708, -0.0460, -0.0746, 0.0868, -0.1300],
    [0.2846, 0.5952, 1.4945, 0.4920, 0.1808, -0.3230, -0.5550, -0.0478, -0.0506, 0.0790, -0.1159],
    [0.3583, 0.5951, 1.6828, 0.5778, 0.1082, -0.2861, -0.5391, -0.0498, -0.0259, 0.0707, -0.1051],
    [0.4111, 0.6254, 1.7292, 0.7260, 0.0680, -0.2578, -0.5316, -0.0457, -0.0026, 0.0617, -0.1078],
    [0.4636, 0.6549, 1.7733, 0.8743, 0.0291, -0.2316, -0.5250, -0.0415, 0.0210, 0.0523, -0.1168],
    [0.5166, 0.6828, 1.8173, 1.0229, -0.0091, -0.2067, -0.5174, -0.0375, 0.0449, 0.0424, -0.1319],
    [0.5722, 0.6703, 1.8057, 1.0988, -0.0349, -0.1796, -0.5182, -0.0294, 0.0589, 0.0338, -0.1369],
    [0.6282, 0.6479, 1.7801, 1.1579, -0.0582, -0.1517, -0.5200, -0.0199, 0.0722, 0.0257, -0.1414],
    [0.6854, 0.6253, 1.7550, 1.2200, -0.0817, -0.1236, -0.5224, -0.0100, 0.0842, 0.0176, -0.1495],
    [0.7369, 0.5761, 1.6876, 1.2299, -0.1012, -0.0930, -0.5141, -0.0087, 0.0968, 0.0091, -0.1483],
    [0.7845, 0.5100, 1.5917, 1.2043, -0.1210, -0.0630, -0.4998, -0.0130, 0.1095, 0.0001, -0.1417],
    [0.8323, 0.4444, 1.4963, 1.1803, -0.1432, -0.0351, -0.4854, -0.0184, 0.1220, -0.0092, -0.1383],
    [0.8653, 0.3676, 1.3913, 1.1281, -0.1650, -0.0084, -0.4743, -0.0162, 0.1358, -0.0170, -0.1314],
    [0.9474, 0.2740, 0.9682, 1.0302, -0.2963, 0.0177, -0.4695, -0.0031, 0.1526, -0.0230, -0.1180],
    [0.9167, 0.1823, 1.0082, 0.9340, -0.2598, 0.0425, -0.4647, 0.0105, 0.1696, -0.0283, -0.1087],
    [0.8809, 0.0930, 1.0412, 0.8410, -0.2264, 0.0673, -0.4622, 0.0226, 0.1825, -0.0347, -0.1037],
    [0.8153, 0.0103, 1.0279, 0.7573, -0.1956, 0.0958, -0.4704, 0.0300, 0.1801, -0.0410, -0.1034],
    [0.7481, -0.0717, 1.0078, 0.6742, -0.1660, 0.1256, -0.4786, 0.0373, 0.1766, -0.0470, -0.1068],
    [0.7818, -0.1526, 0.6210, 0.5932, -0.2884, 0.1571, -0.4869, 0.0452, 0.1741, -0.0529, -0.1140],
    [0.7553, -0.2077, 0.5663, 0.5714, -0.3020, 0.1975, -0.5038, 0.0487, 0.1483, -0.0598, -0.0928],
    [0.7312, -0.2272, 0.5181, 0.5644, -0.3155, 0.2312, -0.5206, 0.0523, 0.1212, -0.0660, -0.0881],
    [0.7067, -0.2459, 0.4701, 0.5580, -0.3288, 0.2652, -0.5372, 0.0559, 0.0945, -0.0729, -0.0834],
    [0.6841, -0.1606, 0.4267, 0.7222, -0.3355, 0.2789, -0.5524, 0.0589, 0.0742, -0.0756, -0.0859],
    [0.7014, -0.0476, 0.2787, 0.9328, -0.3604, 0.2872, -0.5675, 0.0610, 0.0556, -0.0780, -0.1802],
    [0.6598, 0.0653, 0.3081, 1.1409, -0.3247, 0.2924, -0.5824, 0.0634, 0.0375, -0.0799, -0.1513],
];
// CLIP RUN_F END

// CLIP RUN_FL BEGIN
/// AN-02-S13: Run, forward left, from the Pro Rifle Pack clip "run forward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_FL: [[f32; 11]; 48] = [
    [0.5706, 0.1627, 0.6009, 1.2460, -0.2733, 0.4125, 0.0277, -0.0221, 0.1566, -0.0247, -0.0969],
    [0.5650, 0.2026, 0.6557, 1.3615, -0.2403, 0.4126, 0.0277, -0.0223, 0.1478, -0.0256, -0.0902],
    [0.5678, 0.2462, 0.7253, 1.4825, -0.2078, 0.4036, 0.0309, -0.0209, 0.1377, -0.0258, -0.0880],
    [0.5765, 0.2895, 0.8075, 1.6051, -0.1763, 0.3921, 0.0340, -0.0193, 0.1268, -0.0262, -0.0903],
    [0.5757, 0.3365, 0.8679, 1.7152, -0.1428, 0.3681, 0.0456, -0.0167, 0.1110, -0.0261, -0.0917],
    [0.5722, 0.3857, 0.9230, 1.8163, -0.1090, 0.3372, 0.0626, -0.0136, 0.0925, -0.0261, -0.0944],
    [0.5697, 0.4344, 0.9837, 1.9161, -0.0761, 0.3049, 0.0798, -0.0104, 0.0732, -0.0262, -0.1003],
    [0.5514, 0.4964, 1.0146, 1.9831, -0.0432, 0.2637, 0.0998, -0.0064, 0.0518, -0.0265, -0.1025],
    [0.5073, 0.5807, 0.9936, 2.0030, -0.0111, 0.2099, 0.1239, -0.0018, 0.0264, -0.0256, -0.0964],
    [0.4640, 0.6663, 0.9763, 2.0227, 0.0187, 0.1559, 0.1479, 0.0026, 0.0010, -0.0254, -0.0927],
    [0.4198, 0.7400, 0.9594, 2.0241, 0.0465, 0.1072, 0.1723, 0.0070, -0.0235, -0.0247, -0.0906],
    [0.3677, 0.7651, 0.9274, 1.9550, 0.0721, 0.0780, 0.1967, 0.0120, -0.0450, -0.0204, -0.0874],
    [0.3139, 0.7909, 0.8931, 1.8855, 0.0956, 0.0456, 0.2211, 0.0171, -0.0666, -0.0168, -0.0853],
    [0.2588, 0.8180, 0.8576, 1.8159, 0.1173, 0.0108, 0.2451, 0.0223, -0.0881, -0.0132, -0.0847],
    [0.2028, 0.8464, 0.8217, 1.7457, 0.1370, -0.0241, 0.2695, 0.0278, -0.1099, -0.0097, -0.0857],
    [0.1458, 0.8763, 0.7851, 1.6757, 0.1553, -0.0585, 0.2933, 0.0327, -0.1319, -0.0063, -0.0882],
    [0.0874, 0.9075, 0.7473, 1.6056, 0.1720, -0.0916, 0.3168, 0.0381, -0.1531, -0.0032, -0.0921],
    [0.0285, 0.9390, 0.7100, 1.5339, 0.1872, -0.1225, 0.3405, 0.0431, -0.1751, -0.0004, -0.0978],
    [-0.0318, 0.9715, 0.6717, 1.4630, 0.2010, -0.1505, 0.3641, 0.0486, -0.1966, 0.0024, -0.1052],
    [-0.1279, 1.0047, 0.6185, 1.3925, 0.2253, -0.1758, 0.3873, 0.0537, -0.2183, 0.0052, -0.0608],
    [-0.1296, 0.9812, 0.6924, 1.2713, 0.2370, -0.1813, 0.4066, 0.0536, -0.2238, 0.0070, -0.0578],
    [-0.0919, 0.9194, 0.8488, 1.1142, 0.2438, -0.1748, 0.4233, 0.0497, -0.2192, 0.0095, -0.0563],
    [-0.0533, 0.8565, 1.0074, 0.9553, 0.2496, -0.1677, 0.4398, 0.0465, -0.2141, 0.0115, -0.0548],
    [-0.0112, 0.7882, 1.1471, 0.8167, 0.2493, -0.1583, 0.4504, 0.0395, -0.2024, 0.0144, -0.0553],
    [0.0364, 0.7123, 1.2608, 0.7073, 0.2413, -0.1462, 0.4532, 0.0277, -0.1802, 0.0184, -0.0588],
    [0.0838, 0.6009, 1.3746, 0.6671, 0.2340, -0.1324, 0.4558, 0.0157, -0.1572, 0.0218, -0.0814],
    [0.1367, 0.5285, 1.4829, 0.6116, 0.2220, -0.1150, 0.4538, 0.0037, -0.1325, 0.0251, -0.0640],
    [0.2111, 0.5288, 1.5723, 0.6974, 0.1884, -0.1039, 0.4368, -0.0097, -0.1039, 0.0273, -0.0671],
    [0.2852, 0.5313, 1.6624, 0.7877, 0.1537, -0.0918, 0.4190, -0.0239, -0.0737, 0.0296, -0.0736],
    [0.3579, 0.5366, 1.7525, 0.8847, 0.1170, -0.0784, 0.4013, -0.0389, -0.0429, 0.0309, -0.0840],
    [0.4475, 0.5039, 1.8044, 0.9044, 0.0644, -0.0624, 0.3779, -0.0449, -0.0197, 0.0317, -0.0834],
    [0.5353, 0.4736, 1.8559, 0.9277, 0.0111, -0.0445, 0.3545, -0.0512, 0.0048, 0.0324, -0.0850],
    [0.6204, 0.4457, 1.9072, 0.9552, -0.0430, -0.0246, 0.3310, -0.0585, 0.0295, 0.0330, -0.0889],
    [0.6952, 0.3889, 1.8957, 0.9187, -0.0693, -0.0025, 0.3095, -0.0567, 0.0476, 0.0310, -0.0828],
    [0.7676, 0.3251, 1.8683, 0.8631, -0.0901, 0.0208, 0.2884, -0.0524, 0.0635, 0.0287, -0.0752],
    [0.8394, 0.2632, 1.8407, 0.8079, -0.1124, 0.0454, 0.2672, -0.0475, 0.0800, 0.0268, -0.0696],
    [0.8736, 0.2049, 1.7713, 0.7564, -0.1311, 0.0718, 0.2440, -0.0392, 0.0930, 0.0223, -0.0665],
    [0.8824, 0.1491, 1.6736, 0.7064, -0.1498, 0.0999, 0.2202, -0.0277, 0.1047, 0.0163, -0.0655],
    [0.8901, 0.0943, 1.5756, 0.6551, -0.1724, 0.1298, 0.1961, -0.0151, 0.1158, 0.0112, -0.0662],
    [0.8840, 0.0500, 1.4661, 0.6254, -0.1946, 0.1620, 0.1729, -0.0034, 0.1258, 0.0062, -0.0716],
    [0.8568, 0.0289, 1.3377, 0.6263, -0.2153, 0.1991, 0.1502, 0.0073, 0.1337, 0.0006, -0.0417],
    [0.8287, 0.0313, 1.2098, 0.6327, -0.2387, 0.2239, 0.1273, 0.0184, 0.1417, -0.0044, -0.0347],
    [0.7989, 0.0367, 1.0905, 0.6561, -0.2595, 0.2483, 0.1067, 0.0264, 0.1481, -0.0083, -0.0302],
    [0.7625, 0.0533, 0.9990, 0.7523, -0.2620, 0.2726, 0.0936, 0.0186, 0.1503, -0.0106, -0.0356],
    [0.7260, 0.0722, 0.9081, 0.8511, -0.2660, 0.2987, 0.0806, 0.0101, 0.1515, -0.0130, -0.0411],
    [0.6894, 0.0919, 0.8180, 0.9494, -0.2719, 0.3266, 0.0671, 0.0018, 0.1529, -0.0157, -0.0465],
    [0.6529, 0.1139, 0.7294, 1.0486, -0.2786, 0.3551, 0.0541, -0.0060, 0.1548, -0.0185, -0.0519],
    [0.6190, 0.1372, 0.6177, 1.1472, -0.3070, 0.3841, 0.0407, -0.0138, 0.1557, -0.0215, -0.1154],
];
// CLIP RUN_FL END

// CLIP RUN_L BEGIN
/// AN-02-S13: Run, left, from the Pro Rifle Pack clip "run left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_L: [[f32; 11]; 48] = [
    [0.2688, 0.8234, 0.9369, 0.8759, -0.1241, 0.1582, -1.1813, 0.0398, -0.1517, -0.0295, -0.1017],
    [0.3076, 0.8722, 0.9272, 0.8746, -0.1039, 0.1789, -1.1872, 0.0380, -0.1517, -0.0381, -0.0937],
    [0.3411, 0.8797, 0.8961, 0.8128, -0.0860, 0.2075, -1.1733, 0.0333, -0.1517, -0.0465, -0.0834],
    [0.3746, 0.8856, 0.8679, 0.7493, -0.0675, 0.2383, -1.1596, 0.0278, -0.1517, -0.0539, -0.0757],
    [0.4079, 0.7140, 0.8418, 1.2715, -0.0483, 0.0553, -1.1453, 0.0224, -0.1517, -0.0615, -0.0702],
    [0.4275, 0.6606, 0.7852, 1.2853, -0.0273, 0.0254, -1.1193, 0.0186, -0.1356, -0.0608, -0.0618],
    [0.4427, 0.6005, 0.7228, 1.2757, -0.0042, -0.0063, -1.0900, 0.0150, -0.0961, -0.0583, -0.0551],
    [0.4576, 0.5405, 0.6639, 1.2658, 0.0211, -0.0387, -1.0600, 0.0113, -0.0563, -0.0563, -0.0513],
    [0.4724, 0.4807, 0.6087, 1.2551, 0.0487, -0.0722, -1.0306, 0.0084, -0.0162, -0.0541, -0.0505],
    [0.4871, 0.4218, 0.5571, 1.2442, 0.0789, -0.1060, -1.0007, 0.0052, 0.0236, -0.0525, -0.0524],
    [0.5021, 0.3639, 0.5092, 1.2331, 0.1117, -0.1411, -0.9710, 0.0024, 0.0647, -0.0508, -0.0570],
    [0.5302, 0.3147, 0.4909, 1.2076, 0.1484, -0.1740, -0.9397, 0.0000, 0.0975, -0.0467, -0.0670],
    [0.5996, 0.2772, 0.4663, 1.1604, 0.1954, -0.2043, -0.9065, -0.0012, 0.1177, -0.0379, -0.0512],
    [0.6410, 0.2401, 0.4965, 1.1116, 0.2332, -0.2351, -0.8736, -0.0024, 0.1375, -0.0291, -0.0474],
    [0.6766, 0.2076, 0.5313, 1.0622, 0.2643, -0.2633, -0.8427, -0.0041, 0.1540, -0.0210, -0.0452],
    [0.6942, 0.1913, 0.5899, 1.0103, 0.2676, -0.2772, -0.8204, -0.0076, 0.1589, -0.0162, -0.0491],
    [0.7120, 0.1753, 0.6480, 0.9588, 0.2692, -0.2902, -0.7984, -0.0109, 0.1632, -0.0112, -0.0529],
    [0.7298, 0.1587, 0.7060, 0.9065, 0.2694, -0.3027, -0.7761, -0.0143, 0.1677, -0.0063, -0.0567],
    [0.7478, 0.1423, 0.7633, 0.8548, 0.2685, -0.3143, -0.7538, -0.0178, 0.1721, -0.0014, -0.0605],
    [0.7666, 0.1256, 0.8203, 0.8033, 0.2661, -0.3251, -0.7320, -0.0208, 0.1770, 0.0041, -0.0643],
    [0.7856, 0.1094, 0.8763, 0.7531, 0.2624, -0.3351, -0.7098, -0.0239, 0.1813, 0.0091, -0.0681],
    [0.7930, 0.1903, 0.9502, 0.7838, 0.2158, -0.2877, -0.7151, -0.0250, 0.1697, 0.0124, -0.1049],
    [0.7976, 0.2341, 1.0293, 0.8177, 0.1583, -0.2497, -0.7288, -0.0260, 0.1537, 0.0154, -0.0985],
    [0.8016, 0.2823, 1.1081, 0.8588, 0.1012, -0.2135, -0.7408, -0.0266, 0.1380, 0.0185, -0.0954],
    [0.8057, 0.3243, 1.1873, 0.8830, 0.0442, -0.1774, -0.7535, -0.0274, 0.1220, 0.0216, -0.0915],
    [0.8095, 0.3705, 1.2655, 0.9139, -0.0128, -0.1425, -0.7665, -0.0284, 0.1059, 0.0249, -0.0906],
    [0.8130, 0.4173, 1.3430, 0.9437, -0.0692, -0.1085, -0.7790, -0.0298, 0.0904, 0.0283, -0.0915],
    [0.8044, 0.4588, 1.4114, 0.9585, -0.1207, -0.0749, -0.7949, -0.0291, 0.0736, 0.0313, -0.0915],
    [0.7773, 0.4922, 1.4664, 0.9529, -0.1640, -0.0421, -0.8145, -0.0272, 0.0556, 0.0341, -0.0897],
    [0.7498, 0.5319, 1.5211, 0.9574, -0.2062, -0.0110, -0.8348, -0.0253, 0.0369, 0.0370, -0.0915],
    [0.7160, 0.5604, 1.5659, 0.9363, -0.2441, 0.0188, -0.8549, -0.0232, 0.0208, 0.0400, -0.0906],
    [0.6577, 0.5621, 1.5705, 0.8596, -0.2723, 0.0462, -0.8765, -0.0208, 0.0089, 0.0428, -0.0819],
    [0.5996, 0.5650, 1.5749, 0.7848, -0.3005, 0.0716, -0.8978, -0.0187, -0.0015, 0.0464, -0.0764],
    [0.5422, 0.5702, 1.5789, 0.7134, -0.3287, 0.0953, -0.9193, -0.0167, -0.0126, 0.0496, -0.0740],
    [0.4877, 0.5664, 1.5130, 0.6232, -0.3290, 0.1169, -0.9412, -0.0138, -0.0247, 0.0502, -0.0717],
    [0.4337, 0.5605, 1.4465, 0.5272, -0.3300, 0.1370, -0.9628, -0.0115, -0.0372, 0.0510, -0.0721],
    [0.3797, 0.5528, 1.3794, 0.4259, -0.3316, 0.1561, -0.9841, -0.0088, -0.0487, 0.0514, -0.0755],
    [0.3304, 0.5802, 1.3049, 0.3909, -0.3188, 0.1759, -1.0047, -0.0055, -0.0654, 0.0476, -0.0878],
    [0.2840, 0.6174, 1.2290, 0.3723, -0.3044, 0.1956, -1.0250, -0.0017, -0.0829, 0.0427, -0.1038],
    [0.2384, 0.6458, 1.1518, 0.3775, -0.2914, 0.2055, -1.0449, 0.0020, -0.1003, 0.0382, -0.0446],
    [0.2164, 0.6608, 1.0857, 0.4012, -0.2764, 0.2022, -1.0641, 0.0083, -0.1168, 0.0310, -0.0455],
    [0.2099, 0.6652, 1.0275, 0.4408, -0.2595, 0.1946, -1.0831, 0.0163, -0.1321, 0.0222, -0.0497],
    [0.2036, 0.6701, 0.9690, 0.4812, -0.2426, 0.1873, -1.1019, 0.0249, -0.1481, 0.0139, -0.0539],
    [0.2034, 0.5539, 0.9352, 0.8153, -0.2273, 0.0667, -1.1199, 0.0316, -0.1517, 0.0062, -0.0773],
    [0.2116, 0.5962, 0.9380, 0.8074, -0.2146, 0.0839, -1.1379, 0.0359, -0.1517, 0.0007, -0.0836],
    [0.2205, 0.6650, 0.9417, 0.8485, -0.2016, 0.1012, -1.1553, 0.0405, -0.1517, -0.0054, -0.0998],
    [0.2296, 0.7181, 0.9434, 0.8575, -0.1863, 0.1180, -1.1702, 0.0439, -0.1517, -0.0114, -0.1127],
    [0.2280, 0.7719, 0.9429, 0.8697, -0.1452, 0.1377, -1.1760, 0.0418, -0.1517, -0.0205, -0.1109],
];
// CLIP RUN_L END

// CLIP RUN_BL BEGIN
/// AN-02-S13: Run, backward left, from the Pro Rifle Pack clip "run backward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_BL: [[f32; 11]; 48] = [
    [0.1279, 0.9093, 0.8379, 1.4444, 0.1940, -0.0314, -1.0953, 0.0066, -0.1682, 0.0099, -0.1088],
    [0.2025, 0.8838, 0.8916, 1.4853, 0.1776, -0.0130, -1.0983, 0.0072, -0.1563, 0.0134, -0.1065],
    [0.2598, 0.8364, 0.9040, 1.5030, 0.1578, 0.0015, -1.0897, 0.0046, -0.1299, 0.0153, -0.0991],
    [0.3106, 0.7738, 0.9009, 1.5039, 0.1383, 0.0135, -1.0739, -0.0000, -0.0927, 0.0153, -0.0912],
    [0.3648, 0.7111, 0.9053, 1.5049, 0.1213, 0.0253, -1.0577, -0.0049, -0.0557, 0.0155, -0.0870],
    [0.4097, 0.6483, 0.8886, 1.5055, 0.1059, 0.0373, -1.0414, -0.0096, -0.0179, 0.0159, -0.0814],
    [0.4606, 0.5855, 0.8843, 1.5059, 0.0929, 0.0498, -1.0253, -0.0141, 0.0194, 0.0165, -0.0804],
    [0.5127, 0.5232, 0.8820, 1.5068, 0.0819, 0.0622, -1.0094, -0.0188, 0.0570, 0.0164, -0.0820],
    [0.5624, 0.4632, 0.8730, 1.4960, 0.0732, 0.0704, -0.9900, -0.0227, 0.0917, 0.0161, -0.0848],
    [0.5923, 0.4110, 0.8226, 1.4403, 0.0687, 0.0609, -0.9578, -0.0242, 0.1192, 0.0123, -0.0833],
    [0.6216, 0.3580, 0.7711, 1.3832, 0.0680, 0.0505, -0.9258, -0.0256, 0.1465, 0.0086, -0.0846],
    [0.6502, 0.3057, 0.7184, 1.3261, 0.0710, 0.0393, -0.8938, -0.0272, 0.1730, 0.0052, -0.0886],
    [0.6777, 0.2538, 0.6635, 1.2692, 0.0779, 0.0275, -0.8618, -0.0292, 0.2005, 0.0018, -0.0952],
    [0.7051, 0.2022, 0.6079, 1.2114, 0.0890, 0.0149, -0.8296, -0.0313, 0.2278, -0.0014, -0.1050],
    [0.7280, 0.1516, 0.5603, 1.1540, 0.1085, 0.0017, -0.7976, -0.0340, 0.2550, -0.0049, -0.0810],
    [0.7681, 0.1130, 0.6247, 1.0885, 0.1195, -0.0188, -0.7611, -0.0297, 0.2640, -0.0030, -0.0836],
    [0.8135, 0.0776, 0.7218, 1.0197, 0.1316, -0.0416, -0.7230, -0.0242, 0.2676, -0.0004, -0.0878],
    [0.8591, 0.0431, 0.8196, 0.9494, 0.1449, -0.0644, -0.6847, -0.0179, 0.2705, 0.0019, -0.0920],
    [0.8945, -0.0130, 0.9181, 0.9021, 0.1472, -0.1342, -0.6661, -0.0135, 0.2625, 0.0065, -0.1530],
    [0.9223, 0.0681, 1.0156, 0.9650, 0.1433, -0.1289, -0.6597, -0.0105, 0.2459, 0.0128, -0.1481],
    [0.9505, 0.1489, 1.1143, 1.0270, 0.1397, -0.1231, -0.6539, -0.0071, 0.2295, 0.0190, -0.1458],
    [0.9604, 0.2262, 1.2162, 1.0776, 0.1260, -0.1132, -0.6604, -0.0034, 0.2094, 0.0249, -0.1433],
    [0.9434, 0.3005, 1.3255, 1.1156, 0.0973, -0.0993, -0.6874, 0.0006, 0.1835, 0.0309, -0.1403],
    [0.9256, 0.3754, 1.4339, 1.1530, 0.0711, -0.0892, -0.7139, 0.0045, 0.1579, 0.0369, -0.1400],
    [0.8952, 0.4473, 1.5292, 1.1808, 0.0458, -0.0824, -0.7390, 0.0075, 0.1312, 0.0405, -0.1403],
    [0.8127, 0.5054, 1.5724, 1.1766, 0.0199, -0.0783, -0.7579, 0.0096, 0.1013, 0.0351, -0.1361],
    [0.7293, 0.5633, 1.6151, 1.1726, -0.0060, -0.0768, -0.7767, 0.0112, 0.0715, 0.0294, -0.1346],
    [0.6463, 0.6220, 1.6580, 1.1703, -0.0320, -0.0779, -0.7954, 0.0130, 0.0416, 0.0242, -0.1359],
    [0.5370, 0.6530, 1.6216, 1.1152, -0.0464, -0.0776, -0.7963, 0.0143, 0.0109, 0.0082, -0.1280],
    [0.4268, 0.6830, 1.5844, 1.0611, -0.0602, -0.0779, -0.7973, 0.0154, -0.0202, -0.0080, -0.1230],
    [0.3172, 0.7132, 1.5473, 1.0093, -0.0736, -0.0786, -0.7983, 0.0165, -0.0512, -0.0232, -0.1210],
    [0.2120, 0.7241, 1.4530, 0.9225, -0.0717, -0.0809, -0.8057, 0.0177, -0.0735, -0.0329, -0.1153],
    [0.1070, 0.7296, 1.3437, 0.8274, -0.0659, -0.0844, -0.8138, 0.0188, -0.0930, -0.0409, -0.1115],
    [0.0019, 0.7348, 1.2338, 0.7328, -0.0597, -0.0895, -0.8223, 0.0200, -0.1134, -0.0484, -0.1115],
    [-0.0836, 0.7695, 1.1210, 0.6911, -0.0378, -0.1026, -0.8423, 0.0213, -0.1335, -0.0497, -0.1235],
    [-0.1567, 0.8102, 1.0041, 0.7078, -0.0042, -0.1423, -0.8701, 0.0224, -0.1535, -0.0467, -0.0862],
    [-0.2297, 0.8537, 0.8873, 0.7362, 0.0316, -0.1564, -0.8984, 0.0239, -0.1737, -0.0438, -0.0813],
    [-0.2754, 0.8942, 0.7935, 0.7869, 0.0727, -0.1672, -0.9285, 0.0232, -0.1908, -0.0383, -0.0786],
    [-0.2791, 0.9323, 0.7360, 0.8733, 0.1191, -0.1711, -0.9609, 0.0198, -0.2035, -0.0295, -0.0789],
    [-0.2819, 0.9699, 0.6793, 0.9604, 0.1656, -0.1734, -0.9939, 0.0166, -0.2163, -0.0203, -0.0793],
    [-0.2791, 1.0022, 0.6310, 1.0431, 0.2055, -0.1710, -1.0236, 0.0131, -0.2262, -0.0133, -0.0801],
    [-0.2574, 1.0138, 0.6148, 1.1080, 0.2206, -0.1563, -1.0415, 0.0102, -0.2262, -0.0115, -0.0828],
    [-0.2352, 1.0248, 0.6002, 1.1725, 0.2355, -0.1416, -1.0594, 0.0069, -0.2262, -0.0100, -0.0855],
    [-0.2125, 1.0355, 0.5855, 1.2363, 0.2498, -0.1272, -1.0773, 0.0035, -0.2260, -0.0088, -0.0882],
    [-0.1496, 1.0105, 0.6342, 1.2782, 0.2352, -0.1071, -1.0807, 0.0040, -0.2146, -0.0048, -0.0940],
    [-0.0851, 0.9853, 0.6852, 1.3198, 0.2211, -0.0876, -1.0845, 0.0045, -0.2027, -0.0013, -0.0998],
    [-0.0224, 0.9601, 0.7269, 1.3614, 0.2277, -0.0686, -1.0881, 0.0053, -0.1912, 0.0023, -0.1207],
    [0.0530, 0.9347, 0.7829, 1.4031, 0.2105, -0.0498, -1.0915, 0.0060, -0.1800, 0.0062, -0.1134],
];
// CLIP RUN_BL END

// CLIP RUN_B BEGIN
/// AN-02-S13: Run, backward, from the Pro Rifle Pack clip "run backward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_B: [[f32; 11]; 48] = [
    [0.1516, 0.9155, 0.7994, 1.4082, 0.2569, -0.0979, -0.5750, 0.0233, -0.1536, -0.0160, -0.1070],
    [0.2246, 0.9004, 0.8648, 1.4554, 0.2288, -0.0684, -0.5786, 0.0262, -0.1419, -0.0155, -0.1055],
    [0.2797, 0.8634, 0.8889, 1.4785, 0.1972, -0.0390, -0.5710, 0.0246, -0.1178, -0.0114, -0.0992],
    [0.3238, 0.8119, 0.8869, 1.4859, 0.1652, -0.0097, -0.5563, 0.0196, -0.0849, -0.0056, -0.0906],
    [0.3723, 0.7601, 0.8934, 1.4925, 0.1357, 0.0192, -0.5421, 0.0149, -0.0520, 0.0010, -0.0859],
    [0.4147, 0.7080, 0.8850, 1.4988, 0.1075, 0.0482, -0.5274, 0.0100, -0.0185, 0.0076, -0.0809],
    [0.4623, 0.6561, 0.8862, 1.5048, 0.0812, 0.0773, -0.5130, 0.0051, 0.0141, 0.0143, -0.0799],
    [0.5226, 0.6048, 0.9115, 1.5108, 0.0569, 0.1061, -0.4981, 0.0005, 0.0468, 0.0211, -0.0854],
    [0.5591, 0.5539, 0.8854, 1.5058, 0.0336, 0.1294, -0.4801, -0.0043, 0.0783, 0.0269, -0.0842],
    [0.5894, 0.5037, 0.8429, 1.4567, 0.0147, 0.1293, -0.4490, -0.0078, 0.1040, 0.0299, -0.0829],
    [0.6199, 0.4531, 0.7990, 1.4067, -0.0011, 0.1287, -0.4184, -0.0113, 0.1296, 0.0327, -0.0841],
    [0.6503, 0.4021, 0.7534, 1.3568, -0.0139, 0.1272, -0.3878, -0.0144, 0.1553, 0.0355, -0.0878],
    [0.6809, 0.3509, 0.7061, 1.3057, -0.0237, 0.1251, -0.3572, -0.0173, 0.1807, 0.0386, -0.0940],
    [0.7119, 0.3001, 0.6576, 1.2551, -0.0303, 0.1222, -0.3264, -0.0196, 0.2074, 0.0414, -0.1029],
    [0.7466, 0.2497, 0.5994, 1.2044, -0.0213, 0.1186, -0.2960, -0.0218, 0.2334, 0.0439, -0.0800],
    [0.7871, 0.2063, 0.6591, 1.1459, -0.0121, 0.1026, -0.2588, -0.0150, 0.2424, 0.0445, -0.0826],
    [0.8320, 0.1652, 0.7486, 1.0845, 0.0014, 0.0832, -0.2202, -0.0064, 0.2468, 0.0436, -0.0869],
    [0.8773, 0.1248, 0.8387, 1.0212, 0.0161, 0.0629, -0.1808, 0.0023, 0.2517, 0.0420, -0.0912],
    [0.9108, 0.0418, 0.9346, 0.9783, 0.0231, -0.0034, -0.1610, 0.0077, 0.2443, 0.0408, -0.1531],
    [0.9364, 0.1184, 1.0338, 1.0323, 0.0248, -0.0157, -0.1538, 0.0108, 0.2298, 0.0388, -0.1490],
    [0.9622, 0.1945, 1.1336, 1.0838, 0.0267, -0.0272, -0.1463, 0.0134, 0.2142, 0.0377, -0.1470],
    [0.9670, 0.2684, 1.2361, 1.1237, 0.0235, -0.0340, -0.1526, 0.0158, 0.1952, 0.0355, -0.1445],
    [0.9401, 0.3396, 1.3427, 1.1503, 0.0149, -0.0376, -0.1777, 0.0179, 0.1704, 0.0317, -0.1411],
    [0.9131, 0.4079, 1.4491, 1.1701, 0.0063, -0.0442, -0.2035, 0.0203, 0.1455, 0.0282, -0.1389],
    [0.8735, 0.4778, 1.5408, 1.1946, -0.0004, -0.0542, -0.2272, 0.0214, 0.1200, 0.0238, -0.1404],
    [0.7840, 0.5298, 1.5769, 1.1804, 0.0007, -0.0665, -0.2440, 0.0160, 0.0933, 0.0154, -0.1358],
    [0.6938, 0.5817, 1.6117, 1.1684, 0.0012, -0.0809, -0.2615, 0.0111, 0.0654, 0.0064, -0.1342],
    [0.6040, 0.6334, 1.6467, 1.1577, 0.0018, -0.0975, -0.2791, 0.0061, 0.0386, -0.0027, -0.1354],
    [0.4951, 0.6576, 1.6029, 1.0952, 0.0107, -0.1121, -0.2796, -0.0058, 0.0099, -0.0135, -0.1277],
    [0.3855, 0.6813, 1.5579, 1.0348, 0.0219, -0.1271, -0.2805, -0.0178, -0.0187, -0.0249, -0.1231],
    [0.2765, 0.7051, 1.5127, 0.9768, 0.0344, -0.1426, -0.2818, -0.0298, -0.0472, -0.0361, -0.1217],
    [0.1733, 0.7041, 1.4117, 0.8743, 0.0538, -0.1583, -0.2880, -0.0340, -0.0668, -0.0427, -0.1150],
    [0.0709, 0.6962, 1.2951, 0.7610, 0.0763, -0.1755, -0.2959, -0.0359, -0.0842, -0.0484, -0.1105],
    [-0.0310, 0.6875, 1.1780, 0.6477, 0.1013, -0.1945, -0.3044, -0.0377, -0.1006, -0.0538, -0.1105],
    [-0.1124, 0.7103, 1.0559, 0.5947, 0.1351, -0.2231, -0.3239, -0.0347, -0.1183, -0.0544, -0.1233],
    [-0.1796, 0.7394, 0.9297, 0.6011, 0.1760, -0.2771, -0.3517, -0.0277, -0.1361, -0.0523, -0.0862],
    [-0.2465, 0.7745, 0.8022, 0.6226, 0.2187, -0.3002, -0.3796, -0.0211, -0.1546, -0.0499, -0.0813],
    [-0.2849, 0.8114, 0.7009, 0.6728, 0.2599, -0.3160, -0.4093, -0.0134, -0.1712, -0.0457, -0.0785],
    [-0.2796, 0.8526, 0.6427, 0.7676, 0.2966, -0.3193, -0.4416, -0.0044, -0.1838, -0.0374, -0.0787],
    [-0.2732, 0.8939, 0.5856, 0.8633, 0.3325, -0.3204, -0.4744, 0.0047, -0.1971, -0.0289, -0.0790],
    [-0.2624, 0.9310, 0.5381, 0.9547, 0.3613, -0.3158, -0.5039, 0.0115, -0.2080, -0.0220, -0.0797],
    [-0.2391, 0.9504, 0.5224, 1.0267, 0.3660, -0.2955, -0.5219, 0.0111, -0.2091, -0.0201, -0.0823],
    [-0.2159, 0.9693, 0.5069, 1.0978, 0.3705, -0.2757, -0.5399, 0.0106, -0.2102, -0.0187, -0.0850],
    [-0.1923, 0.9884, 0.4921, 1.1693, 0.3748, -0.2560, -0.5578, 0.0096, -0.2116, -0.0168, -0.0876],
    [-0.1318, 0.9744, 0.5515, 1.2177, 0.3474, -0.2232, -0.5616, 0.0124, -0.2003, -0.0165, -0.0934],
    [-0.0694, 0.9601, 0.6131, 1.2658, 0.3208, -0.1908, -0.5648, 0.0152, -0.1890, -0.0166, -0.0992],
    [0.0049, 0.9453, 0.6654, 1.3132, 0.3148, -0.1594, -0.5687, 0.0179, -0.1769, -0.0166, -0.1183],
    [0.0784, 0.9307, 0.7329, 1.3611, 0.2855, -0.1283, -0.5717, 0.0205, -0.1659, -0.0166, -0.1112],
];
// CLIP RUN_B END

// CLIP RUN_BR BEGIN
/// AN-02-S13: Run, backward right, from the Pro Rifle Pack clip "run backward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_BR: [[f32; 11]; 48] = [
    [0.3399, 0.7365, 0.9678, 1.3488, 0.1005, -0.0499, 0.3436, -0.0033, -0.0777, -0.0432, -0.0995],
    [0.3872, 0.7350, 0.9816, 1.4151, 0.0805, -0.0300, 0.3521, -0.0052, -0.0683, -0.0383, -0.0980],
    [0.4304, 0.7337, 0.9853, 1.4814, 0.0612, -0.0104, 0.3606, -0.0067, -0.0589, -0.0336, -0.0960],
    [0.4773, 0.7321, 0.9964, 1.5468, 0.0428, 0.0088, 0.3688, -0.0083, -0.0496, -0.0290, -0.0971],
    [0.5177, 0.7089, 0.9916, 1.5761, 0.0252, 0.0264, 0.3807, -0.0095, -0.0370, -0.0230, -0.0966],
    [0.5616, 0.6710, 0.9924, 1.5806, 0.0088, 0.0425, 0.3946, -0.0105, -0.0227, -0.0157, -0.0988],
    [0.6083, 0.6324, 0.9978, 1.5849, -0.0066, 0.0581, 0.4082, -0.0111, -0.0083, -0.0089, -0.1035],
    [0.6515, 0.5815, 0.9949, 1.5589, -0.0212, 0.0737, 0.4210, -0.0119, 0.0049, -0.0026, -0.1081],
    [0.6848, 0.5111, 0.9727, 1.4870, -0.0352, 0.0886, 0.4317, -0.0113, 0.0171, 0.0020, -0.1106],
    [0.7182, 0.4392, 0.9499, 1.4145, -0.0482, 0.1028, 0.4430, -0.0106, 0.0285, 0.0069, -0.1147],
    [0.7515, 0.3664, 0.9266, 1.3416, -0.0608, 0.1162, 0.4537, -0.0104, 0.0399, 0.0115, -0.1203],
    [0.7848, 0.2935, 0.9022, 1.2694, -0.0725, 0.1287, 0.4649, -0.0098, 0.0517, 0.0163, -0.1274],
    [0.8297, 0.2203, 0.8487, 1.1976, -0.0734, 0.1408, 0.4758, -0.0094, 0.0632, 0.0210, -0.0851],
    [0.8614, 0.1473, 0.8236, 1.1266, -0.0801, 0.1518, 0.4873, -0.0089, 0.0745, 0.0257, -0.0804],
    [0.8647, 0.0780, 0.8378, 1.0140, -0.0790, 0.1531, 0.4950, 0.0005, 0.0821, 0.0320, -0.0748],
    [0.8680, 0.0081, 0.8526, 0.9012, -0.0779, 0.1554, 0.5027, 0.0101, 0.0900, 0.0382, -0.0692],
    [0.8713, -0.0626, 0.8674, 0.7890, -0.0767, 0.1583, 0.5109, 0.0199, 0.0971, 0.0442, -0.0637],
    [0.8791, -0.0661, 0.9222, 0.7596, -0.0741, 0.1491, 0.5123, 0.0216, 0.0990, 0.0467, -0.0649],
    [0.8883, -0.0515, 0.9880, 0.7529, -0.0717, 0.1366, 0.5120, 0.0212, 0.0990, 0.0480, -0.0679],
    [0.8972, -0.0366, 1.0534, 0.7467, -0.0698, 0.1240, 0.5118, 0.0209, 0.0989, 0.0495, -0.0709],
    [0.9066, -0.0221, 1.1194, 0.7400, -0.0684, 0.1115, 0.5118, 0.0203, 0.0991, 0.0510, -0.0739],
    [0.9159, 0.0250, 1.1852, 0.7436, -0.0678, 0.0857, 0.5116, 0.0200, 0.0992, 0.0523, -0.0957],
    [0.9248, 0.0664, 1.2502, 0.7398, -0.0682, 0.0726, 0.5109, 0.0196, 0.1000, 0.0539, -0.0849],
    [0.9226, 0.1146, 1.3169, 0.7527, -0.0649, 0.0606, 0.5089, 0.0168, 0.0987, 0.0540, -0.0780],
    [0.9030, 0.1733, 1.3862, 0.7898, -0.0565, 0.0495, 0.5037, 0.0106, 0.0940, 0.0537, -0.0763],
    [0.8839, 0.2340, 1.4562, 0.8309, -0.0480, 0.0383, 0.4983, 0.0045, 0.0904, 0.0524, -0.0769],
    [0.8556, 0.2936, 1.5155, 0.8690, -0.0385, 0.0266, 0.4927, -0.0011, 0.0849, 0.0511, -0.0786],
    [0.7943, 0.3461, 1.5366, 0.8893, -0.0258, 0.0145, 0.4839, -0.0052, 0.0767, 0.0475, -0.0790],
    [0.7323, 0.3997, 1.5572, 0.9118, -0.0129, 0.0017, 0.4743, -0.0089, 0.0682, 0.0442, -0.0812],
    [0.6707, 0.4545, 1.5775, 0.9367, -0.0003, -0.0120, 0.4657, -0.0127, 0.0594, 0.0403, -0.0855],
    [0.6042, 0.4813, 1.5474, 0.9007, 0.0238, -0.0267, 0.4495, -0.0126, 0.0395, 0.0313, -0.0805],
    [0.5378, 0.5090, 1.5169, 0.8687, 0.0487, -0.0426, 0.4336, -0.0125, 0.0195, 0.0219, -0.0780],
    [0.4716, 0.5387, 1.4858, 0.8427, 0.0745, -0.0601, 0.4175, -0.0120, -0.0007, 0.0121, -0.0781],
    [0.4109, 0.5748, 1.4324, 0.8309, 0.0946, -0.0795, 0.3995, -0.0103, -0.0207, 0.0014, -0.0823],
    [0.3517, 0.6124, 1.3723, 0.8229, 0.1144, -0.1011, 0.3809, -0.0079, -0.0421, -0.0103, -0.0887],
    [0.2925, 0.6493, 1.3115, 0.8144, 0.1349, -0.1247, 0.3626, -0.0057, -0.0624, -0.0220, -0.0967],
    [0.2432, 0.7021, 1.2489, 0.7632, 0.1524, -0.1526, 0.3463, -0.0031, -0.0797, -0.0328, -0.0673],
    [0.2000, 0.7165, 1.1845, 0.7774, 0.1667, -0.1680, 0.3317, -0.0007, -0.0952, -0.0427, -0.0645],
    [0.1569, 0.7276, 1.1201, 0.7926, 0.1803, -0.1844, 0.3171, 0.0020, -0.1103, -0.0527, -0.0618],
    [0.1235, 0.7343, 1.0588, 0.8187, 0.1891, -0.1927, 0.3065, 0.0041, -0.1212, -0.0604, -0.0606],
    [0.1047, 0.7376, 1.0025, 0.8602, 0.1906, -0.1877, 0.3012, 0.0055, -0.1262, -0.0651, -0.0617],
    [0.0863, 0.7406, 0.9462, 0.9020, 0.1912, -0.1821, 0.2954, 0.0067, -0.1308, -0.0696, -0.0628],
    [0.0791, 0.7427, 0.9031, 0.9484, 0.1879, -0.1724, 0.2925, 0.0073, -0.1333, -0.0723, -0.0645],
    [0.1188, 0.7419, 0.9136, 1.0145, 0.1723, -0.1516, 0.3012, 0.0055, -0.1242, -0.0675, -0.0685],
    [0.1592, 0.7410, 0.9248, 1.0811, 0.1573, -0.1308, 0.3095, 0.0034, -0.1149, -0.0625, -0.0726],
    [0.2004, 0.7399, 0.9263, 1.1477, 0.1666, -0.1102, 0.3180, 0.0019, -0.1058, -0.0576, -0.1137],
    [0.2476, 0.7388, 0.9426, 1.2148, 0.1436, -0.0899, 0.3261, -0.0002, -0.0961, -0.0528, -0.1078],
    [0.2940, 0.7377, 0.9561, 1.2818, 0.1215, -0.0697, 0.3351, -0.0016, -0.0873, -0.0482, -0.1030],
];
// CLIP RUN_BR END

// CLIP RUN_R BEGIN
/// AN-02-S13: Run, right, from the Pro Rifle Pack clip "run right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_R: [[f32; 11]; 48] = [
    [0.6067, 0.0522, 0.3291, 1.2348, 0.2794, -0.3758, -1.0127, 0.0239, 0.1478, -0.0077, -0.1200],
    [0.6159, 0.1294, 0.4403, 1.3323, 0.2509, -0.3401, -1.0030, 0.0211, 0.1354, -0.0100, -0.1079],
    [0.6275, 0.2065, 0.5533, 1.4293, 0.2235, -0.3052, -0.9933, 0.0182, 0.1226, -0.0121, -0.1009],
    [0.6399, 0.2838, 0.6660, 1.5260, 0.1967, -0.2711, -0.9835, 0.0149, 0.1091, -0.0146, -0.0986],
    [0.6517, 0.3610, 0.7766, 1.6215, 0.1697, -0.2376, -0.9743, 0.0117, 0.0960, -0.0171, -0.1004],
    [0.6628, 0.4377, 0.8857, 1.7161, 0.1422, -0.2046, -0.9649, 0.0082, 0.0826, -0.0201, -0.1061],
    [0.6382, 0.5086, 0.9280, 1.7244, 0.1095, -0.1703, -0.9662, 0.0092, 0.0555, -0.0220, -0.1030],
    [0.5903, 0.5765, 0.9260, 1.6766, 0.0751, -0.1316, -0.9755, 0.0130, 0.0197, -0.0231, -0.0942],
    [0.5421, 0.6447, 0.9238, 1.6284, 0.0424, -0.0904, -0.9837, 0.0174, -0.0169, -0.0244, -0.0879],
    [0.4940, 0.7128, 0.9223, 1.5811, 0.0109, -0.0461, -0.9922, 0.0220, -0.0538, -0.0254, -0.0841],
    [0.4462, 0.7804, 0.9218, 1.5335, -0.0194, 0.0004, -0.9998, 0.0270, -0.0899, -0.0269, -0.0829],
    [0.3987, 0.8473, 0.9229, 1.4864, -0.0491, 0.0488, -1.0075, 0.0319, -0.0900, -0.0295, -0.0843],
    [0.3471, 0.9050, 0.9176, 1.4326, -0.0780, 0.0934, -1.0181, 0.0359, -0.0900, -0.0301, -0.0869],
    [0.2774, 0.9322, 0.8803, 1.3524, -0.1035, 0.1146, -1.0435, 0.0359, -0.0900, -0.0219, -0.0864],
    [0.2069, 0.9590, 0.8429, 1.2722, -0.1268, 0.1377, -1.0685, 0.0353, -0.0900, -0.0135, -0.0887],
    [0.1353, 0.9860, 0.8057, 1.1922, -0.1479, 0.1620, -1.0941, 0.0350, -0.0900, -0.0054, -0.0937],
    [0.0987, 0.9635, 0.8450, 1.0920, -0.1217, 0.1628, -1.1228, 0.0291, -0.0900, 0.0010, -0.0747],
    [0.0793, 0.9409, 0.8811, 0.9914, -0.1173, 0.1627, -1.1519, 0.0229, -0.0900, 0.0073, -0.0713],
    [0.0595, 0.9188, 0.9171, 0.8920, -0.1121, 0.1619, -1.1809, 0.0168, -0.0900, 0.0141, -0.0679],
    [0.0824, 0.8800, 1.0502, 0.8156, -0.1085, 0.1542, -1.1972, 0.0133, -0.0900, 0.0176, -0.0680],
    [0.1162, 0.8367, 1.2098, 0.7451, -0.1045, 0.1436, -1.2095, 0.0107, -0.0900, 0.0210, -0.0689],
    [0.1504, 0.7924, 1.3705, 0.6723, -0.0978, 0.1319, -1.2220, 0.0078, -0.0900, 0.0240, -0.0698],
    [0.1604, 0.7776, 1.4965, 0.5545, -0.0954, 0.1090, -1.2294, 0.0029, -0.0900, 0.0247, -0.1345],
    [0.1539, 0.7319, 1.6002, 0.5700, -0.0961, 0.0850, -1.2331, -0.0032, -0.0900, 0.0239, -0.1152],
    [0.1472, 0.6849, 1.7045, 0.5809, -0.0941, 0.0626, -1.2370, -0.0090, -0.0900, 0.0229, -0.0983],
    [0.1586, 0.6625, 1.8021, 0.6376, -0.0884, 0.0427, -1.2394, -0.0157, -0.0900, 0.0205, -0.0897],
    [0.1968, 0.6813, 1.8891, 0.7727, -0.0813, 0.0251, -1.2383, -0.0239, -0.0900, 0.0163, -0.0956],
    [0.2351, 0.7004, 1.9764, 0.9090, -0.0729, 0.0070, -1.2378, -0.0318, -0.0900, 0.0120, -0.1070],
    [0.2921, 0.7094, 2.0543, 1.0292, -0.0644, -0.0116, -1.2363, -0.0387, -0.0900, 0.0077, -0.1204],
    [0.4245, 0.6815, 2.0939, 1.0830, -0.0600, -0.0300, -1.2311, -0.0441, -0.0900, 0.0039, -0.1240],
    [0.5579, 0.6529, 2.1334, 1.1374, -0.0565, -0.0494, -1.2257, -0.0494, -0.0900, -0.0003, -0.1305],
    [0.6910, 0.6233, 2.1734, 1.1923, -0.0537, -0.0700, -1.2205, -0.0544, -0.0900, -0.0048, -0.1399],
    [0.7915, 0.5529, 2.1222, 1.1635, -0.0530, -0.0895, -1.2073, -0.0540, -0.0900, -0.0061, -0.1334],
    [0.8917, 0.4845, 2.0703, 1.1355, -0.0488, -0.1105, -1.1933, -0.0535, -0.0900, -0.0081, -0.1296],
    [0.9920, 0.4187, 2.0190, 1.1105, -0.0420, -0.1332, -1.1801, -0.0528, -0.0710, -0.0098, -0.1292],
    [1.0079, 0.3103, 1.8660, 0.9808, -0.0121, -0.1540, -1.1602, -0.0456, -0.0357, -0.0063, -0.1105],
    [1.0107, 0.2033, 1.6935, 0.8476, 0.0162, -0.1768, -1.1397, -0.0372, 0.0017, -0.0014, -0.0962],
    [1.0131, 0.0996, 1.5215, 0.7172, 0.0495, -0.2021, -1.1189, -0.0285, 0.0393, 0.0028, -0.0878],
    [0.9975, 0.0333, 1.3473, 0.6720, 0.0869, -0.2330, -1.0996, -0.0211, 0.0732, 0.0071, -0.0959],
    [0.9687, -0.0108, 1.1681, 0.6785, 0.1283, -0.2694, -1.0812, -0.0134, 0.1042, 0.0114, -0.1143],
    [0.9391, -0.0386, 0.9888, 0.6688, 0.1727, -0.3302, -1.0629, -0.0063, 0.1349, 0.0153, -0.0774],
    [0.9010, -0.0490, 0.8259, 0.6944, 0.2090, -0.3499, -1.0494, 0.0009, 0.1592, 0.0181, -0.0733],
    [0.8511, -0.0540, 0.6843, 0.7378, 0.2312, -0.3629, -1.0423, 0.0075, 0.1717, 0.0195, -0.0709],
    [0.8015, -0.0583, 0.5440, 0.7815, 0.2540, -0.3766, -1.0348, 0.0142, 0.1832, 0.0207, -0.0684],
    [0.7516, -0.0597, 0.4226, 0.8352, 0.2700, -0.3898, -1.0288, 0.0199, 0.1912, 0.0201, -0.0669],
    [0.6994, -0.0486, 0.3726, 0.9330, 0.2608, -0.4003, -1.0263, 0.0225, 0.1807, 0.0118, -0.0689],
    [0.6475, -0.0366, 0.3231, 1.0345, 0.2518, -0.4079, -1.0246, 0.0245, 0.1700, 0.0034, -0.0710],
    [0.5956, -0.0241, 0.2744, 1.1376, 0.2434, -0.4126, -1.0227, 0.0268, 0.1600, -0.0055, -0.0730],
];
// CLIP RUN_R END

// CLIP RUN_FR BEGIN
/// AN-02-S13: Run, forward right, from the Pro Rifle Pack clip "run forward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const RUN_FR: [[f32; 11]; 48] = [
    [0.5216, -0.3331, 0.3141, 0.8697, 0.1148, -0.1374, -0.6353, 0.0411, 0.0927, -0.0116, -0.0693],
    [0.5184, -0.2358, 0.4209, 1.0326, 0.1030, -0.1160, -0.6418, 0.0414, 0.0738, -0.0157, -0.0613],
    [0.5177, -0.1364, 0.5319, 1.2013, 0.0925, -0.0940, -0.6495, 0.0417, 0.0550, -0.0198, -0.0586],
    [0.5162, -0.0370, 0.6417, 1.3685, 0.0835, -0.0723, -0.6565, 0.0416, 0.0362, -0.0241, -0.0605],
    [0.4816, 0.0672, 0.6870, 1.4430, 0.0745, -0.0542, -0.6650, 0.0410, 0.0197, -0.0246, -0.0579],
    [0.4382, 0.1737, 0.7153, 1.4929, 0.0666, -0.0388, -0.6737, 0.0403, 0.0037, -0.0242, -0.0558],
    [0.3950, 0.2813, 0.7449, 1.5425, 0.0599, -0.0251, -0.6829, 0.0397, -0.0116, -0.0241, -0.0565],
    [0.3519, 0.3892, 0.7760, 1.5919, 0.0544, -0.0130, -0.6917, 0.0390, -0.0273, -0.0238, -0.0602],
    [0.3084, 0.4968, 0.8083, 1.6412, 0.0501, -0.0025, -0.7000, 0.0383, -0.0430, -0.0238, -0.0667],
    [0.2643, 0.6035, 0.8416, 1.6902, 0.0469, 0.0067, -0.7090, 0.0376, -0.0588, -0.0238, -0.0763],
    [0.1959, 0.6801, 0.8214, 1.6789, 0.0446, 0.0117, -0.7192, 0.0362, -0.0738, -0.0236, -0.0798],
    [0.0924, 0.7133, 0.7184, 1.5756, 0.0436, 0.0118, -0.7320, 0.0335, -0.0870, -0.0241, -0.0737],
    [-0.0112, 0.7460, 0.6136, 1.4707, 0.0443, 0.0098, -0.7447, 0.0309, -0.1004, -0.0239, -0.0717],
    [-0.1017, 0.7755, 0.5406, 1.3716, 0.0474, 0.0054, -0.7587, 0.0279, -0.1148, -0.0228, -0.0778],
    [-0.0882, 0.7929, 0.5980, 1.2945, 0.0544, 0.0030, -0.7784, 0.0218, -0.1307, -0.0150, -0.0595],
    [-0.0889, 0.8105, 0.6584, 1.2177, 0.0496, 0.0010, -0.7984, 0.0160, -0.1471, -0.0072, -0.0556],
    [-0.0896, 0.8276, 0.7188, 1.1408, 0.0450, -0.0004, -0.8188, 0.0096, -0.1638, 0.0004, -0.0517],
    [-0.0874, 0.7955, 0.7861, 1.0379, 0.0510, -0.0125, -0.8215, 0.0032, -0.1621, 0.0020, -0.0531],
    [-0.0856, 0.7626, 0.8531, 0.9325, 0.0569, -0.0252, -0.8244, -0.0035, -0.1603, 0.0037, -0.0544],
    [-0.0836, 0.7297, 0.9208, 0.8268, 0.0625, -0.0382, -0.8269, -0.0101, -0.1582, 0.0052, -0.0557],
    [-0.0814, 0.6963, 0.9890, 0.7195, 0.0682, -0.0514, -0.8298, -0.0167, -0.1558, 0.0065, -0.0570],
    [-0.0797, 0.7174, 1.0560, 0.4629, 0.0737, -0.1159, -0.8322, -0.0233, -0.1536, 0.0083, -0.1207],
    [-0.0778, 0.6496, 1.1235, 0.4498, 0.0788, -0.1107, -0.8349, -0.0299, -0.1513, 0.0099, -0.0978],
    [-0.0706, 0.6158, 1.1957, 0.4994, 0.0782, -0.1037, -0.8300, -0.0344, -0.1401, 0.0109, -0.0840],
    [-0.0602, 0.6203, 1.2715, 0.6214, 0.0734, -0.0977, -0.8206, -0.0379, -0.1237, 0.0117, -0.0821],
    [-0.0494, 0.5934, 1.3476, 0.6834, 0.0683, -0.0924, -0.8112, -0.0412, -0.1062, 0.0124, -0.0768],
    [-0.0005, 0.5698, 1.4263, 0.7523, 0.0542, -0.0885, -0.8010, -0.0433, -0.0870, 0.0128, -0.0757],
    [0.1069, 0.5318, 1.5100, 0.7940, 0.0286, -0.0855, -0.7896, -0.0436, -0.0660, 0.0126, -0.0738],
    [0.2161, 0.4937, 1.5948, 0.8365, 0.0045, -0.0842, -0.7780, -0.0444, -0.0438, 0.0119, -0.0748],
    [0.3259, 0.4557, 1.6798, 0.8806, -0.0181, -0.0847, -0.7663, -0.0448, -0.0224, 0.0117, -0.0791],
    [0.4363, 0.4172, 1.7650, 0.9261, -0.0393, -0.0869, -0.7545, -0.0455, -0.0007, 0.0112, -0.0866],
    [0.5461, 0.3779, 1.8495, 0.9729, -0.0598, -0.0910, -0.7429, -0.0460, 0.0212, 0.0103, -0.0973],
    [0.6550, 0.3368, 1.9334, 1.0199, -0.0796, -0.0971, -0.7309, -0.0466, 0.0428, 0.0099, -0.1114],
    [0.7337, 0.2414, 1.8720, 0.9396, -0.0767, -0.1029, -0.7134, -0.0452, 0.0666, 0.0064, -0.1041],
    [0.8118, 0.1467, 1.8093, 0.8576, -0.0739, -0.1113, -0.6955, -0.0442, 0.0910, 0.0032, -0.1002],
    [0.8900, 0.0529, 1.7465, 0.7753, -0.0722, -0.1223, -0.6781, -0.0434, 0.1150, 0.0001, -0.1001],
    [0.9123, -0.0511, 1.5997, 0.6660, -0.0586, -0.1343, -0.6631, -0.0369, 0.1351, 0.0033, -0.1000],
    [0.9203, -0.1587, 1.4290, 0.5479, -0.0411, -0.1483, -0.6494, -0.0292, 0.1527, 0.0086, -0.1037],
    [0.9283, -0.2668, 1.2581, 0.4304, -0.0224, -0.1656, -0.6350, -0.0213, 0.1702, 0.0141, -0.1130],
    [0.9020, -0.2688, 1.1062, 0.4437, -0.0067, -0.1661, -0.6263, -0.0125, 0.1796, 0.0190, -0.0645],
    [0.8524, -0.2496, 0.9640, 0.5372, 0.0070, -0.1606, -0.6206, -0.0022, 0.1820, 0.0244, -0.0601],
    [0.8030, -0.2299, 0.8224, 0.6318, 0.0201, -0.1548, -0.6148, 0.0078, 0.1840, 0.0292, -0.0557],
    [0.7571, -0.2256, 0.7119, 0.7001, 0.0301, -0.1498, -0.6141, 0.0159, 0.1802, 0.0298, -0.0535],
    [0.7169, -0.2426, 0.6466, 0.7294, 0.0378, -0.1487, -0.6163, 0.0203, 0.1664, 0.0227, -0.0544],
    [0.6774, -0.2608, 0.5828, 0.7575, 0.0451, -0.1468, -0.6199, 0.0246, 0.1516, 0.0159, -0.0553],
    [0.6377, -0.2790, 0.5188, 0.7851, 0.0523, -0.1448, -0.6235, 0.0289, 0.1373, 0.0089, -0.0561],
    [0.6477, -0.2969, 0.3315, 0.8136, 0.1441, -0.1424, -0.6274, 0.0329, 0.1226, 0.0020, -0.1117],
    [0.5846, -0.3149, 0.3241, 0.8418, 0.1286, -0.1401, -0.6312, 0.0372, 0.1080, -0.0047, -0.0890],
];
// CLIP RUN_FR END

// CLIP SPRINT_F BEGIN
/// AN-02-S13: Sprint, forward, from the Pro Rifle Pack clip "sprint forward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_F: [[f32; 11]; 48] = [
    [0.6854, -0.4673, 0.2945, 0.9775, -0.0091, -0.0539, -0.0028, 0.0219, 0.5207, -0.0573, -0.1232],
    [0.6260, -0.4106, 0.3296, 1.1140, 0.0058, -0.0724, 0.0254, 0.0233, 0.5146, -0.0706, -0.0960],
    [0.6431, -0.3351, 0.5087, 1.2446, 0.0151, -0.0872, 0.0523, 0.0234, 0.4914, -0.0813, -0.0874],
    [0.6631, -0.2599, 0.6898, 1.3751, 0.0205, -0.1008, 0.0793, 0.0238, 0.4681, -0.0924, -0.0893],
    [0.6627, -0.1804, 0.8332, 1.4952, 0.0212, -0.1079, 0.1042, 0.0230, 0.4343, -0.1007, -0.0945],
    [0.5939, -0.0809, 0.8490, 1.5755, 0.0159, -0.0964, 0.1205, 0.0198, 0.3606, -0.1000, -0.0842],
    [0.5275, 0.0197, 0.8678, 1.6554, 0.0083, -0.0872, 0.1368, 0.0176, 0.2862, -0.0992, -0.0786],
    [0.4643, 0.1211, 0.8926, 1.7348, -0.0016, -0.0802, 0.1535, 0.0153, 0.2114, -0.0984, -0.0783],
    [0.4040, 0.2226, 0.9245, 1.8143, -0.0142, -0.0749, 0.1696, 0.0139, 0.1362, -0.0975, -0.0836],
    [0.3461, 0.3240, 0.9638, 1.8931, -0.0297, -0.0720, 0.1863, 0.0128, 0.0612, -0.0970, -0.0945],
    [0.2892, 0.4244, 1.0098, 1.9712, -0.0487, -0.0706, 0.2028, 0.0124, -0.0142, -0.0961, -0.1116],
    [0.1847, 0.5122, 0.9516, 1.9385, -0.0646, -0.0587, 0.1997, 0.0097, -0.0917, -0.0868, -0.1137],
    [0.0672, 0.5968, 0.8613, 1.8770, -0.0782, -0.0425, 0.1917, 0.0061, -0.1708, -0.0752, -0.1154],
    [-0.0522, 0.6814, 0.7661, 1.8148, -0.0912, -0.0241, 0.1835, 0.0028, -0.2494, -0.0642, -0.1224],
    [-0.1740, 0.7659, 0.6717, 1.7524, -0.0717, -0.0036, 0.1763, -0.0011, -0.3293, -0.0526, -0.1074],
    [-0.2812, 0.8498, 0.5797, 1.6902, -0.0804, 0.0187, 0.1683, -0.0049, -0.4083, -0.0411, -0.1035],
    [-0.3872, 0.9329, 0.4884, 1.6285, -0.0889, 0.0432, 0.1605, -0.0088, -0.4877, -0.0299, -0.0997],
    [-0.4789, 0.9793, 0.4300, 1.5321, -0.0865, 0.0549, 0.1476, -0.0100, -0.5472, -0.0180, -0.0955],
    [-0.5512, 0.9722, 0.4207, 1.3825, -0.0677, 0.0469, 0.1273, -0.0070, -0.5774, -0.0045, -0.0909],
    [-0.6223, 0.9644, 0.4138, 1.2311, -0.0469, 0.0425, 0.1074, -0.0048, -0.6066, 0.0095, -0.0862],
    [-0.6745, 0.9472, 0.4263, 1.0846, -0.0235, 0.0373, 0.0867, -0.0020, -0.6252, 0.0223, -0.0820],
    [-0.6526, 0.8941, 0.5195, 0.9554, 0.0057, 0.0170, 0.0612, 0.0040, -0.5970, 0.0313, -0.0794],
    [-0.6291, 0.8408, 0.6143, 0.8251, 0.0340, -0.0017, 0.0365, 0.0101, -0.5685, 0.0408, -0.0768],
    [-0.6052, 0.7879, 0.7095, 0.6958, 0.0624, -0.0197, 0.0112, 0.0154, -0.5407, 0.0513, -0.0742],
    [-0.5438, 0.7787, 0.8270, 0.5003, 0.0804, -0.0962, -0.0155, 0.0090, -0.4849, 0.0560, -0.1389],
    [-0.4815, 0.7050, 0.9465, 0.5151, 0.0966, -0.1043, -0.0422, 0.0024, -0.4292, 0.0611, -0.1111],
    [-0.4185, 0.6294, 1.0669, 0.5211, 0.1102, -0.1082, -0.0686, -0.0037, -0.3731, 0.0661, -0.0873],
    [-0.3180, 0.6070, 1.2071, 0.6260, 0.1117, -0.1102, -0.0948, -0.0109, -0.3128, 0.0689, -0.0800],
    [-0.2074, 0.5998, 1.3545, 0.7601, 0.1111, -0.1098, -0.1212, -0.0183, -0.2507, 0.0706, -0.0829],
    [-0.0969, 0.5916, 1.5013, 0.8950, 0.1111, -0.1062, -0.1470, -0.0250, -0.1883, 0.0725, -0.0930],
    [0.0333, 0.5497, 1.6319, 0.9670, 0.1004, -0.0976, -0.1712, -0.0307, -0.1311, 0.0738, -0.0978],
    [0.1790, 0.4886, 1.7523, 1.0017, 0.0859, -0.0847, -0.1941, -0.0350, -0.0757, 0.0746, -0.1002],
    [0.3253, 0.4299, 1.8726, 1.0431, 0.0754, -0.0687, -0.2172, -0.0391, -0.0207, 0.0750, -0.1084],
    [0.4482, 0.3495, 1.9439, 1.0383, 0.0642, -0.0492, -0.2333, -0.0362, 0.0273, 0.0770, -0.1118],
    [0.5369, 0.2370, 1.9436, 0.9574, 0.0462, -0.0288, -0.2386, -0.0231, 0.0639, 0.0804, -0.1052],
    [0.6261, 0.1278, 1.9428, 0.8806, 0.0285, -0.0081, -0.2436, -0.0103, 0.1002, 0.0838, -0.1048],
    [0.7122, 0.0180, 1.9315, 0.8012, 0.0100, 0.0125, -0.2461, 0.0001, 0.1384, 0.0850, -0.1094],
    [0.7881, -0.1054, 1.8776, 0.6909, -0.0133, 0.0297, -0.2396, -0.0009, 0.1838, 0.0785, -0.1154],
    [0.8636, -0.2315, 1.8234, 0.5766, -0.0380, 0.0469, -0.2337, -0.0024, 0.2305, 0.0718, -0.1280],
    [0.9387, -0.3615, 1.7690, 0.4595, -0.0636, 0.0640, -0.2270, -0.0041, 0.2764, 0.0655, -0.1481],
    [0.9570, -0.4376, 1.6401, 0.4308, -0.0771, 0.0453, -0.2072, -0.0064, 0.3252, 0.0524, -0.1035],
    [0.9746, -0.4926, 1.5094, 0.4123, -0.0924, 0.0487, -0.1876, -0.0096, 0.3741, 0.0394, -0.0979],
    [0.9922, -0.5470, 1.3785, 0.3941, -0.1104, 0.0525, -0.1677, -0.0135, 0.4227, 0.0267, -0.0923],
    [0.9493, -0.5646, 1.2097, 0.4443, -0.1138, 0.0422, -0.1424, -0.0068, 0.4513, 0.0139, -0.0875],
    [0.8916, -0.5714, 1.0288, 0.5142, -0.1127, 0.0274, -0.1151, 0.0018, 0.4758, 0.0005, -0.0829],
    [0.8346, -0.5779, 0.8489, 0.5839, -0.1112, 0.0125, -0.0885, 0.0100, 0.4995, -0.0134, -0.0782],
    [0.7731, -0.5546, 0.6864, 0.6949, -0.0950, -0.0082, -0.0605, 0.0154, 0.5119, -0.0279, -0.0764],
    [0.7082, -0.5117, 0.5336, 0.8351, -0.0713, -0.0319, -0.0316, 0.0191, 0.5164, -0.0423, -0.0764],
];
// CLIP SPRINT_F END

// CLIP SPRINT_FL BEGIN
/// AN-02-S13: Sprint, forward left, from the Pro Rifle Pack clip "sprint forward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_FL: [[f32; 11]; 48] = [
    [0.6435, -0.1230, 0.4838, 1.5319, 0.0338, 0.1232, 0.6373, 0.0114, 0.3475, -0.0308, -0.0463],
    [0.5817, -0.0393, 0.4980, 1.6410, 0.0702, 0.1077, 0.6571, 0.0075, 0.3360, -0.0316, -0.0724],
    [0.5169, 0.0448, 0.5152, 1.7500, 0.0842, 0.0931, 0.6776, 0.0038, 0.3244, -0.0329, -0.0578],
    [0.4652, 0.1418, 0.5579, 1.8328, 0.0941, 0.0877, 0.6917, 0.0004, 0.3000, -0.0303, -0.0499],
    [0.4219, 0.2474, 0.6179, 1.9010, 0.1022, 0.0872, 0.7020, -0.0013, 0.2673, -0.0262, -0.0484],
    [0.3791, 0.3529, 0.6806, 1.9688, 0.1094, 0.0868, 0.7128, -0.0027, 0.2343, -0.0225, -0.0517],
    [0.3263, 0.4568, 0.7248, 2.0237, 0.1154, 0.0861, 0.7249, -0.0041, 0.1984, -0.0191, -0.0571],
    [0.2342, 0.5567, 0.6848, 2.0287, 0.1179, 0.0838, 0.7435, -0.0063, 0.1476, -0.0169, -0.0547],
    [0.1432, 0.6565, 0.6463, 2.0325, 0.1183, 0.0816, 0.7625, -0.0081, 0.0968, -0.0150, -0.0565],
    [0.0538, 0.7563, 0.6114, 2.0367, 0.1166, 0.0790, 0.7815, -0.0093, 0.0454, -0.0127, -0.0630],
    [-0.0400, 0.8460, 0.5676, 1.9940, 0.1145, 0.0585, 0.7939, -0.0083, -0.0215, -0.0145, -0.0728],
    [-0.1367, 0.9331, 0.5191, 1.9388, 0.1118, 0.0356, 0.8048, -0.0067, -0.0940, -0.0170, -0.0867],
    [-0.2358, 1.0192, 0.4683, 1.8836, 0.1080, 0.0151, 0.8157, -0.0049, -0.1659, -0.0188, -0.1053],
    [-0.2906, 1.0829, 0.4520, 1.8046, 0.1030, -0.0031, 0.8204, -0.0023, -0.2335, -0.0199, -0.0803],
    [-0.3073, 1.1049, 0.4925, 1.6769, 0.1008, -0.0186, 0.8136, 0.0031, -0.2925, -0.0205, -0.0779],
    [-0.3237, 1.1264, 0.5341, 1.5485, 0.0994, -0.0343, 0.8065, 0.0084, -0.3513, -0.0209, -0.0756],
    [-0.3394, 1.1473, 0.5766, 1.4208, 0.0985, -0.0513, 0.7996, 0.0125, -0.4106, -0.0209, -0.0732],
    [-0.3290, 1.1044, 0.6450, 1.3314, 0.1101, -0.0630, 0.7663, 0.0136, -0.4072, -0.0171, -0.0711],
    [-0.3153, 1.0549, 0.7169, 1.2438, 0.1240, -0.0762, 0.7304, 0.0141, -0.3971, -0.0132, -0.0690],
    [-0.3008, 1.0050, 0.7888, 1.1547, 0.1394, -0.0916, 0.6940, 0.0151, -0.3869, -0.0093, -0.0670],
    [-0.2857, 0.9544, 0.8604, 1.0645, 0.1553, -0.1085, 0.6576, 0.0160, -0.3764, -0.0057, -0.0649],
    [-0.2697, 0.9028, 0.9318, 0.9726, 0.1716, -0.1270, 0.6211, 0.0176, -0.3664, -0.0020, -0.0628],
    [-0.2524, 0.8505, 1.0038, 0.8799, 0.1875, -0.1467, 0.5847, 0.0188, -0.3557, 0.0017, -0.0607],
    [-0.2338, 0.7978, 1.0761, 0.7873, 0.2022, -0.1673, 0.5489, 0.0204, -0.3455, 0.0053, -0.0586],
    [-0.2141, 0.7441, 1.1487, 0.6946, 0.2161, -0.1890, 0.5123, 0.0220, -0.3347, 0.0089, -0.0565],
    [-0.1930, 0.6899, 1.2224, 0.6020, 0.2276, -0.2111, 0.4758, 0.0242, -0.3244, 0.0121, -0.0544],
    [-0.1704, 0.5722, 1.2980, 0.6342, 0.2363, -0.2380, 0.4396, 0.0263, -0.3134, 0.0159, -0.0905],
    [-0.1104, 0.5046, 1.4049, 0.6438, 0.2249, -0.2131, 0.4161, 0.0180, -0.2900, 0.0177, -0.0746],
    [-0.0283, 0.4447, 1.5311, 0.6645, 0.2020, -0.1863, 0.4000, 0.0042, -0.2589, 0.0185, -0.0642],
    [0.0533, 0.3837, 1.6595, 0.6806, 0.1772, -0.1584, 0.3841, -0.0095, -0.2282, 0.0188, -0.0574],
    [0.5372, 0.3817, 2.0806, 0.8222, 0.0110, -0.1320, 0.3018, -0.0609, -0.0886, 0.0242, -0.0731],
    [0.6690, 0.3018, 2.0743, 0.7925, -0.0138, -0.0949, 0.2846, -0.0611, -0.0407, 0.0269, -0.0669],
    [0.7993, 0.2234, 2.0663, 0.7620, -0.0377, -0.0561, 0.2672, -0.0613, 0.0057, 0.0295, -0.0648],
    [0.9291, 0.1490, 2.0611, 0.7373, -0.0613, -0.0152, 0.2496, -0.0617, 0.0521, 0.0321, -0.0678],
    [1.0051, 0.0670, 1.9814, 0.6965, -0.0729, 0.0251, 0.2448, -0.0540, 0.0965, 0.0337, -0.0731],
    [1.0627, -0.0198, 1.8809, 0.6458, -0.0919, 0.0656, 0.2436, -0.0443, 0.1399, 0.0347, -0.0818],
    [1.1182, -0.1083, 1.7810, 0.5913, -0.1198, 0.1079, 0.2420, -0.0348, 0.1830, 0.0356, -0.0951],
    [1.1487, -0.2587, 1.6634, 0.5102, -0.1443, 0.1941, 0.2493, -0.0238, 0.2188, 0.0361, -0.0768],
    [1.1317, -0.2994, 1.5077, 0.4935, -0.1541, 0.2084, 0.2745, -0.0110, 0.2412, 0.0341, -0.0723],
    [1.1142, -0.3386, 1.3513, 0.4797, -0.1664, 0.2226, 0.2996, 0.0024, 0.2640, 0.0309, -0.0678],
    [1.0968, -0.3777, 1.1969, 0.4655, -0.1800, 0.2370, 0.3246, 0.0157, 0.2857, 0.0277, -0.0634],
    [1.0470, -0.3940, 1.0583, 0.5689, -0.1666, 0.2450, 0.3653, 0.0237, 0.2972, 0.0210, -0.0596],
    [0.9945, -0.4068, 0.9200, 0.6854, -0.1460, 0.2508, 0.4066, 0.0314, 0.3073, 0.0139, -0.0559],
    [0.9424, -0.4202, 0.7818, 0.8005, -0.1198, 0.2531, 0.4490, 0.0387, 0.3183, 0.0055, -0.0522],
    [0.8863, -0.3970, 0.6778, 0.9376, -0.0887, 0.2359, 0.4911, 0.0390, 0.3287, -0.0034, -0.0494],
    [0.8254, -0.3347, 0.6095, 1.1006, -0.0554, 0.1997, 0.5339, 0.0315, 0.3389, -0.0123, -0.0475],
    [0.7636, -0.2712, 0.5415, 1.2641, -0.0229, 0.1671, 0.5760, 0.0234, 0.3500, -0.0214, -0.0456],
    [0.7009, -0.2064, 0.4766, 1.4227, 0.0092, 0.1394, 0.6169, 0.0155, 0.3588, -0.0296, -0.0439],
];
// CLIP SPRINT_FL END

// CLIP SPRINT_L BEGIN
/// AN-02-S13: Sprint, left, from the Pro Rifle Pack clip "sprint left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_L: [[f32; 11]; 48] = [
    [-0.0371, 1.0320, 0.6812, 1.5821, -0.3786, 0.3250, -0.8447, -0.0116, 0.0823, 0.0230, -0.1487],
    [0.0662, 1.0298, 0.7444, 1.7018, -0.3136, 0.3100, -0.8273, -0.0079, 0.0993, 0.0220, -0.1227],
    [0.1624, 1.0265, 0.7964, 1.8209, -0.2500, 0.3012, -0.8103, -0.0045, 0.1161, 0.0209, -0.1033],
    [0.2557, 0.9569, 0.8612, 1.8804, -0.1849, 0.2606, -0.8140, -0.0019, 0.1295, 0.0164, -0.0940],
    [0.3515, 0.8687, 0.9311, 1.9254, -0.1210, 0.2161, -0.8228, 0.0005, 0.1417, 0.0107, -0.0927],
    [0.4503, 0.7788, 1.0015, 1.9732, -0.0580, 0.1778, -0.8317, 0.0029, 0.1540, 0.0051, -0.0987],
    [0.5051, 0.6700, 0.9777, 1.9609, 0.0045, 0.1289, -0.8602, 0.0040, 0.1606, -0.0019, -0.0936],
    [0.5294, 0.5473, 0.8884, 1.9036, 0.0601, 0.0655, -0.9018, 0.0048, 0.1643, -0.0090, -0.0828],
    [0.5554, 0.4243, 0.7980, 1.8458, 0.1089, -0.0010, -0.9432, 0.0063, 0.1683, -0.0158, -0.0789],
    [0.5769, 0.3164, 0.6935, 1.7680, 0.1503, -0.0603, -0.9901, 0.0073, 0.1673, -0.0217, -0.0796],
    [0.5896, 0.2284, 0.5649, 1.6638, 0.1822, -0.1007, -1.0460, 0.0067, 0.1594, -0.0266, -0.0844],
    [0.6038, 0.1380, 0.4307, 1.5623, 0.2079, -0.1354, -1.1013, 0.0068, 0.1508, -0.0306, -0.0971],
    [0.6103, 0.0540, 0.4035, 1.4682, 0.1695, -0.1639, -1.1534, 0.0081, 0.1421, -0.0335, -0.0541],
    [0.6656, -0.0030, 0.4692, 1.3922, 0.1536, -0.1848, -1.1908, 0.0118, 0.1285, -0.0344, -0.0533],
    [0.7222, -0.0604, 0.5391, 1.3170, 0.1378, -0.2031, -1.2284, 0.0162, 0.1145, -0.0341, -0.0525],
    [0.7785, -0.1179, 0.6099, 1.2423, 0.1220, -0.2188, -1.2665, 0.0205, 0.1004, -0.0336, -0.0518],
    [0.8454, -0.1453, 0.7460, 1.1737, 0.0985, -0.2332, -1.2914, 0.0238, 0.0862, -0.0326, -0.0578],
    [0.9122, -0.1724, 0.8845, 1.1060, 0.0759, -0.2463, -1.3158, 0.0275, 0.0717, -0.0307, -0.0638],
    [0.9788, -0.2000, 1.0232, 1.0384, 0.0535, -0.2577, -1.3410, 0.0309, 0.0575, -0.0287, -0.0698],
    [1.0202, -0.2346, 1.1701, 0.9157, 0.0282, -0.2523, -1.3635, 0.0319, 0.0365, -0.0251, -0.0782],
    [1.0552, -0.2708, 1.3207, 0.7771, 0.0022, -0.2429, -1.3851, 0.0332, 0.0139, -0.0207, -0.0871],
    [1.0892, -0.3070, 1.4696, 0.6375, -0.0238, -0.2342, -1.4069, 0.0336, -0.0091, -0.0162, -0.0960],
    [1.0715, -0.2650, 1.5754, 0.6121, -0.0420, -0.2203, -1.4175, 0.0313, -0.0313, -0.0126, -0.1006],
    [1.0182, -0.1673, 1.6523, 0.6680, -0.0548, -0.2044, -1.4202, 0.0268, -0.0543, -0.0097, -0.1022],
    [0.9645, -0.1164, 1.7290, 0.7051, -0.0674, -0.1955, -1.4224, 0.0226, -0.0764, -0.0069, -0.1368],
    [0.9108, 0.0068, 1.8063, 0.7762, -0.0800, -0.1754, -1.4247, 0.0186, -0.0988, -0.0041, -0.1175],
    [0.8572, 0.1272, 1.8833, 0.8427, -0.0925, -0.1581, -1.4275, 0.0143, -0.1210, -0.0013, -0.1052],
    [0.8037, 0.2454, 1.9600, 0.9036, -0.1056, -0.1431, -1.4299, 0.0100, -0.1433, 0.0017, -0.0990],
    [0.7411, 0.3530, 2.0154, 0.9384, -0.1191, -0.1295, -1.4278, 0.0055, -0.1628, 0.0040, -0.0950],
    [0.6420, 0.4245, 1.9898, 0.8947, -0.1338, -0.1143, -1.4064, 0.0005, -0.1721, 0.0053, -0.0832],
    [0.5417, 0.4944, 1.9633, 0.8533, -0.1481, -0.0965, -1.3849, -0.0043, -0.1812, 0.0065, -0.0779],
    [0.4417, 0.5654, 1.9365, 0.8171, -0.1626, -0.0760, -1.3638, -0.0093, -0.1902, 0.0082, -0.0795],
    [0.3469, 0.6032, 1.8482, 0.7205, -0.1689, -0.0485, -1.3329, -0.0125, -0.1875, 0.0092, -0.0776],
    [0.2518, 0.6422, 1.7593, 0.6303, -0.1766, -0.0158, -1.3017, -0.0157, -0.1847, 0.0111, -0.0838],
    [0.1571, 0.6827, 1.6697, 0.5457, -0.1857, 0.0234, -1.2703, -0.0190, -0.1819, 0.0131, -0.0979],
    [0.0846, 0.7444, 1.5615, 0.5336, -0.2053, 0.0850, -1.2346, -0.0220, -0.1680, 0.0140, -0.0578],
    [0.0175, 0.8015, 1.4470, 0.5672, -0.2283, 0.1348, -1.1980, -0.0245, -0.1512, 0.0149, -0.0520],
    [-0.0497, 0.8563, 1.3309, 0.6003, -0.2523, 0.1868, -1.1613, -0.0266, -0.1341, 0.0163, -0.0462],
    [-0.0875, 0.8970, 1.2247, 0.6612, -0.2780, 0.2332, -1.1251, -0.0277, -0.1145, 0.0174, -0.0457],
    [-0.1055, 0.9268, 1.1257, 0.7396, -0.3034, 0.2730, -1.0892, -0.0282, -0.0925, 0.0184, -0.0489],
    [-0.1242, 0.9538, 1.0258, 0.8157, -0.3280, 0.3097, -1.0534, -0.0282, -0.0710, 0.0195, -0.0521],
    [-0.1434, 0.9778, 0.9259, 0.8888, -0.3523, 0.3435, -1.0175, -0.0280, -0.0491, 0.0212, -0.0553],
    [-0.1631, 0.9993, 0.8262, 0.9593, -0.3761, 0.3746, -0.9815, -0.0277, -0.0272, 0.0230, -0.0585],
    [-0.1830, 1.0190, 0.7276, 1.0283, -0.4002, 0.4022, -0.9457, -0.0268, -0.0054, 0.0251, -0.0617],
    [-0.1886, 1.0335, 0.6535, 1.1049, -0.4134, 0.4173, -0.9135, -0.0253, 0.0151, 0.0269, -0.0658],
    [-0.1341, 1.0330, 0.6759, 1.2216, -0.3838, 0.3913, -0.8961, -0.0220, 0.0319, 0.0257, -0.0734],
    [-0.0779, 1.0330, 0.7008, 1.3414, -0.3550, 0.3667, -0.8790, -0.0182, 0.0486, 0.0248, -0.0811],
    [-0.0215, 1.0327, 0.7252, 1.4616, -0.3272, 0.3442, -0.8621, -0.0149, 0.0660, 0.0241, -0.0888],
];
// CLIP SPRINT_L END

// CLIP SPRINT_BL BEGIN
/// AN-02-S13: Sprint, backward left, from the Pro Rifle Pack clip "sprint backward left" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_BL: [[f32; 11]; 48] = [
    [0.1339, 1.1085, 0.7716, 1.2447, 0.0513, -0.0240, -0.7359, -0.0076, -0.2156, 0.0642, -0.0785],
    [0.2259, 1.1357, 0.7718, 1.3748, 0.0545, -0.0253, -0.7321, -0.0094, -0.1759, 0.0728, -0.0664],
    [0.3272, 1.1102, 0.7942, 1.4532, 0.0588, -0.0222, -0.7248, -0.0111, -0.1361, 0.0752, -0.0639],
    [0.4299, 1.0843, 0.8189, 1.5310, 0.0649, -0.0205, -0.7175, -0.0123, -0.0970, 0.0771, -0.0679],
    [0.5341, 1.0583, 0.8437, 1.6085, 0.0732, -0.0198, -0.7101, -0.0129, -0.0588, 0.0800, -0.0785],
    [0.5658, 1.0041, 0.7238, 1.6309, 0.0814, -0.0161, -0.7016, -0.0142, -0.0222, 0.0775, -0.0728],
    [0.5768, 0.9421, 0.5671, 1.6383, 0.0918, -0.0120, -0.6925, -0.0156, 0.0130, 0.0735, -0.0714],
    [0.5875, 0.8796, 0.4098, 1.6457, 0.1051, -0.0096, -0.6838, -0.0169, 0.0478, 0.0697, -0.0808],
    [0.6420, 0.8052, 0.3337, 1.6178, 0.1232, -0.0098, -0.6751, -0.0179, 0.0748, 0.0613, -0.1072],
    [0.6838, 0.7221, 0.4125, 1.5660, 0.1138, -0.0125, -0.6670, -0.0173, 0.0957, 0.0498, -0.0751],
    [0.7406, 0.6379, 0.4657, 1.5132, 0.1172, -0.0169, -0.6591, -0.0175, 0.1165, 0.0387, -0.0688],
    [0.7966, 0.5538, 0.5171, 1.4605, 0.1215, -0.0226, -0.6510, -0.0182, 0.1367, 0.0274, -0.0626],
    [0.8528, 0.4701, 0.5692, 1.4077, 0.1262, -0.0296, -0.6429, -0.0191, 0.1562, 0.0162, -0.0563],
    [0.9086, 0.3871, 0.6205, 1.3551, 0.1310, -0.0378, -0.6350, -0.0204, 0.1761, 0.0053, -0.0501],
    [0.9539, 0.3062, 0.6605, 1.2872, 0.1353, -0.0484, -0.6265, -0.0214, 0.1911, -0.0046, -0.0452],
    [0.9570, 0.2295, 0.6535, 1.1593, 0.1373, -0.0631, -0.6151, -0.0186, 0.1932, -0.0100, -0.0458],
    [0.9594, 0.1520, 0.6458, 1.0287, 0.1391, -0.0771, -0.6036, -0.0159, 0.1960, -0.0154, -0.0464],
    [0.9617, 0.0743, 0.6377, 0.8975, 0.1405, -0.0900, -0.5922, -0.0134, 0.1988, -0.0210, -0.0469],
    [0.9640, -0.0032, 0.6299, 0.7679, 0.1421, -0.1022, -0.5809, -0.0108, 0.2013, -0.0267, -0.0475],
    [0.9663, -0.0801, 0.6226, 0.6398, 0.1437, -0.1136, -0.5695, -0.0080, 0.2038, -0.0323, -0.0481],
    [0.9686, -0.1562, 0.6152, 0.5152, 0.1450, -0.1247, -0.5583, -0.0055, 0.2068, -0.0382, -0.0487],
    [0.9976, -0.3125, 0.6830, 0.4552, 0.1445, -0.1903, -0.5413, -0.0011, 0.2101, -0.0375, -0.1391],
    [1.0353, -0.1926, 0.7739, 0.5179, 0.1446, -0.1680, -0.5234, 0.0039, 0.2128, -0.0351, -0.1095],
    [1.0730, -0.0772, 0.8656, 0.5778, 0.1459, -0.1458, -0.5053, 0.0091, 0.2155, -0.0332, -0.0874],
    [1.0930, 0.0431, 0.9528, 0.6549, 0.1402, -0.1233, -0.4867, 0.0132, 0.2092, -0.0327, -0.0746],
    [1.1013, 0.1703, 1.0368, 0.7502, 0.1292, -0.0995, -0.4682, 0.0169, 0.1971, -0.0330, -0.0714],
    [1.1093, 0.2983, 1.1204, 0.8467, 0.1197, -0.0734, -0.4488, 0.0211, 0.1851, -0.0334, -0.0757],
    [1.0986, 0.4021, 1.1951, 0.8875, 0.1032, -0.0434, -0.4311, 0.0232, 0.1683, -0.0357, -0.0776],
    [1.0590, 0.4660, 1.2548, 0.8439, 0.0763, -0.0101, -0.4157, 0.0245, 0.1429, -0.0391, -0.0713],
    [1.0185, 0.5279, 1.3134, 0.8008, 0.0516, 0.0254, -0.4005, 0.0261, 0.1178, -0.0422, -0.0713],
    [0.9710, 0.5858, 1.3622, 0.7525, 0.0267, 0.0637, -0.3859, 0.0270, 0.0900, -0.0460, -0.0769],
    [0.8960, 0.6558, 1.3727, 0.7268, -0.0027, 0.1046, -0.3765, 0.0246, 0.0517, -0.0498, -0.0924],
    [0.8205, 0.6647, 1.3825, 0.5859, -0.0313, 0.1466, -0.3673, 0.0227, 0.0137, -0.0539, -0.0990],
    [0.7451, 0.6977, 1.3916, 0.4902, -0.0596, 0.1946, -0.3577, 0.0218, -0.0251, -0.0579, -0.1208],
    [0.6550, 0.6773, 1.3540, 0.5944, -0.0761, 0.1746, -0.3572, 0.0173, -0.0683, -0.0562, -0.0652],
    [0.5643, 0.6954, 1.3155, 0.5793, -0.0927, 0.1954, -0.3571, 0.0131, -0.1117, -0.0547, -0.0592],
    [0.4739, 0.7124, 1.2771, 0.5619, -0.1096, 0.2166, -0.3565, 0.0094, -0.1549, -0.0534, -0.0532],
    [0.3895, 0.7422, 1.2103, 0.5715, -0.1039, 0.1989, -0.3946, 0.0080, -0.1832, -0.0448, -0.0511],
    [0.3056, 0.7751, 1.1357, 0.5896, -0.0915, 0.1711, -0.4421, 0.0071, -0.2079, -0.0344, -0.0499],
    [0.2208, 0.8073, 1.0616, 0.6085, -0.0775, 0.1430, -0.4900, 0.0062, -0.2325, -0.0245, -0.0488],
    [0.1353, 0.8392, 0.9876, 0.6279, -0.0618, 0.1147, -0.5380, 0.0053, -0.2573, -0.0153, -0.0476],
    [0.0498, 0.8703, 0.9147, 0.6476, -0.0445, 0.0862, -0.5862, 0.0048, -0.2814, -0.0062, -0.0465],
    [-0.0356, 0.9003, 0.8431, 0.6676, -0.0251, 0.0573, -0.6346, 0.0043, -0.3058, 0.0023, -0.0453],
    [-0.0743, 0.9345, 0.7957, 0.7144, -0.0055, 0.0321, -0.6734, 0.0037, -0.3183, 0.0120, -0.0473],
    [-0.0423, 0.9757, 0.7846, 0.8016, 0.0125, 0.0111, -0.6993, 0.0017, -0.3124, 0.0243, -0.0539],
    [-0.0094, 1.0167, 0.7755, 0.8890, 0.0297, -0.0094, -0.7249, -0.0003, -0.3068, 0.0367, -0.0606],
    [0.0290, 1.0542, 0.7680, 0.9843, 0.0438, -0.0251, -0.7443, -0.0022, -0.2945, 0.0479, -0.0673],
    [0.0351, 1.0812, 0.7595, 1.1140, 0.0492, -0.0239, -0.7399, -0.0052, -0.2550, 0.0560, -0.0954],
];
// CLIP SPRINT_BL END

// CLIP SPRINT_B BEGIN
/// AN-02-S13: Sprint, backward, from the Pro Rifle Pack clip "sprint backward" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_B: [[f32; 11]; 48] = [
    [0.1183, 1.0645, 0.7545, 1.1620, 0.1272, -0.2197, -0.2960, 0.0910, -0.2205, 0.0458, -0.0828],
    [0.2270, 1.1040, 0.8013, 1.3066, 0.0986, -0.2069, -0.2924, 0.0962, -0.1817, 0.0528, -0.0749],
    [0.3177, 1.0912, 0.8065, 1.3948, 0.0709, -0.1804, -0.2852, 0.0910, -0.1438, 0.0563, -0.0670],
    [0.4327, 1.0770, 0.8629, 1.4819, 0.0458, -0.1571, -0.2782, 0.0869, -0.1067, 0.0597, -0.0737],
    [0.5341, 1.0624, 0.8859, 1.5693, 0.0214, -0.1368, -0.2713, 0.0827, -0.0689, 0.0633, -0.0816],
    [0.5668, 1.0172, 0.7700, 1.5989, -0.0019, -0.1116, -0.2628, 0.0711, -0.0354, 0.0637, -0.0734],
    [0.5782, 0.9628, 0.6159, 1.6133, -0.0233, -0.0868, -0.2543, 0.0578, -0.0027, 0.0637, -0.0688],
    [0.5903, 0.9072, 0.4631, 1.6275, -0.0433, -0.0638, -0.2453, 0.0446, 0.0299, 0.0635, -0.0749],
    [0.6624, 0.8400, 0.4224, 1.6180, -0.0639, -0.0417, -0.2365, 0.0296, 0.0599, 0.0603, -0.1011],
    [0.7546, 0.7645, 0.5064, 1.5934, -0.0840, -0.0206, -0.2288, 0.0129, 0.0863, 0.0548, -0.0729],
    [0.8534, 0.6888, 0.5815, 1.5681, -0.1038, -0.0013, -0.2203, -0.0036, 0.1134, 0.0498, -0.0664],
    [0.9227, 0.6098, 0.6503, 1.5277, -0.1177, 0.0144, -0.2124, -0.0198, 0.1350, 0.0438, -0.0602],
    [0.9488, 0.5254, 0.7104, 1.4652, -0.1198, 0.0263, -0.2045, -0.0352, 0.1496, 0.0356, -0.0547],
    [0.9746, 0.4406, 0.7694, 1.4014, -0.1221, 0.0372, -0.1975, -0.0508, 0.1624, 0.0278, -0.0492],
    [0.9968, 0.3585, 0.8166, 1.3278, -0.1231, 0.0445, -0.1893, -0.0635, 0.1735, 0.0204, -0.0450],
    [1.0013, 0.2847, 0.8120, 1.2129, -0.1249, 0.0397, -0.1780, -0.0651, 0.1770, 0.0135, -0.0457],
    [1.0054, 0.2105, 0.8072, 1.0964, -0.1267, 0.0355, -0.1667, -0.0668, 0.1800, 0.0066, -0.0464],
    [1.0096, 0.1359, 0.8027, 0.9790, -0.1286, 0.0317, -0.1556, -0.0685, 0.1837, -0.0004, -0.0472],
    [1.0137, 0.0613, 0.7980, 0.8622, -0.1303, 0.0281, -0.1442, -0.0704, 0.1866, -0.0074, -0.0479],
    [1.0179, -0.0127, 0.7940, 0.7467, -0.1322, 0.0250, -0.1329, -0.0726, 0.1903, -0.0145, -0.0486],
    [1.0220, -0.0859, 0.7904, 0.6341, -0.1342, 0.0221, -0.1217, -0.0747, 0.1936, -0.0215, -0.0494],
    [1.0463, -0.2760, 0.8481, 0.5358, -0.1317, 0.0146, -0.1052, -0.0658, 0.1981, -0.0260, -0.1329],
    [1.0763, -0.1729, 0.9228, 0.5559, -0.1267, -0.0047, -0.0869, -0.0539, 0.2023, -0.0305, -0.1015],
    [1.1067, -0.0753, 0.9981, 0.5703, -0.1206, -0.0191, -0.0686, -0.0430, 0.2070, -0.0350, -0.0768],
    [1.1193, 0.0372, 1.0719, 0.6241, -0.1134, -0.0298, -0.0501, -0.0346, 0.2033, -0.0389, -0.0634],
    [1.1198, 0.1745, 1.1444, 0.7358, -0.1059, -0.0377, -0.0311, -0.0278, 0.1948, -0.0431, -0.0645],
    [1.1205, 0.3152, 1.2168, 0.8509, -0.0993, -0.0427, -0.0119, -0.0211, 0.1863, -0.0471, -0.0740],
    [1.0999, 0.4269, 1.2786, 0.8957, -0.0943, -0.0440, 0.0055, -0.0173, 0.1723, -0.0510, -0.0789],
    [1.0474, 0.4893, 1.3254, 0.8354, -0.0910, -0.0416, 0.0208, -0.0176, 0.1495, -0.0543, -0.0722],
    [0.9948, 0.5512, 1.3716, 0.7773, -0.0887, -0.0367, 0.0365, -0.0177, 0.1264, -0.0576, -0.0723],
    [0.9356, 0.6086, 1.4086, 0.7115, -0.0874, -0.0296, 0.0508, -0.0185, 0.1012, -0.0602, -0.0778],
    [0.8496, 0.6492, 1.4087, 0.6129, -0.0875, -0.0216, 0.0601, -0.0251, 0.0626, -0.0596, -0.0859],
    [0.7631, 0.6866, 1.4078, 0.5067, -0.0881, -0.0118, 0.0694, -0.0311, 0.0245, -0.0590, -0.1013],
    [0.6769, 0.7247, 1.4070, 0.3974, -0.0890, 0.0003, 0.0789, -0.0367, -0.0141, -0.0581, -0.1255],
    [0.5951, 0.6969, 1.3392, 0.5162, -0.0707, -0.0194, 0.0476, -0.0285, -0.0456, -0.0518, -0.0670],
    [0.5121, 0.7159, 1.2706, 0.5173, -0.0507, -0.0389, 0.0158, -0.0205, -0.0776, -0.0454, -0.0641],
    [0.4279, 0.7338, 1.2010, 0.5163, -0.0295, -0.0589, -0.0158, -0.0128, -0.1096, -0.0392, -0.0612],
    [0.3432, 0.7509, 1.1310, 0.5152, -0.0066, -0.0793, -0.0473, -0.0054, -0.1419, -0.0328, -0.0584],
    [0.2582, 0.7680, 1.0607, 0.5148, 0.0177, -0.1001, -0.0792, 0.0020, -0.1740, -0.0260, -0.0555],
    [0.1729, 0.7840, 0.9905, 0.5128, 0.0435, -0.1215, -0.1112, 0.0086, -0.2063, -0.0195, -0.0526],
    [0.0881, 0.7998, 0.9206, 0.5120, 0.0712, -0.1430, -0.1429, 0.0151, -0.2391, -0.0130, -0.0497],
    [0.0039, 0.8152, 0.8515, 0.5110, 0.1008, -0.1652, -0.1748, 0.0214, -0.2709, -0.0065, -0.0468],
    [-0.0791, 0.8298, 0.7832, 0.5099, 0.1327, -0.1874, -0.2068, 0.0273, -0.3032, -0.0002, -0.0439],
    [-0.1131, 0.8555, 0.7404, 0.5502, 0.1540, -0.2080, -0.2359, 0.0373, -0.3205, 0.0068, -0.0448],
    [-0.0744, 0.8992, 0.7372, 0.6548, 0.1561, -0.2249, -0.2607, 0.0532, -0.3161, 0.0151, -0.0517],
    [-0.0351, 0.9431, 0.7346, 0.7600, 0.1579, -0.2416, -0.2851, 0.0691, -0.3102, 0.0236, -0.0585],
    [0.0091, 0.9859, 0.7350, 0.8730, 0.1557, -0.2517, -0.3046, 0.0824, -0.2993, 0.0324, -0.0654],
    [0.0185, 1.0251, 0.7298, 1.0168, 0.1592, -0.2346, -0.3000, 0.0863, -0.2604, 0.0393, -0.1010],
];
// CLIP SPRINT_B END

// CLIP SPRINT_BR BEGIN
/// AN-02-S13: Sprint, backward right, from the Pro Rifle Pack clip "sprint backward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_BR: [[f32; 11]; 48] = [
    [0.0678, 0.8886, 0.6173, 1.3425, 0.1468, -0.0788, 0.0318, 0.0492, -0.2049, -0.0451, -0.0650],
    [0.1358, 0.8679, 0.6452, 1.3659, 0.0948, -0.0313, 0.0204, 0.0500, -0.1918, -0.0352, -0.0552],
    [0.2047, 0.8444, 0.6781, 1.3879, 0.0426, 0.0148, 0.0091, 0.0504, -0.1782, -0.0250, -0.0508],
    [0.2752, 0.8191, 0.7161, 1.4095, -0.0106, 0.0597, -0.0018, 0.0512, -0.1645, -0.0153, -0.0517],
    [0.3473, 0.7919, 0.7593, 1.4311, -0.0654, 0.1034, -0.0131, 0.0523, -0.1513, -0.0052, -0.0582],
    [0.4210, 0.7629, 0.8067, 1.4522, -0.1229, 0.1454, -0.0243, 0.0529, -0.1377, 0.0049, -0.0704],
    [0.4950, 0.7324, 0.8551, 1.4731, -0.1840, 0.1864, -0.0358, 0.0536, -0.1240, 0.0148, -0.0881],
    [0.5069, 0.6837, 0.7730, 1.4464, -0.2411, 0.2274, -0.0396, 0.0523, -0.0976, 0.0253, -0.0895],
    [0.5020, 0.6293, 0.6582, 1.4077, -0.2970, 0.2685, -0.0407, 0.0508, -0.0672, 0.0359, -0.0925],
    [0.4966, 0.5749, 0.5450, 1.3694, -0.3549, 0.3092, -0.0422, 0.0490, -0.0376, 0.0464, -0.1028],
    [0.5334, 0.5202, 0.4868, 1.3196, -0.3855, 0.3334, -0.0368, 0.0464, -0.0125, 0.0528, -0.0856],
    [0.5645, 0.4655, 0.5289, 1.2611, -0.3880, 0.3466, -0.0262, 0.0434, 0.0084, 0.0565, -0.0823],
    [0.5966, 0.4109, 0.5734, 1.2029, -0.3908, 0.3605, -0.0158, 0.0405, 0.0297, 0.0602, -0.0791],
    [0.6289, 0.3564, 0.6184, 1.1445, -0.3935, 0.3745, -0.0055, 0.0376, 0.0508, 0.0641, -0.0758],
    [0.6611, 0.3021, 0.6631, 1.0863, -0.3961, 0.3890, 0.0050, 0.0352, 0.0723, 0.0678, -0.0725],
    [0.6933, 0.2478, 0.7076, 1.0280, -0.3987, 0.4039, 0.0155, 0.0324, 0.0933, 0.0716, -0.0692],
    [0.7228, 0.1981, 0.7605, 0.9735, -0.3977, 0.4135, 0.0283, 0.0292, 0.1121, 0.0737, -0.0672],
    [0.7413, 0.1641, 0.8467, 0.9308, -0.3823, 0.4005, 0.0508, 0.0220, 0.1211, 0.0675, -0.0701],
    [0.7608, 0.1301, 0.9342, 0.8887, -0.3666, 0.3879, 0.0739, 0.0150, 0.1296, 0.0613, -0.0731],
    [0.7801, 0.0956, 1.0223, 0.8453, -0.3511, 0.3762, 0.0965, 0.0074, 0.1378, 0.0554, -0.0760],
    [0.8001, 0.0612, 1.1111, 0.8020, -0.3355, 0.3652, 0.1194, -0.0004, 0.1455, 0.0499, -0.0789],
    [0.8200, 0.0268, 1.1993, 0.7583, -0.3203, 0.3546, 0.1424, -0.0085, 0.1535, 0.0444, -0.0818],
    [0.8398, -0.0071, 1.2863, 0.7153, -0.3058, 0.3451, 0.1648, -0.0163, 0.1603, 0.0390, -0.0847],
    [0.8299, 0.0395, 1.3421, 0.7291, -0.2832, 0.3223, 0.1869, -0.0242, 0.1631, 0.0320, -0.1269],
    [0.8117, 0.1169, 1.3892, 0.7776, -0.2593, 0.2675, 0.2086, -0.0325, 0.1642, 0.0246, -0.1102],
    [0.7936, 0.1955, 1.4358, 0.8234, -0.2359, 0.2175, 0.2303, -0.0406, 0.1654, 0.0173, -0.0984],
    [0.7756, 0.2827, 1.4830, 0.8840, -0.2129, 0.1725, 0.2521, -0.0491, 0.1660, 0.0101, -0.0943],
    [0.7568, 0.3578, 1.5293, 0.9114, -0.1906, 0.1297, 0.2743, -0.0579, 0.1664, 0.0033, -0.0893],
    [0.7380, 0.4419, 1.5754, 0.9527, -0.1687, 0.0909, 0.2958, -0.0668, 0.1666, -0.0030, -0.0915],
    [0.6940, 0.5079, 1.5990, 0.9491, -0.1483, 0.0540, 0.3149, -0.0731, 0.1626, -0.0079, -0.0901],
    [0.6113, 0.5459, 1.5881, 0.8848, -0.1284, 0.0191, 0.3300, -0.0757, 0.1503, -0.0109, -0.0825],
    [0.5282, 0.5864, 1.5758, 0.8261, -0.1082, -0.0132, 0.3449, -0.0778, 0.1377, -0.0137, -0.0810],
    [0.4480, 0.6291, 1.5563, 0.7701, -0.0866, -0.0446, 0.3557, -0.0784, 0.1213, -0.0169, -0.0849],
    [0.3777, 0.6631, 1.5095, 0.6949, -0.0592, -0.0811, 0.3489, -0.0723, 0.0898, -0.0215, -0.0918],
    [0.3070, 0.6935, 1.4629, 0.6132, -0.0306, -0.1197, 0.3425, -0.0660, 0.0570, -0.0263, -0.1038],
    [0.2360, 0.7205, 1.4157, 0.5247, -0.0011, -0.1618, 0.3356, -0.0595, 0.0244, -0.0309, -0.1216],
    [0.1951, 0.7687, 1.3526, 0.4748, 0.0310, -0.2183, 0.3149, -0.0515, -0.0070, -0.0385, -0.1519],
    [0.1540, 0.7635, 1.2889, 0.6609, 0.0636, -0.1240, 0.2935, -0.0434, -0.0374, -0.0461, -0.0682],
    [0.1127, 0.7933, 1.2253, 0.7092, 0.0965, -0.1240, 0.2722, -0.0352, -0.0684, -0.0536, -0.0633],
    [0.0840, 0.8093, 1.1494, 0.7646, 0.1203, -0.1330, 0.2467, -0.0254, -0.0928, -0.0595, -0.0610],
    [0.0584, 0.8221, 1.0691, 0.8228, 0.1408, -0.1423, 0.2203, -0.0157, -0.1154, -0.0647, -0.0594],
    [0.0332, 0.8338, 0.9885, 0.8798, 0.1601, -0.1492, 0.1939, -0.0060, -0.1379, -0.0694, -0.0578],
    [-0.0021, 0.8464, 0.8783, 0.9489, 0.1872, -0.1556, 0.1672, 0.0042, -0.1564, -0.0713, -0.0596],
    [-0.0437, 0.8589, 0.7492, 1.0253, 0.2172, -0.1596, 0.1412, 0.0140, -0.1725, -0.0707, -0.0635],
    [-0.0850, 0.8719, 0.6214, 1.1011, 0.2443, -0.1606, 0.1151, 0.0242, -0.1887, -0.0699, -0.0675],
    [-0.0958, 0.8817, 0.5511, 1.1703, 0.2517, -0.1512, 0.0917, 0.0324, -0.1994, -0.0668, -0.0715],
    [-0.0757, 0.8861, 0.5582, 1.2313, 0.2470, -0.1298, 0.0712, 0.0384, -0.2027, -0.0598, -0.0985],
    [-0.0013, 0.8899, 0.5910, 1.2911, 0.1972, -0.1074, 0.0505, 0.0442, -0.2053, -0.0529, -0.0797],
];
// CLIP SPRINT_BR END

// CLIP SPRINT_R BEGIN
/// AN-02-S13: Sprint, right, from the Pro Rifle Pack clip "sprint right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_R: [[f32; 11]; 48] = [
    [0.5869, -0.0400, 0.3594, 1.4038, 0.3261, -0.3347, -0.9070, -0.0147, 0.1866, -0.0484, -0.1221],
    [0.5581, 0.0250, 0.4393, 1.4718, 0.2861, -0.2941, -0.8618, -0.0115, 0.1770, -0.0504, -0.0994],
    [0.5497, 0.1123, 0.5519, 1.5266, 0.2484, -0.2490, -0.8061, -0.0051, 0.1739, -0.0540, -0.0870],
    [0.5462, 0.1982, 0.6641, 1.5826, 0.2065, -0.2017, -0.7503, 0.0012, 0.1725, -0.0573, -0.0811],
    [0.5445, 0.2831, 0.7698, 1.6310, 0.1585, -0.1445, -0.6990, 0.0069, 0.1671, -0.0581, -0.0804],
    [0.5405, 0.3709, 0.8623, 1.6650, 0.1042, -0.0768, -0.6527, 0.0116, 0.1565, -0.0553, -0.0835],
    [0.5418, 0.4596, 0.9572, 1.6983, 0.0445, -0.0121, -0.6077, 0.0165, 0.1461, -0.0522, -0.0928],
    [0.5340, 0.5434, 1.0264, 1.7148, -0.0217, 0.0505, -0.5682, 0.0206, 0.1326, -0.0477, -0.1032],
    [0.4780, 0.6074, 0.9956, 1.6701, -0.0915, 0.1201, -0.5526, 0.0218, 0.1067, -0.0386, -0.0995],
    [0.4256, 0.6704, 0.9666, 1.6261, -0.1621, 0.1943, -0.5371, 0.0231, 0.0808, -0.0296, -0.1014],
    [0.3777, 0.7330, 0.9417, 1.5839, -0.2343, 0.2717, -0.5219, 0.0245, 0.0545, -0.0204, -0.1093],
    [0.2923, 0.7412, 0.8483, 1.4643, -0.3015, 0.3168, -0.5288, 0.0237, 0.0241, -0.0095, -0.1107],
    [0.2064, 0.7459, 0.7532, 1.3439, -0.3684, 0.3691, -0.5358, 0.0224, -0.0058, 0.0019, -0.1184],
    [0.1201, 0.7475, 0.6575, 1.2235, -0.4362, 0.4273, -0.5429, 0.0213, -0.0352, 0.0128, -0.1331],
    [0.0836, 0.7322, 0.6829, 1.1240, -0.4416, 0.4381, -0.5687, 0.0195, -0.0576, 0.0189, -0.0989],
    [0.0774, 0.7138, 0.7353, 1.0299, -0.4378, 0.4362, -0.5986, 0.0173, -0.0769, 0.0237, -0.0911],
    [0.0707, 0.6972, 0.7886, 0.9364, -0.4347, 0.4335, -0.6290, 0.0148, -0.0962, 0.0285, -0.0832],
    [0.0645, 0.6813, 0.8428, 0.8431, -0.4319, 0.4298, -0.6591, 0.0123, -0.1153, 0.0339, -0.0753],
    [0.0581, 0.6675, 0.8972, 0.7518, -0.4296, 0.4244, -0.6892, 0.0098, -0.1346, 0.0393, -0.0675],
    [0.0518, 0.6546, 0.9525, 0.6609, -0.4274, 0.4169, -0.7195, 0.0070, -0.1533, 0.0452, -0.0596],
    [0.0454, 0.6429, 1.0079, 0.5717, -0.4254, 0.4073, -0.7493, 0.0042, -0.1726, 0.0509, -0.0517],
    [0.0387, 0.6318, 1.0637, 0.4837, -0.4230, 0.3945, -0.7794, 0.0010, -0.1909, 0.0567, -0.0438],
    [0.0321, 0.6208, 1.1206, 0.3965, -0.4201, 0.3787, -0.8094, -0.0027, -0.2092, 0.0629, -0.0359],
    [0.0388, 0.6092, 1.1934, 0.3324, -0.4098, 0.3513, -0.8410, -0.0060, -0.2240, 0.0680, -0.0309],
    [0.0978, 0.5940, 1.3279, 0.3567, -0.3731, 0.2898, -0.8784, -0.0083, -0.2253, 0.0689, -0.0374],
    [0.1562, 0.5772, 1.4605, 0.3831, -0.3346, 0.2302, -0.9157, -0.0108, -0.2257, 0.0700, -0.0438],
    [0.2130, 0.5630, 1.5896, 0.3952, -0.2947, 0.1804, -0.9536, -0.0133, -0.2264, 0.0717, -0.0832],
    [0.3038, 0.5683, 1.6995, 0.5163, -0.2636, 0.1221, -0.9859, -0.0153, -0.2172, 0.0664, -0.0726],
    [0.3949, 0.5751, 1.8067, 0.6456, -0.2351, 0.0692, -1.0187, -0.0171, -0.2067, 0.0616, -0.0704],
    [0.4858, 0.5797, 1.9105, 0.7777, -0.2089, 0.0207, -1.0514, -0.0187, -0.1972, 0.0573, -0.0761],
    [0.5047, 0.5449, 1.9323, 0.8463, -0.2039, -0.0202, -1.0576, -0.0191, -0.1949, 0.0563, -0.0779],
    [0.5047, 0.4921, 1.9323, 0.8824, -0.2039, -0.0598, -1.0576, -0.0190, -0.1947, 0.0561, -0.0795],
    [0.5047, 0.4299, 1.9323, 0.9000, -0.2039, -0.0995, -1.0576, -0.0191, -0.1948, 0.0562, -0.0829],
    [0.8607, 0.3905, 2.0239, 0.9711, -0.1053, -0.1373, -1.1391, -0.0155, -0.1069, 0.0224, -0.1005],
    [0.9405, 0.2832, 1.9683, 0.8979, -0.0793, -0.1688, -1.1493, -0.0131, -0.0716, 0.0113, -0.0967],
    [1.0180, 0.1786, 1.9104, 0.8288, -0.0507, -0.2001, -1.1587, -0.0100, -0.0376, 0.0006, -0.0991],
    [1.0652, 0.0681, 1.8230, 0.7438, -0.0212, -0.2325, -1.1619, -0.0071, 0.0000, -0.0103, -0.1048],
    [1.0667, -0.0560, 1.6893, 0.6219, 0.0096, -0.2686, -1.1547, -0.0045, 0.0427, -0.0211, -0.1121],
    [1.0672, -0.1828, 1.5532, 0.4936, 0.0431, -0.3090, -1.1472, -0.0017, 0.0862, -0.0315, -0.1270],
    [1.0564, -0.3392, 1.4169, 0.3838, 0.0772, -0.3866, -1.1370, 0.0001, 0.1272, -0.0412, -0.1095],
    [1.0035, -0.3097, 1.2796, 0.5036, 0.1029, -0.3984, -1.1159, 0.0002, 0.1612, -0.0472, -0.0994],
    [0.9500, -0.2751, 1.1413, 0.6296, 0.1291, -0.4118, -1.0951, 0.0003, 0.1948, -0.0527, -0.0894],
    [0.8956, -0.2375, 1.0033, 0.7554, 0.1555, -0.4253, -1.0740, 0.0001, 0.2287, -0.0584, -0.0793],
    [0.8290, -0.2049, 0.8788, 0.8806, 0.1712, -0.4265, -1.0479, -0.0034, 0.2335, -0.0560, -0.0719],
    [0.7626, -0.1704, 0.7561, 1.0124, 0.1868, -0.4231, -1.0226, -0.0070, 0.2384, -0.0530, -0.0645],
    [0.6962, -0.1337, 0.6355, 1.1515, 0.2033, -0.4134, -0.9965, -0.0107, 0.2434, -0.0500, -0.0571],
    [0.6511, -0.1013, 0.5669, 1.2413, 0.2158, -0.3919, -0.9673, -0.0122, 0.2281, -0.0494, -0.0580],
    [0.6388, -0.0705, 0.3206, 1.3206, 0.3745, -0.3653, -0.9372, -0.0133, 0.2076, -0.0489, -0.1556],
];
// CLIP SPRINT_R END

// CLIP SPRINT_FR BEGIN
/// AN-02-S13: Sprint, forward right, from the Pro Rifle Pack clip "sprint forward right" (examples/fit_clip, torso,yaw,legs,height,plant). Generated by scripts/clip_table.py: 48 samples over one loop;
/// fields: hipL, hipR, kneeL, kneeR, sideL, sideR, yaw, lean, twist, tilt, bob.
pub const SPRINT_FR: [[f32; 11]; 48] = [
    [0.6668, -0.1422, 0.5589, 1.3481, 0.0915, -0.3419, -0.5101, -0.0318, 0.3123, -0.0090, -0.0954],
    [0.5885, -0.0617, 0.5581, 1.5243, 0.0781, -0.3279, -0.4755, -0.0267, 0.2963, -0.0161, -0.0733],
    [0.5095, 0.0155, 0.5493, 1.6997, 0.0608, -0.3050, -0.4400, -0.0218, 0.2801, -0.0232, -0.0552],
    [0.4465, 0.1251, 0.5673, 1.8027, 0.0388, -0.2600, -0.4088, -0.0196, 0.2355, -0.0256, -0.0445],
    [0.3900, 0.2458, 0.5945, 1.8864, 0.0128, -0.2133, -0.3786, -0.0176, 0.1841, -0.0265, -0.0394],
    [0.3361, 0.3691, 0.6235, 1.9701, -0.0172, -0.1684, -0.3483, -0.0145, 0.1317, -0.0274, -0.0390],
    [0.2844, 0.4932, 0.6536, 2.0533, -0.0516, -0.1254, -0.3174, -0.0108, 0.0802, -0.0280, -0.0433],
    [0.2345, 0.6164, 0.6844, 2.1363, -0.0910, -0.0833, -0.2864, -0.0061, 0.0277, -0.0283, -0.0523],
    [0.1856, 0.7374, 0.7146, 2.2187, -0.1359, -0.0425, -0.2558, -0.0011, -0.0247, -0.0280, -0.0660],
    [0.1216, 0.8594, 0.7216, 2.2443, -0.1820, -0.0259, -0.2455, 0.0029, -0.0860, -0.0258, -0.0820],
    [0.0298, 0.9865, 0.6892, 2.1937, -0.2219, -0.0355, -0.2661, 0.0038, -0.1595, -0.0206, -0.0984],
    [-0.0395, 1.1130, 0.6870, 2.1514, -0.1912, -0.0354, -0.2864, 0.0040, -0.2332, -0.0145, -0.0741],
    [-0.1112, 1.2218, 0.6588, 2.0996, -0.2064, -0.0293, -0.3101, 0.0044, -0.3018, -0.0087, -0.0724],
    [-0.1845, 1.2655, 0.6057, 1.9869, -0.2035, -0.0186, -0.3457, 0.0045, -0.3479, -0.0042, -0.0702],
    [-0.2578, 1.3082, 0.5517, 1.8731, -0.2008, -0.0013, -0.3809, 0.0044, -0.3943, 0.0008, -0.0681],
    [-0.3303, 1.3507, 0.4982, 1.7594, -0.1985, 0.0180, -0.4162, 0.0038, -0.4402, 0.0057, -0.0659],
    [-0.2813, 1.3203, 0.6530, 1.6019, -0.1820, 0.0395, -0.4406, 0.0080, -0.4477, 0.0075, -0.0648],
    [-0.2294, 1.2918, 0.8141, 1.4439, -0.1665, 0.0551, -0.4652, 0.0115, -0.4539, 0.0097, -0.0636],
    [-0.1773, 1.2646, 0.9734, 1.2873, -0.1530, 0.0655, -0.4891, 0.0156, -0.4610, 0.0124, -0.0624],
    [-0.1313, 1.1842, 1.0928, 1.1364, -0.1350, 0.0576, -0.5116, 0.0202, -0.4350, 0.0099, -0.0626],
    [-0.0871, 1.0875, 1.2023, 0.9815, -0.1148, 0.0435, -0.5331, 0.0250, -0.4007, 0.0066, -0.0631],
    [-0.0427, 0.9888, 1.3124, 0.8224, -0.0937, 0.0272, -0.5546, 0.0301, -0.3658, 0.0038, -0.0636],
    [0.0017, 0.8887, 1.4220, 0.6621, -0.0713, 0.0105, -0.5758, 0.0356, -0.3323, 0.0018, -0.0640],
    [0.0464, 0.7877, 1.5311, 0.5014, -0.0483, -0.0064, -0.5973, 0.0412, -0.2988, 0.0006, -0.0645],
    [0.0913, 0.6863, 1.6389, 0.3426, -0.0247, -0.0224, -0.6186, 0.0471, -0.2661, -0.0001, -0.0650],
    [0.1396, 0.6168, 1.7258, 0.3093, -0.0057, -0.0712, -0.6385, 0.0513, -0.2278, -0.0016, -0.0970],
    [0.1926, 0.6090, 1.7835, 0.4537, 0.0061, -0.0892, -0.6561, 0.0524, -0.1796, -0.0050, -0.0832],
    [0.2461, 0.6040, 1.8412, 0.6011, 0.0181, -0.1050, -0.6732, 0.0542, -0.1330, -0.0075, -0.0788],
    [0.2995, 0.5890, 1.8919, 0.7297, 0.0276, -0.1186, -0.6902, 0.0551, -0.0872, -0.0090, -0.0801],
    [0.3550, 0.5367, 1.9177, 0.7880, 0.0287, -0.1283, -0.7040, 0.0491, -0.0472, -0.0079, -0.0775],
    [0.4101, 0.4852, 1.9428, 0.8499, 0.0299, -0.1366, -0.7172, 0.0439, -0.0070, -0.0063, -0.0807],
    [0.4656, 0.4347, 1.9681, 0.9177, 0.0314, -0.1441, -0.7315, 0.0392, 0.0334, -0.0048, -0.0902],
    [0.5947, 0.3365, 1.9781, 0.8848, 0.0226, -0.1473, -0.7395, 0.0265, 0.0735, 0.0007, -0.0872],
    [0.7247, 0.2381, 1.9882, 0.8502, 0.0139, -0.1501, -0.7479, 0.0149, 0.1139, 0.0062, -0.0889],
    [0.8545, 0.1431, 1.9982, 0.8236, 0.0048, -0.1529, -0.7562, 0.0027, 0.1537, 0.0119, -0.0971],
    [0.9447, 0.0369, 1.9393, 0.7704, -0.0136, -0.1570, -0.7573, -0.0073, 0.1944, 0.0159, -0.1064],
    [1.0255, -0.0745, 1.8631, 0.7058, -0.0316, -0.1630, -0.7560, -0.0163, 0.2348, 0.0191, -0.1199],
    [1.1054, -0.1911, 1.7861, 0.6339, -0.0475, -0.1749, -0.7552, -0.0250, 0.2742, 0.0224, -0.0866],
    [1.1352, -0.2736, 1.6621, 0.5768, -0.0586, -0.1901, -0.7501, -0.0335, 0.3039, 0.0251, -0.0831],
    [1.1305, -0.3472, 1.5045, 0.5237, -0.0640, -0.2133, -0.7418, -0.0412, 0.3270, 0.0276, -0.0789],
    [1.1255, -0.4201, 1.3461, 0.4712, -0.0655, -0.2385, -0.7333, -0.0487, 0.3497, 0.0307, -0.0748],
    [1.0971, -0.4652, 1.1969, 0.4698, -0.0562, -0.2623, -0.7192, -0.0530, 0.3655, 0.0305, -0.0713],
    [1.0331, -0.4681, 1.0571, 0.5446, -0.0350, -0.2835, -0.6970, -0.0523, 0.3713, 0.0263, -0.0690],
    [0.9687, -0.4693, 0.9173, 0.6198, -0.0139, -0.3064, -0.6746, -0.0515, 0.3757, 0.0222, -0.0666],
    [0.9040, -0.4555, 0.7965, 0.7062, 0.0052, -0.3280, -0.6503, -0.0504, 0.3770, 0.0176, -0.0647],
    [0.8394, -0.3816, 0.7497, 0.8517, 0.0191, -0.3389, -0.6151, -0.0459, 0.3610, 0.0109, -0.0646],
    [0.7753, -0.3036, 0.7048, 1.0096, 0.0315, -0.3459, -0.5801, -0.0413, 0.3449, 0.0043, -0.0645],
    [0.7116, -0.2234, 0.6602, 1.1753, 0.0421, -0.3474, -0.5449, -0.0365, 0.3286, -0.0023, -0.0644],
];
// CLIP SPRINT_FR END

// CLIP HIGH_CRAWL BEGIN
/// Patch: Animations 2: the high crawl (on elbows and knees, body off the ground, rifle cradled across the forearms),
/// solved by `crawl::solve` (`cargo run -p scalia-render --example crawl_solve rows`); a test keeps them equal.
/// 32 samples over one loop; fields: hipL, hipR, kneeL, kneeR, sideL, sideR, shL, shR, elL, elR, abL, abR, reachL,
/// reachR, lean, raise, twist, head, hipRoll (yaw: lying prone it rolls the hips), rollL, rollR (thigh roll).
pub const HIGH_CRAWL: [[f32; 21]; 32] = [
    [1.8594, 1.3775, 1.8235, 1.3557, 0.0146, -0.0093, 1.1745, 2.2794, 1.8666, 1.1262, 2.1030, -0.7935, 0.0000, 0.0000, 0.1555, 0.2901, 0.0058, 1.1555, -0.0017, 0.4217, -0.4386],
    [1.8350, 1.3483, 1.8059, 1.3245, 0.0121, -0.0101, 1.1780, 2.2690, 1.8766, 1.1436, 2.0015, -0.8256, 0.0000, 0.0000, 0.1555, 0.2886, 0.0059, 1.1555, -0.0009, 0.4306, -0.4393],
    [1.8075, 1.3221, 1.7828, 1.2938, 0.0104, -0.0120, 1.1839, 2.2511, 1.8795, 1.1645, 1.9084, -0.8617, 0.0000, 0.0000, 0.1554, 0.2887, 0.0055, 1.1554, 0.0005, 0.4343, -0.4366],
    [1.7777, 1.2993, 1.7552, 1.2643, 0.0095, -0.0146, 1.1919, 2.2269, 1.8760, 1.1885, 1.8248, -0.9019, 0.0000, 0.0000, 0.1551, 0.2901, 0.0048, 1.1551, 0.0015, 0.4344, -0.4301],
    [1.7468, 1.2721, 1.7247, 1.2430, 0.0093, -0.0124, 1.1622, 2.1987, 1.9292, 1.2143, 1.7173, -0.9468, 0.0000, 0.0000, 0.1549, 0.2925, 0.0041, 1.1549, 0.0015, 0.4327, -0.4439],
    [1.7156, 1.2664, 1.6930, 1.2497, 0.0094, -0.0075, 1.1243, 2.1688, 1.9840, 1.2401, 1.6957, -0.9964, 0.0000, 0.0000, 0.1548, 0.2952, 0.0034, 1.1548, 0.0006, 0.4306, -0.4657],
    [1.6846, 1.2853, 1.6612, 1.2835, 0.0097, -0.0009, 1.0858, 2.1392, 2.0276, 1.2646, 1.7601, -1.0512, 0.0000, 0.0000, 0.1547, 0.2976, 0.0028, 1.1547, -0.0005, 0.4287, -0.4909],
    [1.6539, 1.3298, 1.6299, 1.3437, 0.0099, 0.0069, 1.0417, 2.1108, 2.0645, 1.2870, 1.8990, -1.1117, 0.0000, 0.0000, 0.1546, 0.2995, 0.0021, 1.1546, -0.0012, 0.4272, -0.5187],
    [1.6234, 1.4009, 1.5989, 1.4287, 0.0101, 0.0145, 0.9885, 2.0838, 2.0977, 1.3071, 2.0954, -1.1786, 0.0000, 0.0000, 0.1544, 0.3007, 0.0015, 1.1544, -0.0016, 0.4261, -0.5451],
    [1.5932, 1.4959, 1.5681, 1.5322, 0.0103, 0.0194, 0.9299, 2.0581, 2.1271, 1.3247, 2.3119, -1.2526, 0.0000, 0.0000, 0.1541, 0.3016, 0.0009, 1.1541, -0.0018, 0.4252, -0.5628],
    [1.5631, 1.6037, 1.5374, 1.6405, 0.0106, 0.0197, 0.8779, 2.0338, 2.1470, 1.3394, 2.5077, -1.3338, 0.0000, 0.0000, 0.1539, 0.3020, 0.0004, 1.1539, -0.0021, 0.4247, -0.5645],
    [1.5331, 1.7068, 1.5069, 1.7357, 0.0108, 0.0151, 0.8397, 2.0117, 2.1541, 1.3506, 2.6591, -1.4222, 0.0000, 0.0000, 0.1537, 0.3020, -0.0001, 1.1537, -0.0024, 0.4248, -0.5491],
    [1.5029, 1.7903, 1.4768, 1.8057, 0.0108, 0.0077, 0.8161, 1.9936, 2.1493, 1.3571, 2.7660, -1.5172, 0.0000, 0.0000, 0.1537, 0.3012, -0.0009, 1.1537, -0.0023, 0.4259, -0.5222],
    [1.4722, 1.8476, 1.4471, 1.8476, 0.0104, -0.0001, 0.8056, 1.9806, 2.1351, 1.3583, 2.8372, -1.6179, 0.0000, 0.0000, 0.1540, 0.2992, -0.0021, 1.1540, -0.0015, 0.4283, -0.4922],
    [1.4407, 1.8784, 1.4173, 1.8638, 0.0098, -0.0065, 0.8078, 1.9730, 2.1129, 1.3538, 2.8812, -1.7231, 0.0000, 0.0000, 0.1546, 0.2962, -0.0035, 1.1546, -0.0001, 0.4319, -0.4648],
    [1.4087, 1.8827, 1.3868, 1.8561, 0.0093, -0.0113, 0.8251, 1.9696, 2.0807, 1.3442, 2.9042, -1.8316, 0.0000, 0.0000, 0.1551, 0.2929, -0.0048, 1.1551, 0.0012, 0.4357, -0.4416],
    [1.3775, 1.8594, 1.3557, 1.8235, 0.0093, -0.0146, 0.8622, 1.9674, 2.0343, 1.3318, 2.9081, -1.9410, 0.0000, 0.0000, 0.1555, 0.2901, -0.0057, 1.1555, 0.0017, 0.4386, -0.4217],
    [1.3483, 1.8350, 1.3245, 1.8059, 0.0102, -0.0121, 0.8724, 1.9638, 2.0347, 1.3188, 2.8824, -2.0492, 0.0000, 0.0000, 0.1556, 0.2886, -0.0059, 1.1556, 0.0009, 0.4393, -0.4306],
    [1.3221, 1.8075, 1.2938, 1.7828, 0.0120, -0.0104, 0.8903, 1.9578, 2.0253, 1.3067, 2.8581, -2.1537, 0.0000, 0.0000, 0.1554, 0.2887, -0.0055, 1.1554, -0.0005, 0.4366, -0.4343],
    [1.2993, 1.7777, 1.2643, 1.7552, 0.0146, -0.0095, 0.9146, 1.9498, 2.0075, 1.2955, 2.8349, -2.2533, 0.0000, 0.0000, 0.1551, 0.2901, -0.0048, 1.1551, -0.0015, 0.4301, -0.4344],
    [1.2721, 1.7468, 1.2430, 1.7247, 0.0124, -0.0093, 0.9428, 1.9795, 1.9841, 1.2682, 2.8115, -2.3042, 0.0000, 0.0000, 0.1549, 0.2925, -0.0041, 1.1549, -0.0015, 0.4439, -0.4327],
    [1.2664, 1.7156, 1.2497, 1.6930, 0.0075, -0.0094, 0.9727, 2.0175, 1.9583, 1.2499, 2.7868, -2.2634, 0.0000, 0.0000, 0.1548, 0.2952, -0.0034, 1.1548, -0.0006, 0.4657, -0.4306],
    [1.2854, 1.6846, 1.2835, 1.6612, 0.0010, -0.0097, 1.0024, 2.0560, 1.9328, 1.2431, 2.7594, -2.1446, 0.0000, 0.0000, 0.1547, 0.2976, -0.0028, 1.1547, 0.0005, 0.4910, -0.4287],
    [1.3299, 1.6539, 1.3437, 1.6299, -0.0068, -0.0099, 1.0308, 2.1001, 1.9093, 1.2411, 2.7279, -1.9523, 0.0000, 0.0000, 0.1546, 0.2995, -0.0021, 1.1546, 0.0012, 0.5189, -0.4272],
    [1.4010, 1.6234, 1.4287, 1.5989, -0.0143, -0.0101, 1.0577, 2.1536, 1.8881, 1.2346, 2.6913, -1.7020, 0.0000, 0.0000, 0.1544, 0.3007, -0.0014, 1.1544, 0.0016, 0.5453, -0.4261],
    [1.4960, 1.5932, 1.5322, 1.5681, -0.0193, -0.0103, 1.0835, 2.2118, 1.8686, 1.2164, 2.6493, -1.4360, 0.0000, 0.0000, 0.1541, 0.3016, -0.0009, 1.1541, 0.0018, 0.5629, -0.4252],
    [1.6037, 1.5631, 1.6404, 1.5374, -0.0196, -0.0105, 1.1078, 2.2636, 1.8512, 1.1873, 2.6012, -1.2038, 0.0000, 0.0000, 0.1539, 0.3020, -0.0004, 1.1539, 0.0021, 0.5646, -0.4247],
    [1.7067, 1.5331, 1.7356, 1.5069, -0.0150, -0.0108, 1.1299, 2.3020, 1.8371, 1.1536, 2.5457, -1.0289, 0.0000, 0.0000, 0.1537, 0.3020, 0.0001, 1.1537, 0.0024, 0.5491, -0.4248],
    [1.7902, 1.5029, 1.8056, 1.4768, -0.0076, -0.0108, 1.1481, 2.3255, 1.8288, 1.1236, 2.4801, -0.9088, 0.0000, 0.0000, 0.1537, 0.3012, 0.0009, 1.1537, 0.0023, 0.5223, -0.4259],
    [1.8476, 1.4722, 1.8475, 1.4471, 0.0001, -0.0104, 1.1612, 2.3360, 1.8288, 1.1019, 2.4020, -0.8321, 0.0000, 0.0000, 0.1540, 0.2992, 0.0021, 1.1540, 0.0015, 0.4922, -0.4283],
    [1.8784, 1.4407, 1.8638, 1.4173, 0.0066, -0.0098, 1.1688, 2.3338, 1.8375, 1.0922, 2.3103, -0.7892, 0.0000, 0.0000, 0.1545, 0.2962, 0.0035, 1.1545, 0.0001, 0.4648, -0.4318],
    [1.8827, 1.4087, 1.8561, 1.3868, 0.0114, -0.0093, 1.1722, 2.3165, 1.8520, 1.0989, 2.2082, -0.7756, 0.0000, 0.0000, 0.1551, 0.2929, 0.0049, 1.1551, -0.0012, 0.4416, -0.4357],
];
// CLIP HIGH_CRAWL END

/// Patch: Animations 2, footsteps on contact: the phases (0..1) at which the left and right boot touch the ground in
/// `pose`'s cycle moving in `dir`, found on the posed boots themselves (the moment the lower end of the boot comes
/// down onto the ground and stays). Crawling, the knees. The game plays a step as the phase crosses one. Worked out
/// once per gait and eighth of a turn (the nearest is used); `None` for poses that do not step.
pub fn touchdowns(pose: Pose, dir: f32) -> Option<[f32; 2]> {
    static CACHE: std::sync::OnceLock<Vec<(Pose, [[f32; 2]; 8])>> = std::sync::OnceLock::new();
    let table = CACHE.get_or_init(|| {
        [Pose::Walk, Pose::Run, Pose::Sprint, Pose::CrouchWalk, Pose::Crawl]
            .iter()
            .map(|&pose| (pose, std::array::from_fn(|s| touchdowns_at(pose, s as f32 * std::f32::consts::FRAC_PI_4))))
            .collect()
    });
    let (_, sectors) = table.iter().find(|(p, _)| *p == pose)?;
    let s = (dir.rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_4).round() as usize % 8;
    Some(sectors[s])
}

fn touchdowns_at(pose: Pose, dir: f32) -> [f32; 2] {
    const N: usize = 96;
    std::array::from_fn(|i| {
        let g = if i == 0 { -1.0 } else { 1.0 };
        let low = |k: usize| {
            let p = params_dir(pose, k as f32 / N as f32, dir);
            if pose == Pose::Crawl {
                pose_rigid(Vec3::new(g * HIP_X, 0.0, KNEE_Z), if i == 0 { Part::LeftShin } else { Part::RightShin }, &p).z
            } else {
                let part = if i == 0 { Part::LeftFoot } else { Part::RightFoot };
                [HEEL_Y, TOE_Y].map(|y| pose_rigid(Vec3::new(g * HIP_X, y, 0.0), part, &p).z).into_iter().fold(f32::MAX, f32::min)
            }
        };
        let h: Vec<f32> = (0..N).map(low).collect();
        let floor = h.iter().copied().fold(f32::MAX, f32::min);
        let down: Vec<bool> = h.iter().map(|z| *z < floor + 0.01).collect();
        // Of the moments a boot comes down, the one that starts its longest stay on the ground.
        (0..N)
            .filter(|&k| down[k] && !down[(k + N - 1) % N])
            .max_by_key(|&k| (0..N).take_while(|d| down[(k + d) % N]).count())
            .map_or(0.0, |k| k as f32 / N as f32)
    })
}

/// Patch: Animations 2, arms clear of the body: the torso as an upright ellipse (half-width at the waist and the
/// chest, half-depth, the heights it spans) and the arm's own thickness. common.wgsl `clear_arms` mirrors these.
pub const TORSO_RX: [f32; 2] = [0.18, 0.22];
pub const TORSO_RY: f32 = 0.16;
pub const TORSO_Z: [f32; 2] = [0.85, 1.30];
pub const ARM_R: f32 = 0.05;

/// Swings an arm out (more `abduct`) just far enough that its elbow, forearm and hand stay outside the torso: the
/// sprint's swing and the carries brought the forearm in through the vest. Upright poses only (lying, the arms rest
/// on the ground). Points in front of or behind the torso (a rifle carried across the chest) are left alone, and so
/// is the rifle arm unless the rifle is slung (its stock sits in the shoulder).
pub fn clear_arms(p: &mut PoseParams) {
    if p.prone != 0.0 {
        return;
    }
    for i in 0..2 {
        // The rifle hand is tied to the rifle and its stock to the shoulder: it moves only with the rifle slung.
        if i == 1 && p.sling < 1.0 {
            continue;
        }
        let (x0, g) = if i == 0 { (-SHOULDER_X, -1.0) } else { (SHOULDER_X, 1.0) };
        for _ in 0..2 {
            let mut need = 0.0f32;
            for q in [Vec3::new(g * ELBOW_X, 0.0, ELBOW_Z), Vec3::new(g * ELBOW_X, 0.0, (ELBOW_Z + PALM.z) * 0.5), Vec3::new(g * PALM.x, PALM.y, PALM.z)] {
                let w = arm(q, p.shoulder[i], p.elbow[i], p.reach[i], p.abduct[i], x0);
                if w.z < TORSO_Z[0] - 0.05 || w.z > TORSO_Z[1] {
                    continue;
                }
                let t = ((w.z - TORSO_Z[0]) / (TORSO_Z[1] - TORSO_Z[0])).clamp(0.0, 1.0);
                let rx = TORSO_RX[0] + (TORSO_RX[1] - TORSO_RX[0]) * t * t * (3.0 - 2.0 * t) + ARM_R;
                let yy = w.y / (TORSO_RY + ARM_R);
                if yy.abs() >= 1.0 {
                    continue;
                }
                let out = rx * (1.0 - yy * yy).sqrt() - w.x * g;
                if out > 0.0 {
                    need = need.max(out / (SHOULDER_Z - w.z).max(0.15));
                }
            }
            p.abduct[i] += need;
        }
    }
}
