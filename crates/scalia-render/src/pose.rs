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
pub const TABLET_AT: Vec3 = Vec3::new(0.0, 0.37, 1.19);
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
fn arm(q: Vec3, shoulder: f32, elbow: f32, reach: f32, side_x: f32) -> Vec3 {
    let p = rot(rot(q, 0.0, ELBOW_Z, elbow), 0.0, SHOULDER_Z, shoulder);
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

/// Where a hand grips, in the figure's frame before lean (for tests and tuning).
pub fn hand(p: &PoseParams, side: Side) -> Vec3 {
    let (i, x) = if side == Side::Left { (0, -SHOULDER_X) } else { (1, SHOULDER_X) };
    arm(Vec3::new(x, 0.0, GRIP_Z), p.shoulder[i], p.elbow[i], p.reach[i], x)
}

fn leg(q: Vec3, hip: f32, knee: f32) -> Vec3 {
    rot(rot(q, 0.0, KNEE_Z, -knee), 0.0, HIP_Z, hip)
}

/// Gait for both legs at a phase: (hip, knee) angles. Each leg swings `swing` radians about `forward`; its knee
/// flexes most mid-swing (as it passes under the body) and a little at mid-stance, carrying the weight, so the hips
/// dip twice per stride (motion guide rule 2). The right leg is half a cycle behind the left.
fn gait(phase: f32, swing: f32, forward: f32, lift: f32, base: f32) -> ([f32; 2], [f32; 2]) {
    let mut hip = [0.0; 2];
    let mut knee = [0.0; 2];
    for (i, off) in [0.0f32, 0.5].into_iter().enumerate() {
        let a = (phase + off) * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        hip[i] = swing * s + forward;
        knee[i] = base + lift * c.max(0.0).powf(1.5) + 0.22 * (-c).max(0.0);
    }
    (hip, knee)
}

/// Pose parameters at a gait phase (0..1; only the gaits use it). Rifle carries per pose (rule 3): low ready when
/// walking, high port when running, angled down across the body when sprinting, shouldered when aiming.
pub fn params(pose: Pose, phase: f32) -> PoseParams {
    let sw = (phase * std::f32::consts::TAU).sin();
    let mut p = PoseParams::default();
    match pose {
        Pose::Stand => carry(&mut p, Carry::SafeHang),
        Pose::Walk => {
            (p.hip, p.knee) = gait(phase, 0.34, 0.0, 0.85, 0.06);
            p.lean = 0.05;
            carry(&mut p, Carry::LowReady);
        }
        Pose::Run => {
            (p.hip, p.knee) = gait(phase, 0.48, 0.08, 1.45, 0.2);
            p.lean = 0.25;
            carry(&mut p, Carry::Collapsed);
            p.head = 0.2;
        }
        Pose::Sprint => {
            (p.hip, p.knee) = gait(phase, 0.58, 0.1, 1.9, 0.26);
            p.lean = 0.42;
            carry(&mut p, Carry::Collapsed);
            // The rifle bobs with the stride.
            p.shoulder[1] += 0.04 * sw;
            p.shoulder[0] += 0.04 * sw;
            p.head = 0.38;
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
            p.reach = [0.27, 0.27];
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
    PoseParams { lean: upper.lean, shoulder: upper.shoulder, elbow: upper.elbow, reach: upper.reach, rifle: upper.rifle, head: upper.head, sling: upper.sling, ..*legs }
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
    let (shoulder, elbow, reach, rifle) = match c {
        Carry::SafeHang => ([0.65, -0.05], [0.05, 1.5], [0.85, 0.75], -1.2),
        Carry::LowReady => ([1.1, 0.2], [0.05, 1.9], [0.65, 0.5], -0.5),
        Carry::Collapsed => ([0.8, 0.0], [0.05, 1.7], [0.75, 0.6], -1.0),
        Carry::HighReady => ([1.0, 0.0], [0.95, 1.9], [0.75, 0.45], 0.85),
        Carry::ReadyUp => ([1.45, 0.5], [-0.05, 1.95], [0.6, 0.4], 0.0),
    };
    p.shoulder = shoulder;
    p.elbow = elbow;
    p.reach = reach;
    p.rifle = rifle;
}

/// Shouldered rifle: right elbow low with the grip by the chest, left arm reaching across to hold the rifle in
/// front of the magazine (angles found by search; `both_hands_hold_the_rifle_when_aiming` checks them).
fn aim_arms(p: &mut PoseParams) {
    carry(p, Carry::ReadyUp);
}

/// Vertical shift that puts the lowest heel or toe of either leg on the ground.
pub fn ground_drop(hip: [f32; 2], knee: [f32; 2]) -> f32 {
    let mut low = f32::MAX;
    for i in 0..2 {
        for y in [-0.115, 0.19] {
            low = low.min(leg(Vec3::new(0.0, y, 0.0), hip[i], knee[i]).z);
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
        Part::LeftArm => arm(v, p.shoulder[0], 0.0, p.reach[0], -SHOULDER_X),
        Part::RightArm => arm(v, p.shoulder[1], 0.0, p.reach[1], SHOULDER_X),
        Part::LeftForearm => arm(v, p.shoulder[0], p.elbow[0], p.reach[0], -SHOULDER_X),
        Part::RightForearm => arm(v, p.shoulder[1], p.elbow[1], p.reach[1], SHOULDER_X),
        Part::Rifle => {
            let h = hand(p, Side::Right);
            let held = rot(v, 0.0, GRIP_Z, p.rifle) + Vec3::new(h.x - SHOULDER_X, h.y, h.z - GRIP_Z);
            if p.sling > 0.0 { held.lerp(slung(v), p.sling) } else { held }
        }
        Part::Head => rot(v, 0.0, NECK_Z, p.head),
        Part::Torso => v,
    };
    if part != Part::LeftThigh && part != Part::RightThigh && part != Part::LeftShin && part != Part::RightShin {
        q = rot(q, 0.0, HIP_Z, -p.lean);
    }
    q.z += p.drop;
    if p.prone > 0.0 {
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2 * p.prone);
        // Mid-transition the bent knees would dip into the ground: lift a little, most at the halfway point.
        q.z += PRONE_LIFT * p.prone + 0.12 * (std::f32::consts::PI * p.prone).sin();
    }
    q
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
        prone: m(a.prone, b.prone),
        drop,
        sling: m(a.sling, b.sling),
    }
}

/// Seconds a pose change takes.
pub const BLEND_S: f32 = 0.35;
