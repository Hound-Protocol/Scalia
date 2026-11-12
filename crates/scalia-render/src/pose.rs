//! Figure poses: the reference implementation of what the vertex shader does (`wgsl/common.wgsl` ports it).
//! A figure faces +y. Angles rotate in the y-z plane; positive swings a hanging limb forward.

use glam::Vec3;

use crate::mesh::Part;

pub const HIP_Z: f32 = 0.88;
pub const KNEE_Z: f32 = 0.46;
pub const SHOULDER_Z: f32 = 1.40;
pub const ELBOW_Z: f32 = 1.14;
pub const NECK_Z: f32 = 1.46;
/// Where the right hand grips the rifle in the rest pose.
pub const GRIP_Z: f32 = 0.95;
/// How far the lying body is lifted so the chest rests on the ground.
pub const PRONE_LIFT: f32 = 0.15;

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
}

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
    /// Head nod around the neck (positive tilts the face up).
    pub head: f32,
    /// Rifle pitch around the grip (positive points the muzzle up).
    pub rifle: f32,
    /// 0 upright, 1 lying flat; fractions only while blending.
    pub prone: f32,
    /// Vertical shift that keeps the lowest foot point on the ground.
    pub drop: f32,
}

/// Rotates `q` in the y-z plane around (pivot_y, pivot_z).
pub fn rot(q: Vec3, pivot_y: f32, pivot_z: f32, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    let (dy, dz) = (q.y - pivot_y, q.z - pivot_z);
    Vec3::new(q.x, pivot_y + dy * c - dz * s, pivot_z + dy * s + dz * c)
}

fn arm(q: Vec3, shoulder: f32, elbow: f32) -> Vec3 {
    rot(rot(q, 0.0, ELBOW_Z, elbow), 0.0, SHOULDER_Z, shoulder)
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
        Pose::Stand => {}
        Pose::Walk => {
            (p.hip, p.knee) = gait(phase, 0.40, 0.0, 0.85, 0.06);
            p.lean = 0.05;
            p.shoulder = [0.45 - 0.08 * sw, 0.35];
            p.elbow = [0.9, 1.0];
            p.rifle = -0.35;
        }
        Pose::Run => {
            (p.hip, p.knee) = gait(phase, 0.62, 0.12, 1.35, 0.18);
            p.lean = 0.25;
            p.shoulder = [0.75 - 0.12 * sw, 0.7];
            p.elbow = [1.55, 1.5];
            p.rifle = 0.7;
            p.head = 0.2;
        }
        Pose::Sprint => {
            (p.hip, p.knee) = gait(phase, 0.85, 0.18, 1.75, 0.22);
            p.lean = 0.42;
            p.shoulder = [0.9 - 0.45 * sw, 0.35 + 0.1 * sw];
            p.elbow = [1.5, 1.3];
            p.rifle = -0.6;
            p.head = 0.38;
        }
        Pose::Crouch => {
            p.lean = 0.45;
            p.hip = [1.4, 1.4];
            p.knee = [2.4, 2.4];
            p.shoulder = [0.3, 0.3];
            p.elbow = [0.9, 0.9];
            p.rifle = 0.5;
            p.head = 0.35;
        }
        Pose::CoverAim => {
            p.lean = 0.2;
            p.hip = [1.4, 1.4];
            p.knee = [2.4, 2.4];
            p.shoulder = [1.1, 1.2];
            p.elbow = [0.4, 0.6];
            p.head = 0.15;
        }
        Pose::Prone => {
            p.prone = 1.0;
            p.shoulder = [2.8, 2.8];
            p.elbow = [1.3, 1.35];
            p.rifle = std::f32::consts::FRAC_PI_2;
            p.head = 1.0;
        }
        Pose::StandAim => {
            p.shoulder = [1.1, 1.2];
            p.elbow = [0.4, 0.6];
        }
    }
    p.drop = ground_drop(p.hip, p.knee);
    p
}

/// Vertical shift that puts the lowest heel or toe of either leg on the ground.
pub fn ground_drop(hip: [f32; 2], knee: [f32; 2]) -> f32 {
    let mut low = f32::MAX;
    for i in 0..2 {
        for y in [-0.10, 0.14] {
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
        Part::LeftArm => rot(v, 0.0, SHOULDER_Z, p.shoulder[0]),
        Part::RightArm => rot(v, 0.0, SHOULDER_Z, p.shoulder[1]),
        Part::LeftForearm => arm(v, p.shoulder[0], p.elbow[0]),
        Part::RightForearm => arm(v, p.shoulder[1], p.elbow[1]),
        Part::Rifle => {
            let hand = arm(Vec3::new(0.0, 0.0, GRIP_Z), p.shoulder[1], p.elbow[1]);
            rot(v, 0.0, GRIP_Z, p.rifle) + Vec3::new(0.0, hand.y, hand.z - GRIP_Z)
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
    // Eased: pose changes start and settle gently (motion guide rule 5).
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
        head: m(a.head, b.head),
        rifle: m(a.rifle, b.rifle),
        prone: m(a.prone, b.prone),
        drop,
    }
}

/// Seconds a pose change takes.
pub const BLEND_S: f32 = 0.35;
