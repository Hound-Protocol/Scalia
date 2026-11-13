//! The soldier's skeleton: our rig's hinges as real bones, so a skinned mesh can bend smoothly at the joints.
//!
//! Every bone's rest frame is aligned with the figure (facing +y, z up, arms hanging down), so the pose model's
//! angles (`pose::PoseParams`) map straight onto bone rotations. `skin_matrices` returns, per bone, the transform
//! from a rest-pose vertex to its posed position: a vertex bound fully to one bone lands exactly where
//! `pose::pose_vertex` puts it (the tests check this for every part), and vertices shared between bones blend.

use crate::mesh::Part;
use crate::pose::{ankle_flat, ANKLE_Z, hand, slung, PoseParams, Side, ELBOW_Z, HIP_Z, RIFLE_GRIP, KNEE_Z, NECK_Z, PRONE_LIFT, SHOULDER_X, SHOULDER_Z};
use glam::{Affine3A, Mat3A, Vec3, Vec3A};

/// The bones, in the order of `skin_matrices`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Bone {
    /// Pelvis: only the whole-body moves (stance height, bob, going prone).
    Root = 0,
    /// Everything above the hips: leans and turns.
    Spine = 1,
    Head = 2,
    LeftUpperArm = 3,
    RightUpperArm = 4,
    /// Forearm and hand.
    LeftForearm = 5,
    RightForearm = 6,
    LeftThigh = 7,
    RightThigh = 8,
    /// Shin and foot.
    LeftShin = 9,
    RightShin = 10,
    Rifle = 11,
    /// Boots: follow the shin, plus the ankle bend that keeps them level.
    LeftFoot = 12,
    RightFoot = 13,
}

pub const BONES: usize = 14;

/// The bone a rigid part of the old rig moves with.
pub fn bone_of(part: Part) -> Bone {
    match part {
        Part::Body | Part::Sway | Part::Glow => Bone::Root,
        Part::Torso => Bone::Spine,
        Part::Head => Bone::Head,
        Part::LeftArm => Bone::LeftUpperArm,
        Part::RightArm => Bone::RightUpperArm,
        Part::LeftForearm => Bone::LeftForearm,
        Part::RightForearm => Bone::RightForearm,
        Part::LeftThigh => Bone::LeftThigh,
        Part::RightThigh => Bone::RightThigh,
        Part::LeftShin => Bone::LeftShin,
        Part::RightShin => Bone::RightShin,
        Part::Rifle => Bone::Rifle,
        Part::LeftFoot => Bone::LeftFoot,
        Part::RightFoot => Bone::RightFoot,
    }
}

/// Rotation about the x axis (pitch) through the line y = 0, z = `pivot_z`.
fn pitch_at(pivot_z: f32, a: f32) -> Affine3A {
    let p = Vec3::new(0.0, 0.0, pivot_z);
    Affine3A::from_translation(p) * Affine3A::from_rotation_x(a) * Affine3A::from_translation(-p)
}

/// An arm bone's transform: elbow bend (forearm only), shoulder swing, outward raise, inward turn.
fn arm_xform(p: &PoseParams, i: usize, with_elbow: bool) -> Affine3A {
    let sx = if i == 0 { -SHOULDER_X } else { SHOULDER_X };
    let g = sx.signum();
    let s = Vec3::new(sx, 0.0, SHOULDER_Z);
    let shoulder = Affine3A::from_translation(s)
        * Affine3A::from_rotation_z(p.reach[i] * g)
        * Affine3A::from_rotation_y(-g * p.abduct[i])
        * Affine3A::from_rotation_x(p.shoulder[i])
        * Affine3A::from_translation(-s);
    if with_elbow { shoulder * pitch_at(ELBOW_Z, p.elbow[i]) } else { shoulder }
}

/// Whole-body placement: stance height and stride bob, then going prone (rotated flat and lifted).
fn posture(p: &PoseParams) -> Affine3A {
    let up = Affine3A::from_translation(Vec3::new(0.0, 0.0, p.drop + p.bob));
    if p.prone > 0.0 {
        let lift = PRONE_LIFT * p.prone + 0.12 * (std::f32::consts::PI * p.prone).sin();
        Affine3A::from_translation(Vec3::new(0.0, 0.0, lift)) * pitch_at(0.0, -std::f32::consts::FRAC_PI_2 * p.prone) * up
    } else {
        up
    }
}

/// Elementwise blend of two affine maps: blending the points they produce, exactly (used for the rifle sling).
fn lerp_affine(a: Affine3A, b: Affine3A, t: f32) -> Affine3A {
    Affine3A {
        matrix3: Mat3A::from_cols(
            a.matrix3.x_axis.lerp(b.matrix3.x_axis, t),
            a.matrix3.y_axis.lerp(b.matrix3.y_axis, t),
            a.matrix3.z_axis.lerp(b.matrix3.z_axis, t),
        ),
        translation: a.translation.lerp(b.translation, t),
    }
}

/// `slung` as an affine map (it is one: rotations then a translation), recovered from four points.
fn slung_affine() -> Affine3A {
    let o = slung(Vec3::ZERO);
    let (x, y, z) = (slung(Vec3::X) - o, slung(Vec3::Y) - o, slung(Vec3::Z) - o);
    Affine3A { matrix3: Mat3A::from_cols(Vec3A::from(x), Vec3A::from(y), Vec3A::from(z)), translation: Vec3A::from(o) }
}

/// Per bone, the map from a rest-pose vertex to where the pose puts it.
pub fn skin_matrices(p: &PoseParams) -> [Affine3A; BONES] {
    let body = posture(p);
    // Above the hips: turned, then leaned forward around the hip line.
    let spine = body * pitch_at(HIP_Z, -p.lean) * Affine3A::from_rotation_z(p.twist);
    let head = spine * Affine3A::from_rotation_z(p.look) * pitch_at(NECK_Z, p.head);
    let h = hand(p, Side::Right);
    let g = Vec3::new(0.0, RIFLE_GRIP.y, RIFLE_GRIP.z);
    // Pitch around the grip's x line, then turn around the vertical through the grip, then into the palm.
    let held = Affine3A::from_translation(h) * Affine3A::from_rotation_z(p.rifle_yaw) * Affine3A::from_translation(-RIFLE_GRIP)
        * Affine3A::from_translation(g) * Affine3A::from_rotation_x(p.rifle) * Affine3A::from_translation(-g);
    let rifle = if p.sling > 0.0 { lerp_affine(held, slung_affine(), p.sling) } else { held };
    let thigh = |i: usize| body * pitch_at(HIP_Z, p.hip[i]);
    let shin = |i: usize| body * pitch_at(HIP_Z, p.hip[i]) * pitch_at(KNEE_Z, -p.knee[i]);
    let foot = |i: usize| shin(i) * pitch_at(ANKLE_Z, ankle_flat(p.hip[i], p.knee[i], p.flat));
    [
        body,
        spine,
        head,
        spine * arm_xform(p, 0, false),
        spine * arm_xform(p, 1, false),
        spine * arm_xform(p, 0, true),
        spine * arm_xform(p, 1, true),
        thigh(0),
        thigh(1),
        shin(0),
        shin(1),
        spine * rifle,
        foot(0),
        foot(1),
    ]
}

/// Where the skeleton's joints are at rest (figure frame): pelvis, hips, knees, shoulders, elbows, neck, grip.
pub fn rest_joint(b: Bone) -> Vec3 {
    match b {
        Bone::Root | Bone::Spine => Vec3::new(0.0, 0.0, HIP_Z),
        Bone::Head => Vec3::new(0.0, 0.0, NECK_Z),
        Bone::LeftUpperArm => Vec3::new(-SHOULDER_X, 0.0, SHOULDER_Z),
        Bone::RightUpperArm => Vec3::new(SHOULDER_X, 0.0, SHOULDER_Z),
        Bone::LeftForearm => Vec3::new(-SHOULDER_X, 0.0, ELBOW_Z),
        Bone::RightForearm => Vec3::new(SHOULDER_X, 0.0, ELBOW_Z),
        Bone::LeftThigh => Vec3::new(-0.105, 0.0, HIP_Z),
        Bone::RightThigh => Vec3::new(0.105, 0.0, HIP_Z),
        Bone::LeftShin => Vec3::new(-0.105, 0.0, KNEE_Z),
        Bone::RightShin => Vec3::new(0.105, 0.0, KNEE_Z),
        Bone::Rifle => RIFLE_GRIP,
        Bone::LeftFoot => Vec3::new(-0.105, 0.0, ANKLE_Z),
        Bone::RightFoot => Vec3::new(0.105, 0.0, ANKLE_Z),
    }
}

/// The matrices as the GPU takes them: three rows (x, y, z) of a 3x4 affine per bone.
pub fn gpu_rows(m: &[Affine3A; BONES], out: &mut Vec<f32>) {
    for a in m {
        let (c, t) = (a.matrix3, a.translation);
        out.extend_from_slice(&[c.x_axis.x, c.y_axis.x, c.z_axis.x, t.x]);
        out.extend_from_slice(&[c.x_axis.y, c.y_axis.y, c.z_axis.y, t.y]);
        out.extend_from_slice(&[c.x_axis.z, c.y_axis.z, c.z_axis.z, t.z]);
    }
}
