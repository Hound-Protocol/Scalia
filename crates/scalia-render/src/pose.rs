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
    // Unknowns: left shoulder, elbow and reach.
    let apply = |q: &PoseParams, x: [f32; 3]| {
        let mut t = *q;
        t.shoulder[0] += x[0];
        t.elbow[0] += x[1];
        t.reach[0] += x[2];
        t
    };
    let res = |q: &PoseParams| held(RIFLE_GUARD, q) - hand(q, Side::Left);
    // Out of reach even after the tilt (going prone, say): damp harder the further it is, so the arm only
    // stretches towards it instead of flailing.
    let excess = (held(RIFLE_GUARD, p).distance(Vec3::new(-SHOULDER_X, 0.0, SHOULDER_Z)) - GRIP_REACH).max(0.0);
    let damp = GRIP_DAMP + 2.0 * excess;
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
    let cap = |d: f32| GRIP_MAX * (d / GRIP_MAX).tanh() * w;
    let mut out = *p;
    out.shoulder[0] += cap(q.shoulder[0] - p.shoulder[0]);
    out.elbow[0] += cap(q.elbow[0] - p.elbow[0]);
    out.reach[0] += cap(q.reach[0] - p.reach[0]);
    out
}
const GRIP_STEPS: usize = 6;
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

/// 1 for poses that hold the rifle with both hands (left palm on the handguard), else 0.
pub fn two_handed(pose: Pose) -> f32 {
    match pose {
        Pose::Stand | Pose::Walk | Pose::Run | Pose::Sprint | Pose::Crouch | Pose::CoverAim | Pose::Cover | Pose::StandAim => 1.0,
        _ => 0.0,
    }
}

fn leg(q: Vec3, hip: f32, knee: f32) -> Vec3 {
    rot(rot(q, 0.0, KNEE_Z, -knee), 0.0, HIP_Z, hip)
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

/// A foot point: ankle bend, then the leg.
fn foot(q: Vec3, hip: f32, knee: f32) -> Vec3 {
    leg(rot(q, 0.0, ANKLE_Z, ankle(hip, knee)), hip, knee)
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
        _ => 1.0,
    }
}
pub const WALK_STRIDE: f32 = 1.21;
pub const RUN_STRIDE: f32 = 2.12;
pub const SPRINT_STRIDE: f32 = 2.84;

/// Pose parameters at a gait phase (0..1; only the gaits use it). Rifle carries per pose (rule 3): low ready when
/// walking, high port when running, angled down across the body when sprinting, shouldered when aiming.
pub fn params(pose: Pose, phase: f32) -> PoseParams {
    let mut p = PoseParams::default();
    match pose {
        Pose::Stand => carry(&mut p, Carry::SafeHang),
        Pose::Walk => {
            (p.hip, p.knee) = gait(phase, 0.45, 0.0, 0.75, 0.06, 0.6);
            p.lean = 0.05;
            carry(&mut p, Carry::LowReady);
            stride(&mut p, phase, 0.018, 0.06, 0.035, 0.025);
        }
        Pose::Run => {
            (p.hip, p.knee) = gait(phase, 0.6, 0.08, 1.15, 0.15, 0.45);
            p.lean = 0.25;
            carry(&mut p, Carry::Collapsed);
            p.head = 0.2;
            stride(&mut p, phase, 0.03, 0.1, 0.05, 0.04);
        }
        Pose::Sprint => {
            (p.hip, p.knee) = gait(phase, 0.64, 0.1, 1.45, 0.2, 0.36);
            p.lean = 0.42;
            carry(&mut p, Carry::Collapsed);
            p.head = 0.38;
            stride(&mut p, phase, 0.035, 0.12, 0.03, 0.05);
        }
        Pose::Crouch => {
            kneel(&mut p);
            carry(&mut p, Carry::LowReady);
            p.head = 0.35;
        }
        Pose::CoverAim => {
            kneel(&mut p);
            aim_arms(&mut p);
            p.head = 0.05;
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
        Pose::Prone => {
            p.prone = 1.0;
            p.shoulder = [2.8, 2.8];
            p.elbow = [1.3, 1.35];
            p.rifle = std::f32::consts::FRAC_PI_2;
            p.head = 1.0;
        }
        Pose::StandAim => {
            aim_arms(&mut p);
            p.head = -0.1;
        }
    }
    p.drop = ground_drop(p.hip, p.knee);
    p
}

/// Two layers (motion guide rule 1): the legs of one pose (gait, crouch) under the upper body of another (aim,
/// carry), so a soldier can walk while aiming.
pub fn layer(legs: &PoseParams, upper: &PoseParams) -> PoseParams {
    PoseParams { lean: upper.lean, shoulder: upper.shoulder, elbow: upper.elbow, reach: upper.reach, rifle: upper.rifle, rifle_yaw: upper.rifle_yaw, twist: legs.twist + upper.twist, head: upper.head, sling: upper.sling, look: upper.look, abduct: upper.abduct, ..*legs }
}

/// The life a stride puts into the body: a rise and fall twice per cycle, the upper body turning against the legs,
/// and the rifle swaying with each footfall (both arms together, so the hands stay on it).
fn stride(p: &mut PoseParams, phase: f32, bob: f32, twist: f32, arms: f32, rifle: f32) {
    let t = phase * std::f32::consts::TAU;
    p.bob = bob * (2.0 * t).cos();
    p.twist = twist * t.sin();
    let sway = (2.0 * t + 0.5).sin();
    p.shoulder[0] += arms * sway;
    p.shoulder[1] += arms * sway;
    p.rifle += rifle * (2.0 * t).sin();
}

/// Idle life for a soldier standing still (`t` seconds, `seed` 0..1 per soldier, so a squad is never in step):
/// breathing, a slow shift of weight, glances around, and now and then a hitch of the rifle. Moving soldiers get
/// their life from the stride instead; aiming soldiers keep their eyes and rifle on the target.
pub fn idle(p: &PoseParams, pose: Pose, t: f32, seed: f32) -> PoseParams {
    let mut q = *p;
    if matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint | Pose::Prone) {
        return q;
    }
    let tau = std::f32::consts::TAU;
    let breathe = (tau * (t / 3.8 + seed)).sin();
    if matches!(pose, Pose::StandAim | Pose::CoverAim) {
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
    /// Shooting: shouldered and level.
    ReadyUp,
}

pub fn carry(p: &mut PoseParams, c: Carry) {
    let (shoulder, elbow, reach, abduct, rifle, rifle_yaw) = match c {
        // Solved by examples/carry_ik: right hand on the pistol grip on the right side of the chest, the rifle
        // turned across the body (stock out to the right shoulder, muzzle to the left), left hand reaching
        // across to the handguard, elbows bent and out to the side, arms clear of the torso.
        Carry::SafeHang => ([0.32, 0.01], [0.48, 2.19], [1.40, 0.49], [0.67, 0.32], -1.20, 0.75),
        Carry::LowReady => ([0.78, -0.10], [0.47, 1.98], [0.24, 0.80], [-0.01, 0.76], -0.50, 0.75),
        Carry::Collapsed => ([0.64, -0.10], [0.45, 2.05], [0.69, 0.82], [0.46, 0.74], -0.75, 0.75),
        Carry::HighReady => ([1.18, 0.23], [0.86, 1.49], [0.41, 0.37], [0.20, 0.39], 0.85, 0.75),
        Carry::ReadyUp => ([1.22, 0.37], [0.46, 1.89], [0.20, 0.40], [-0.09, 0.92], 0.00, 0.75),
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
    let mut low = f32::MAX;
    for i in 0..2 {
        for y in [HEEL_Y, TOE_Y] {
            low = low.min(foot(Vec3::new(0.0, y, 0.0), hip[i], knee[i]).z);
        }
    }
    -low
}

/// Moves one vertex of a figure part into the pose (in the figure's frame, before heading and position).
pub fn pose_vertex(v: Vec3, part: Part, p: &PoseParams) -> Vec3 {
    let mut q = match part {
        Part::Body | Part::Sway => return v,
        Part::LeftThigh => rot(v, 0.0, HIP_Z, p.hip[0]),
        Part::RightThigh => rot(v, 0.0, HIP_Z, p.hip[1]),
        Part::LeftShin => leg(v, p.hip[0], p.knee[0]),
        Part::RightShin => leg(v, p.hip[1], p.knee[1]),
        Part::LeftFoot => foot(v, p.hip[0], p.knee[0]),
        Part::RightFoot => foot(v, p.hip[1], p.knee[1]),
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
    }
    q.z += p.drop + p.bob;
    if p.prone > 0.0 {
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2 * p.prone);
        // Mid-transition the bent knees would dip into the ground: lift a little, most at the halfway point.
        q.z += PRONE_LIFT * p.prone + 0.12 * (std::f32::consts::PI * p.prone).sin();
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
    let drop = if w <= 0.0 { a.drop } else if w >= 1.0 { b.drop } else { ground_drop(hip, knee) };
    PoseParams {
        lean: m(a.lean, b.lean),
        hip,
        knee,
        shoulder: m2(a.shoulder, b.shoulder),
        elbow: m2(a.elbow, b.elbow),
        reach: m2(a.reach, b.reach),
        head: m(a.head, b.head),
        rifle: m(a.rifle, b.rifle),
        rifle_yaw: m(a.rifle_yaw, b.rifle_yaw),
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
