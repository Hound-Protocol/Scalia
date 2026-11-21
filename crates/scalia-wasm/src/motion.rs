//! Footwork with momentum: a body on foot cannot change its velocity faster than its feet can push. Quick and
//! sharp at a jog; at a sprint the grip thins, so turns bend round in an arc, a reversal is a cut (plant, pivot,
//! push off), stopping dead is a short skid, and easing off from a sprint to a jog is a gradual slow-down.

/// How a body's feet work on the ground (m/s and m/s²).
#[derive(Copy, Clone, Debug)]
pub struct Footing {
    /// Speeding up to `cruise`.
    pub push: f32,
    /// Speeding up past `cruise` (towards a sprint): slower, a build-up.
    pub kick: f32,
    /// Stopping, or turning against the motion.
    pub brake: f32,
    /// Easing off while still going the same way (a sprint settling to a jog): gentle.
    pub coast: f32,
    /// The everyday pace (a jog) up to which `push` applies.
    pub cruise: f32,
    /// Top speed (a sprint): the grip thins towards it.
    pub top: f32,
}

/// One step of a runner's velocity (m/s, x and y) towards `want` (where he is trying to go, at the pace he
/// wants) over `dt` seconds.
pub fn step_velocity(vel: (f32, f32), want: (f32, f32), dt: f32, f: Footing) -> (f32, f32) {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let wanted = (want.0 * want.0 + want.1 * want.1).sqrt();
    let (dx, dy) = (want.0 - vel.0, want.1 - vel.1);
    let need = (dx * dx + dy * dy).sqrt();
    if need < 1e-5 {
        return want;
    }
    let aligned = speed > 1e-3 && wanted > 1e-3 && (vel.0 * want.0 + vel.1 * want.1) / (speed * wanted) > 0.9;
    let rate = if aligned && wanted < speed {
        f.coast
    } else if speed > 1e-3 && (dx * vel.0 + dy * vel.1) < 0.0 {
        f.brake
    } else if speed >= f.cruise {
        f.kick
    } else {
        f.push
    };
    // Grip thins with speed past the cruise: fast turns are wide.
    let over = ((speed - f.cruise) / (f.top - f.cruise).max(0.1)).clamp(0.0, 1.0);
    let grip = rate * (1.0 - (1.0 - GRIP_AT_TOP) * over);
    let k = (grip * dt / need).min(1.0);
    (vel.0 + dx * k, vel.1 + dy * k)
}

/// The share of grip left at top speed.
pub const GRIP_AT_TOP: f32 = 0.55;

/// What his feet are doing about a change of direction or a stop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Skid {
    /// Running as told (or simply turning).
    None,
    /// Told to go back the way he came at pace: plant, pivot round, push off (a 180 cut).
    Cut,
    /// Let go of everything at a sprint: feet planted, a short slide to a stop.
    Stop,
}

/// Whether he is skidding, and how.
pub fn skidding(vel: (f32, f32), want: (f32, f32)) -> Skid {
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let w = (want.0 * want.0 + want.1 * want.1).sqrt();
    if w < 0.05 {
        return if speed > SKID_STOP_FROM { Skid::Stop } else { Skid::None };
    }
    if speed >= CUT_FROM && (vel.0 * want.0 + vel.1 * want.1) / (speed * w) < -0.35 { Skid::Cut } else { Skid::None }
}

/// Below this pace (m/s) he simply turns; above it a reversal is a cut.
pub const CUT_FROM: f32 = 2.2;
/// Stopping dead from at least this pace (m/s, a sprint) is a skid.
pub const SKID_STOP_FROM: f32 = 3.6;

/// The rifle's top swing rate while aimed (rad/s) and its angular acceleration (rad/s²): quicker than a body
/// turning to walk, still a man's arms and hips, not a turret.
pub const AIM_TOP: f32 = 9.0;
pub const AIM_ACCEL: f32 = 45.0;
/// How quickly the last few degrees close (s): the gentle settle onto the mark.
const AIM_SETTLE_S: f32 = 0.05;

fn wrap_pi(mut d: f32) -> f32 {
    while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
    while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
    d
}

/// A shooter's heading following his aim point: it matches the pace of a sweeping mouse (so the barrel sits on
/// the cursor rather than trailing it) and closes a flick with a short, eased pivot that does not overshoot.
#[derive(Copy, Clone, Debug, Default)]
pub struct AimTrack {
    pub heading: f32,
    pub spin: f32,
    last: Option<f32>,
    pace: f32,
}

impl AimTrack {
    pub fn new(heading: f32, spin: f32) -> Self {
        AimTrack { heading, spin, last: None, pace: 0.0 }
    }

    pub fn step(&mut self, want: f32, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        // How fast the mark itself is moving round him (smoothed over a few frames, so mouse jitter is not chased).
        let moving = self.last.map_or(0.0, |l| wrap_pi(want - l) / dt);
        self.last = Some(want);
        self.pace += (moving - self.pace) * (dt / 0.06).min(1.0);
        // The gap left once this step's share of the mark's own motion is covered (so he lands on it, not a frame
        // ahead of it).
        let gap = wrap_pi(want - self.heading) - self.pace * dt;
        // Close the gap: braking-distance limited far out, eased (proportional) near the mark.
        let close = gap.signum() * (2.0 * AIM_ACCEL * gap.abs()).sqrt().min(gap.abs() / AIM_SETTLE_S);
        let target = (self.pace + close).clamp(-AIM_TOP, AIM_TOP);
        self.spin += (target - self.spin).clamp(-AIM_ACCEL * dt, AIM_ACCEL * dt);
        self.heading = wrap_pi(self.heading + self.spin * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEET: Footing = Footing { push: 10.0, kick: 3.0, brake: 10.0, coast: 3.0, cruise: 3.0, top: 4.6 };

    fn run(mut v: (f32, f32), want: (f32, f32), secs: f32) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for _ in 0..(secs / 0.016) as usize {
            v = step_velocity(v, want, 0.016, FEET);
            out.push(v);
        }
        out
    }
    fn secs_until(path: &[(f32, f32)], f: impl Fn(&(f32, f32)) -> bool) -> f32 {
        path.iter().position(f).map_or(f32::INFINITY, |k| k as f32 * 0.016)
    }

    #[test]
    fn a_jog_starts_and_stops_sharply_a_sprint_builds_up() {
        let start = run((0.0, 0.0), (0.0, 3.0), 2.0);
        assert!(secs_until(&start, |v| v.1 >= 2.9) < 0.4, "up to a jog quickly");
        let kick = run((0.0, 3.0), (0.0, 4.6), 2.0);
        let t = secs_until(&kick, |v| v.1 >= 4.5);
        assert!((0.4..1.5).contains(&t), "the sprint builds up ({t:.2} s)");
        let stop = run((0.0, 3.0), (0.0, 0.0), 1.0);
        assert!(secs_until(&stop, |v| v.1 <= 0.05) < 0.4, "a jog stops short");
    }

    #[test]
    fn easing_off_a_sprint_slows_to_a_jog_gradually_without_a_skid() {
        let ease = run((0.0, 4.6), (0.0, 3.0), 2.0);
        let t = secs_until(&ease, |v| v.1 <= 3.05);
        assert!(t > 0.35, "gradual ({t:.2} s)");
        assert_eq!(skidding((0.0, 4.6), (0.0, 3.0)), Skid::None);
        assert_eq!(skidding((0.0, 4.6), (0.0, 0.0)), Skid::Stop, "letting go of everything at a sprint skids");
        assert_eq!(skidding((0.0, 3.0), (0.0, 0.0)), Skid::None, "a jog just stops");
    }

    #[test]
    fn a_reversal_at_pace_is_a_cut_back_along_the_line() {
        let path = run((-4.6, 0.0), (4.6, 0.0), 3.0);
        let t = secs_until(&path, |v| v.0 >= 0.0);
        assert!((0.2..0.8).contains(&t), "plant and pivot ({t:.2} s)");
        assert!(path.iter().all(|v| v.1.abs() < 1e-4), "straight back along the line");
        assert!(path.last().unwrap().0 > 4.0, "then off the other way");
        assert_eq!(skidding((-4.6, 0.0), (4.6, 0.0)), Skid::Cut);
        assert_eq!(skidding((-1.5, 0.0), (1.5, 0.0)), Skid::None, "walking pace just turns round");
    }

    #[test]
    fn a_sprint_turns_in_an_arc_a_walk_turns_on_the_spot() {
        let turned = |speed: f32| {
            let p = run((0.0, speed), (speed, 0.0), 0.15);
            let v = *p.last().unwrap();
            v.0.atan2(v.1).abs()
        };
        assert!(turned(4.6) < turned(1.7), "sprint {} vs walk {}", turned(4.6), turned(1.7));
        assert_eq!(skidding((0.0, 4.6), (4.6, 0.0)), Skid::None, "a right-angle turn is a curve, not a skid");
    }

    /// Track a mouse orbiting at `w` rad/s for `secs`; returns the final gap (rad) and the largest overshoot.
    fn follow(w: f32, secs: f32, step0: f32) -> (f32, f32) {
        let mut a = AimTrack::default();
        let (mut want, mut overshoot) = (step0, 0.0f32);
        for _ in 0..(secs / 0.016) as usize {
            want += w * 0.016;
            a.step(want, 0.016);
            if w == 0.0 { overshoot = overshoot.max((a.heading - want) * step0.signum()); }
        }
        (wrap_pi(want - a.heading), overshoot)
    }

    #[test]
    fn the_rifle_keeps_up_with_a_sweeping_mouse() {
        let (gap, _) = follow(3.0, 1.5, 0.0);
        assert!(gap.abs() < 2f32.to_radians(), "gap {:.1} degrees", gap.to_degrees());
    }

    #[test]
    fn a_flick_is_a_quick_gentle_pivot_without_overshoot() {
        let mut a = AimTrack::default();
        let mut t = 0.0;
        while (0.4 - a.heading).abs() > 1f32.to_radians() && t < 1.0 { a.step(0.4, 0.016); t += 0.016; }
        assert!(t < 0.25 && t > 0.08, "a 23 degree flick settles in {t:.2}s");
        let (_, over) = follow(0.0, 1.0, 0.4);
        assert!(over < 1f32.to_radians(), "overshoot {:.1} degrees", over.to_degrees());
    }

    #[test]
    fn a_big_swing_turns_at_a_human_rate() {
        let mut a = AimTrack::default();
        let mut top = 0.0f32;
        for _ in 0..60 { a.step(3.0, 0.016); top = top.max(a.spin.abs()); }
        assert!(top <= AIM_TOP + 1e-3, "{top}");
    }
}

/// Prone, a soldier turns about his hips, not his boots: his position (the feet, where the figure stands) moves
/// round so the hips stay put as his heading goes from `from` to `to` (radians, 0 facing +y, positive to the
/// left). `hips` is how far ahead of the feet they lie (m).
pub fn prone_turn(pos: (f64, f64), from: f32, to: f32, hips: f32) -> (f64, f64) {
    let ahead = |h: f32| (-(h.sin() * hips) as f64, (h.cos() * hips) as f64);
    let (a, b) = (ahead(from), ahead(to));
    (pos.0 + a.0 - b.0, pos.1 + a.1 - b.1)
}

// The commander's paces (m/s) and feet (viewer.rs `update_commander`).
pub const CMD_WALK: f32 = 1.7;
pub const CMD_AIM_WALK: f32 = 1.2;
pub const CMD_CROUCH: f32 = 1.4;
pub const CMD_CRAWL: f32 = 0.55;
pub const CMD_CRAWL_FAST: f32 = 1.05;
pub const CMD_SPRINT: f32 = 4.6;
/// His usual pace on foot (WASD): a jog. Alt held walks (CMD_WALK), Shift sprints.
pub const CMD_JOG: f32 = 3.0;
/// His feet: sharp up to a jog, a build-up into a sprint, a quick stop, a gentle ease off a sprint.
pub const CMD_FEET: Footing = Footing { push: 10.0, kick: 3.0, brake: 10.0, coast: 3.0, cruise: CMD_JOG, top: CMD_SPRINT };

/// The commander's movement and animation chain off the page (as `update_commander`, standing, rifle carried, camera
/// yaw 0), with the drawn figure evaluated on the CPU as common.wgsl `animate` does (no idle life, no grip).
#[cfg(test)]
mod commander_motion_tests {
    use crate::motion::*;
    use glam::Vec3;
    use scalia_render::pose::Pose;
    use crate::anim::{Animator, Drive, Face, Rifle, Stance};
    use scalia_render::mesh::{figure, part_of};
    use scalia_render::pose::{self, PoseParams};

    pub struct Sample { pub t: f32, pub pose: Pose, pub dir: f32, pub roll: f32, pub pitch: f32, pub speed: f32, pub heading: f32, pub lean: f32, pub head: Vec3, pub hips: Vec3, pub feet: [Vec3; 2] }

    /// Runs `input(t) -> (ix, iy, sprint)` at 60 fps for `secs`.
    pub fn run(secs: f32, input: impl Fn(f32) -> (f32, f32, bool)) -> Vec<Sample> {
        let fig = figure();
        let dt = 1.0 / 60.0;
        let mut a = Animator::new(0.0);
        let (mut dir, mut speed) = ((0.0f32, 1.0f32), 0.0f32);
        let mut pos = (0.0f32, 0.0f32);
        let mut out = Vec::new();
        for k in 0..(secs / dt) as usize {
            let t = k as f32 * dt;
            let ts = 100.0 + t;
            let (ix, iy, sprint) = input(t);
            let len = (ix * ix + iy * iy).sqrt();
            let wants = len > 0.01;
            let pace = if sprint { CMD_SPRINT } else { CMD_JOG };
            let target = if wants { pace } else { 0.0 };
            let want = if wants { (ix / len * target, iy / len * target) } else { (0.0, 0.0) };
            let vel = (dir.0 * speed, dir.1 * speed);
            let skid = crate::motion::skidding(vel, want);
            let vel = crate::motion::step_velocity(vel, want, dt, CMD_FEET);
            speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
            if speed > 1e-3 { dir = (vel.0 / speed, vel.1 / speed); } else if wants { dir = (ix / len, iy / len); }
            let moving = speed > 0.05;
            if moving { pos.0 += dir.0 * speed * dt; pos.1 += dir.1 * speed * dt; }
            let travel = (-dir.0).atan2(dir.1);
            let mut d = Drive::new(dt, ts);
            d.procedural = true;
            d.vel = (dir.0 * speed, dir.1 * speed);
            d.want = Some(want);
            d.face = if skid == crate::motion::Skid::Cut && len > 0.01 { Face::Turn { heading: (-ix).atan2(iy), top: 8.0 } } else if moving { Face::Turn { heading: travel, top: 6.0 - 2.0 * (speed / CMD_SPRINT).min(1.0) } } else { Face::Keep };
            d.stance = Stance::Stand;
            d.rifle = Rifle::Ready;
            d.sprint = true;
            let f = a.step(&d);
            let rec = a.record(&f, (pos.0, pos.1, 0.0), 9.0, 0.0, 0.0, 0.0);
            let pp = mirror(&rec, ts);
            let (roll, pitch) = pose::unpack_tilt(rec[3]);
            let at = Vec3::new(pos.0, pos.1, 0.0);
            let (mut head, mut hn, mut hips, mut hpn) = (Vec3::ZERO, 0.0, Vec3::ZERO, 0.0);
            let mut feet = [Vec3::ZERO; 2];
            let mut fz = [f32::MAX; 2];
            for v in &fig.vertices {
                let part = part_of(v.color);
                let p = pose::figure_to_world(pose::pose_vertex(Vec3::from_array(v.pos), part, &pp), at, rec[4], roll, pitch);
                let o = Vec3::from_array(v.pos);
                if o.z > 1.55 { head += p; hn += 1.0; }
                if (0.85..1.05).contains(&o.z) { hips += p; hpn += 1.0; }
                if o.z < 0.12 { let s = (o.x > 0.0) as usize; if p.z < fz[s] { fz[s] = p.z; feet[s] = p; } }
            }
            out.push(Sample { t, pose: f.pose, dir: f.dir, roll, pitch, speed, heading: f.heading, lean: f.lean, head: head / hn, hips: hips / hpn, feet });
        }
        out
    }

    /// The shader's pose parameters for a record at `time` (common.wgsl `animate`, moving soldiers).
    fn mirror(r: &[f32; 12], time: f32) -> PoseParams {
        let (b, d) = ([r[4], r[5], r[6], r[7]], [r[8], r[9], r[10], r[11]]);
        let phase = if b[2] > 0.0 { (time * b[2] + b[1]).fract() } else if b[2] < 0.0 { b[1].fract() } else { 0.0 };
        let reload = (-b[2] - 1.0).clamp(0.0, 1.0);
        let w = ((time - d[1]) / 0.35).clamp(0.0, 1.0);
        let layer = d[2].floor();
        let turn = d[2].fract() * std::f32::consts::TAU;
        let dirn = if turn > std::f32::consts::PI { turn - std::f32::consts::TAU } else { turn };
        let id = b[3].floor() as u32;
        let ph = |p: u32| if p == 18 { reload } else { phase };
        let mut pp = pose::params_dir(Pose::from_id(id), ph(id), dirn);
        if w < 1.0 { let prev = d[0].floor() as u32; pp = pose::blend(&pose::params_dir(Pose::from_id(prev), ph(prev), dirn), &pp, w); }
        let base = pp;
        if layer >= 1.0 { let u = (layer - 1.0) as u32; pp = pose::layer(&base, &pose::params_dir(Pose::from_id(u), ph(u), 0.0)); }
        let (was, lw) = pose::unpack_layer(d[3]);
        let _ = (was, lw);
        if b[2] == 0.0 { pp = pose::idle(&pp, Pose::from_id(id), time + b[1] * 97.0, b[1]); }
        pp
    }

    /// The distinct poses he shows from `from` seconds on, in order.
    fn poses_after(s: &[Sample], from: f32) -> Vec<Pose> {
        let mut v: Vec<Pose> = Vec::new();
        for x in s.iter().filter(|x| x.t >= from) {
            if v.last() != Some(&x.pose) { v.push(x.pose); }
        }
        v
    }

    #[test]
    fn stopping_from_a_sprint_brakes_then_settles_without_stepping_down_through_the_gaits() {
        let s = run(4.0, |t| (0.0, if t < 2.5 { 1.0 } else { 0.0 }, true));
        assert_eq!(poses_after(&s, 2.4), vec![Pose::Sprint, Pose::Brake, Pose::Ready]);
    }

    #[test]
    fn stopping_from_a_run_settles_straight_to_ready() {
        let s = run(4.0, |t| (0.0, if t < 2.5 { 1.0 } else { 0.0 }, false));
        assert_eq!(poses_after(&s, 2.4), vec![Pose::Run, Pose::Ready]);
    }

    #[test]
    fn tapping_left_and_right_keeps_him_on_his_feet_in_one_gait() {
        let s = run(3.0, |t| (if (t / 0.18) as i32 % 2 == 0 { -1.0 } else { 1.0 }, 0.0, false));
        let p = poses_after(&s, 0.3);
        assert!(!p.contains(&Pose::Ready) && !p.contains(&Pose::Stand), "{p:?}");
        assert!(p.len() <= 2, "{p:?}");
    }

    #[test]
    fn quick_taps_do_not_rock_him_but_setting_off_still_leans_him_in() {
        let taps = run(3.0, |t| (if (t / 0.18) as i32 % 2 == 0 { -1.0 } else { 1.0 }, 0.0, false));
        let rock = taps.iter().filter(|x| x.t > 0.6).map(|x| x.pitch.abs()).fold(0.0, f32::max);
        assert!(rock <= 0.1, "tapping pitches him {rock:.3} rad");
        let start = run(1.0, |_| (0.0, 1.0, true));
        let lean = start.iter().map(|x| x.pitch).fold(0.0, f32::max);
        assert!(lean >= 0.12, "setting off leans him only {lean:.3} rad");
    }

    #[test]
    fn reversing_swings_his_legs_round_rather_than_swapping_them() {
        let s = run(3.0, |t| (if (t / 0.18) as i32 % 2 == 0 { -1.0 } else { 1.0 }, 0.0, false));
        let jump = s.windows(2).map(|w| {
            let d = (w[1].dir - w[0].dir + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            d.abs()
        }).fold(0.0, f32::max);
        assert!(jump <= 0.35, "travel direction jumps {jump:.2} rad in a frame");
    }

    #[test]
    fn a_steady_sprint_at_an_angle_does_not_buzz_from_side_to_side() {
        for s in [run(4.0, |_| (0.37, 1.0, true)), run(4.0, |t| ((0.05 * t).sin(), (0.05 * t).cos(), true))] {
            let w: Vec<f32> = s.iter().filter(|x| x.t > 2.0).map(|x| x.roll).collect();
            let flips = w.windows(3).filter(|v| (v[1] - v[0]) * (v[2] - v[1]) < 0.0).count();
            let step = w.windows(2).map(|v| (v[1] - v[0]).abs()).fold(0.0, f32::max);
            assert!(flips <= 4, "roll changes direction {flips} times in 2 s");
            assert!(step <= 0.01, "roll jumps {step:.4} rad in a frame");
        }
    }

    #[test]
    fn a_turn_at_a_sprint_still_banks_him_into_it() {
        let s = run(3.5, |t| if t < 2.0 { (0.0, 1.0, true) } else { (1.0, 0.0, true) });
        let most = s.iter().filter(|x| x.t > 2.0).map(|x| x.roll.abs()).fold(0.0, f32::max);
        assert!(most >= 0.08, "banks only {most:.3} rad");
        let step = s.windows(2).map(|v| (v[1].roll - v[0].roll).abs()).fold(0.0, f32::max);
        assert!(step <= 0.02, "bank jumps {step:.4} rad in a frame");
    }

    #[test]
    fn prone_he_turns_about_his_hips() {
        let hips_at = |p: (f64, f64), h: f32| (p.0 - (h.sin() * 0.95) as f64, p.1 + (h.cos() * 0.95) as f64);
        let (p0, h0, h1) = ((3.0, 4.0), 0.3f32, 1.9f32);
        let p1 = prone_turn(p0, h0, h1, 0.95);
        let (a, b) = (hips_at(p0, h0), hips_at(p1, h1));
        assert!((a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4, "hips moved from {a:?} to {b:?}");
    }

    #[test]
    #[ignore]
    fn dump() {
        let which = std::env::var("CASE").unwrap_or_default();
        let s = match which.as_str() {
            "ad" => run(3.0, |t| (if (t / 0.18) as i32 % 2 == 0 { -1.0 } else { 1.0 }, 0.0, false)),
            "sprint" => run(4.0, |_| (0.0, 1.0, true)),
            "diag" => run(4.0, |_| (0.37, 1.0, true)),
            "drift" => run(4.0, |t| ((0.05 * t).sin(), (0.05 * t).cos(), true)),
            "stop" => run(4.0, |t| (0.0, if t < 2.5 { 1.0 } else { 0.0 }, true)),
            _ => run(4.0, |t| (0.0, if t < 2.5 { 1.0 } else { 0.0 }, false)),
        };
        let mut o = String::from("t,roll,pitch,pose,speed,heading,lean,hx,hy,hz,px,py,pz,lx,ly,lz,rx,ry,rz\n");
        for x in &s {
            o += &format!("{:.4},{:.4},{:.4},{:?},{:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\n", x.t, x.roll, x.pitch, x.pose, x.speed, x.heading, x.lean, x.head.x, x.head.y, x.head.z, x.hips.x, x.hips.y, x.hips.z, x.feet[0].x, x.feet[0].y, x.feet[0].z, x.feet[1].x, x.feet[1].y, x.feet[1].z);
        }
        std::fs::write(format!("/tmp/cmd_{which}.csv"), o).unwrap();
    }
}
