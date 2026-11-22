//! Procedural animation primitives: a second-order spring and an analytic two-bone IK with a pole.
//! No allocation; safe to call per soldier per frame. Presentation only: nothing here feeds the battle.

use glam::Vec3;

/// Longest step the spring integrates at once, so a stalled frame or a tab switch cannot blow it up.
pub const MAX_DT: f32 = 0.05;

/// Second-order dynamics (frequency `f` in Hz, damping `z`, response `r`): `z < 1` overshoots,
/// `z = 1` settles without overshoot, `r < 0` anticipates by first moving the other way.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    k1: f32,
    k2: f32,
    k3: f32,
    xp: f32,
    pub y: f32,
    pub yd: f32,
}

impl Spring {
    pub fn new(f: f32, z: f32, r: f32, x0: f32) -> Spring {
        let w = std::f32::consts::TAU * f;
        Spring { k1: 2.0 * z / w, k2: 1.0 / (w * w), k3: r * z / w, xp: x0, y: x0, yd: 0.0 }
    }

    /// Moves toward `x`, estimating the target's velocity from its last value.
    pub fn step(&mut self, x: f32, dt: f32) -> f32 {
        let dt = dt.clamp(1e-5, MAX_DT);
        let xd = (x - self.xp) / dt;
        self.xp = x;
        let k2 = self.k2.max(dt * dt / 2.0 + dt * self.k1 / 2.0).max(dt * self.k1);
        self.yd += dt * (x + self.k3 * xd - self.y - self.k1 * self.yd) / k2;
        self.y += dt * self.yd;
        self.y
    }

    /// Jumps to `x` with no motion (spawn, teleport, pose cut).
    pub fn reset(&mut self, x: f32) {
        self.xp = x;
        self.y = x;
        self.yd = 0.0;
    }
}

/// Middle joint (knee or elbow) of a two-bone chain from `root` toward `target`, bending toward `pole`.
/// The end lands on `target` when it is in reach; otherwise the chain points straight at it.
pub fn two_bone(root: Vec3, target: Vec3, upper: f32, lower: f32, pole: Vec3) -> Vec3 {
    let to = target - root;
    if to.length() >= upper + lower {
        return root + to.normalize() * upper;
    }
    let d = to.length().clamp(1e-5, upper + lower - 1e-5).max((upper - lower).abs() + 1e-5);
    let axis = if to.length() > 1e-6 { to / to.length() } else { Vec3::NEG_Z };
    let side = pole - axis * pole.dot(axis);
    let side = if side.length() > 1e-6 { side.normalize() } else { axis.any_orthonormal_vector() };
    let a = (upper * upper + d * d - lower * lower) / (2.0 * d);
    let h = (upper * upper - a * a).max(0.0).sqrt();
    root + axis * a + side * h
}
