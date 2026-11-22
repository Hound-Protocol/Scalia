//! Measurements on the posed soldier mesh: how low each part reaches (ground contact) and how deep the arms sink
//! into the torso (self-clipping). Used by tests and `examples/contact_report.rs`; nothing here runs in the game.

use crate::mesh::{figure, figure_kit, part_of, Part, Vertex};
use crate::pose::{params_dir, pose_mesh_vertex, unpose_upper, Pose, PoseParams};
use glam::Vec3;

/// Lowest height over a cycle per part group (m, negative = under the ground) and the deepest arm point inside the
/// torso (m) with the phase where it happens.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContactStats {
    pub boot: f32,
    pub knee: f32,
    pub elbow: f32,
    pub belly: f32,
    pub head: f32,
    pub rifle: f32,
    pub arm_in: f32,
    /// The same, per arm: left (support), right (rifle).
    pub arm_side: [f32; 2],
    pub arm_at: f32,
}

/// The torso's rest shape as ellipses stacked every `SLICE` metres: (centre x, centre y, radius x, radius y).
const SLICE: f32 = 0.03;
/// Arm points this close to the shoulder pivot (m) are not counted against the torso.
pub const SHOULDER_CLEAR: f32 = 0.16;

pub struct Body {
    verts: Vec<Vertex>,
    rifle: Vec<Vertex>,
    z0: f32,
    slices: Vec<Option<[f32; 4]>>,
}

impl Body {
    pub fn new() -> Body {
        let verts = figure().vertices;
        let rifle: Vec<Vertex> = if verts.iter().any(|v| part_of(v.color) == Part::Rifle) {
            Vec::new()
        } else {
            figure_kit().vertices.into_iter().filter(|v| part_of(v.color) == Part::Rifle).collect()
        };
        let torso: Vec<Vec3> = verts.iter().filter(|v| part_of(v.color) == Part::Torso).map(|v| Vec3::from(v.pos)).collect();
        let z0 = torso.iter().map(|p| p.z).fold(f32::MAX, f32::min);
        let z1 = torso.iter().map(|p| p.z).fold(f32::MIN, f32::max);
        let n = ((z1 - z0) / SLICE).ceil() as usize + 1;
        let slices = (0..n)
            .map(|k| {
                let (lo, hi) = (z0 + k as f32 * SLICE - SLICE, z0 + k as f32 * SLICE + SLICE);
                let pts: Vec<&Vec3> = torso.iter().filter(|p| p.z >= lo && p.z <= hi).collect();
                if pts.len() < 4 {
                    return None;
                }
                let (mut a, mut b) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for p in &pts {
                    a = a.min(**p);
                    b = b.max(**p);
                }
                Some([(a.x + b.x) / 2.0, (a.y + b.y) / 2.0, (b.x - a.x) / 2.0, (b.y - a.y) / 2.0])
            })
            .collect();
        Body { verts, rifle, z0, slices }
    }

    /// The fitted ellipse at height `z` (centre x, centre y, radius x, radius y), if the torso reaches it.
    pub fn slice(&self, z: f32) -> Option<[f32; 4]> {
        let k = ((z - self.z0) / SLICE).round();
        if k < 0.0 || k as usize >= self.slices.len() { None } else { self.slices[k as usize] }
    }

    /// How far a point in the torso's rest frame lies inside the torso (m, 0 outside).
    pub fn depth_in_torso(&self, q: Vec3) -> f32 {
        let k = ((q.z - self.z0) / SLICE).round();
        if k < 0.0 || k as usize >= self.slices.len() {
            return 0.0;
        }
        let Some([cx, cy, rx, ry]) = self.slices[k as usize] else { return 0.0 };
        let r = (((q.x - cx) / rx).powi(2) + ((q.y - cy) / ry).powi(2)).sqrt();
        if r >= 1.0 { 0.0 } else { (1.0 - r) * rx.min(ry) }
    }

    /// Lowest heights and arm penetration for one pose.
    pub fn stats(&self, p: &PoseParams) -> ContactStats {
        let mut s = ContactStats { boot: f32::MAX, knee: f32::MAX, elbow: f32::MAX, belly: f32::MAX, head: f32::MAX, rifle: f32::MAX, ..Default::default() };
        for v in self.verts.iter().chain(self.rifle.iter()) {
            let part = part_of(v.color);
            let at = Vec3::from(v.pos);
            let w = pose_mesh_vertex(at, v.color, p);
            let slot = match part {
                Part::LeftFoot | Part::RightFoot => &mut s.boot,
                Part::LeftShin | Part::RightShin | Part::LeftThigh | Part::RightThigh => &mut s.knee,
                Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm => &mut s.elbow,
                Part::Torso | Part::Body => &mut s.belly,
                Part::Head => &mut s.head,
                Part::Rifle => &mut s.rifle,
                _ => continue,
            };
            *slot = slot.min(w.z);
            // Arms against the torso: arm points clear of it at rest, away from the shoulder (the shoulder turns into
            // the torso on every forward swing, hidden by the seam blend, as is the shoulder cap above the pivot; the
            // clipping that shows is further down).
            let arm = matches!(part, Part::LeftArm | Part::RightArm | Part::LeftForearm | Part::RightForearm);
            let shoulder = Vec3::new(crate::pose::SHOULDER_X * at.x.signum(), 0.0, crate::pose::SHOULDER_Z);
            if arm && self.depth_in_torso(at) == 0.0 && at.distance(shoulder) > SHOULDER_CLEAR && at.z < crate::pose::SHOULDER_Z {
                let d = self.depth_in_torso(unpose_upper(w, p));
                s.arm_in = s.arm_in.max(d);
                let i = if matches!(part, Part::LeftArm | Part::LeftForearm) { 0 } else { 1 };
                s.arm_side[i] = s.arm_side[i].max(d);
            }
        }
        s
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::new()
    }
}

/// `Body::stats` over `n` phases of a pose's cycle: the lowest of each height and the deepest arm penetration.
pub fn contact_stats(pose: Pose, dir: f32, n: usize) -> ContactStats {
    let body = Body::new();
    let mut out = ContactStats { boot: f32::MAX, knee: f32::MAX, elbow: f32::MAX, belly: f32::MAX, head: f32::MAX, rifle: f32::MAX, ..Default::default() };
    for k in 0..n {
        let phase = k as f32 / n as f32;
        let s = body.stats(&params_dir(pose, phase, dir));
        out.boot = out.boot.min(s.boot);
        out.knee = out.knee.min(s.knee);
        out.elbow = out.elbow.min(s.elbow);
        out.belly = out.belly.min(s.belly);
        out.head = out.head.min(s.head);
        out.rifle = out.rifle.min(s.rifle);
        out.arm_side = [out.arm_side[0].max(s.arm_side[0]), out.arm_side[1].max(s.arm_side[1])];
        if s.arm_in > out.arm_in {
            out.arm_in = s.arm_in;
            out.arm_at = phase;
        }
    }
    out
}
