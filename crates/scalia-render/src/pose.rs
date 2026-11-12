//! Figure poses: the reference implementation of what the vertex shader does (`wgsl/common.wgsl` ports it).
//! A figure faces +y. Angles rotate in the y-z plane; positive swings a hanging limb forward.

use glam::Vec3;

use crate::mesh::Part;

pub const HIP_Z: f32 = 0.88;
pub const KNEE_Z: f32 = 0.46;
pub const SHOULDER_Z: f32 = 1.40;
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
    /// Rifle pitch around the grip (positive points the muzzle up).
    pub rifle: f32,
    pub prone: bool,
    /// Vertical shift that keeps the lowest foot point on the ground.
    pub drop: f32,
}

/// Rotates `q` in the y-z plane around (pivot_y, pivot_z).
pub fn rot(q: Vec3, pivot_y: f32, pivot_z: f32, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    let (dy, dz) = (q.y - pivot_y, q.z - pivot_z);
    Vec3::new(q.x, pivot_y + dy * c - dz * s, pivot_z + dy * s + dz * c)
}

fn leg(q: Vec3, hip: f32, knee: f32) -> Vec3 {
    rot(rot(q, 0.0, KNEE_Z, -knee), 0.0, HIP_Z, hip)
}

/// Pose parameters at a gait phase (0..1; only walking and running use it).
pub fn params(pose: Pose, phase: f32) -> PoseParams {
    let sw = (phase * std::f32::consts::TAU).sin();
    let (l, r) = (sw, -sw);
    let mut p = PoseParams::default();
    match pose {
        Pose::Stand => {}
        Pose::Walk => {
            p.hip = [0.45 * l, 0.45 * r];
            p.knee = [0.8 * l.max(0.0) + 0.05, 0.8 * r.max(0.0) + 0.05];
            p.shoulder = [-0.4 * l, 0.2 - 0.08 * r];
        }
        Pose::Run => {
            p.lean = 0.3;
            p.hip = [0.75 * l + 0.1, 0.75 * r + 0.1];
            p.knee = [1.3 * l.max(0.0) + 0.3, 1.3 * r.max(0.0) + 0.3];
            p.shoulder = [0.6 - 0.35 * l, 0.9 - 0.1 * r];
        }
        Pose::Crouch => {
            p.lean = 0.45;
            p.hip = [1.4, 1.4];
            p.knee = [2.4, 2.4];
            p.shoulder = [0.3, 0.3];
        }
        Pose::CoverAim => {
            p.lean = 0.2;
            p.hip = [1.4, 1.4];
            p.knee = [2.4, 2.4];
            p.shoulder = [1.3, 1.45];
        }
        Pose::Prone => {
            p.prone = true;
            p.shoulder = [3.05, 3.05];
            p.rifle = std::f32::consts::FRAC_PI_2;
        }
        Pose::StandAim => p.shoulder = [1.3, 1.45],
    }
    // Lowest heel or toe of either leg goes to the ground.
    let mut low = f32::MAX;
    for i in 0..2 {
        for y in [-0.10, 0.14] {
            low = low.min(leg(Vec3::new(0.0, y, 0.0), p.hip[i], p.knee[i]).z);
        }
    }
    p.drop = -low;
    p
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
        Part::Rifle => {
            let hand = rot(Vec3::new(0.0, 0.0, GRIP_Z), 0.0, SHOULDER_Z, p.shoulder[1]);
            rot(v, 0.0, GRIP_Z, p.rifle) + Vec3::new(0.0, hand.y, hand.z - GRIP_Z)
        }
        Part::Torso => v,
    };
    if matches!(part, Part::Torso | Part::LeftArm | Part::RightArm | Part::Rifle) {
        q = rot(q, 0.0, HIP_Z, -p.lean);
    }
    q.z += p.drop;
    if p.prone {
        q = rot(q, 0.0, 0.0, -std::f32::consts::FRAC_PI_2);
        q.z += PRONE_LIFT;
    }
    q
}
