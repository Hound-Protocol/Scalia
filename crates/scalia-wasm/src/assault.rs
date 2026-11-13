//! The killhouse assault, live: a `scalia_ai` battle run at the training house, and how its soldiers are drawn
//! (pose, gait cadence, aim layer) and heard (shots, grenades, the leader's steps).
use scalia_ai::battle::{Act, Battle, Event, Gait, Soldier, Stance};
pub use scalia_ai::scenario::KILLHOUSE_ATTACKERS as ATTACKERS;
use scalia_render::pose::{stride_m_dir, Pose};

/// Battle steps are fixed, 50 ms; drawing interpolates between the last two.
pub const STEP_MS: f64 = 50.0;

/// Where the enemy stands inside the house (mm from its south-west corner), as in the static scene.

/// Something to play: a shot or a blast, where it happened (absolute metres).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Sound {
    pub x: f64,
    pub y: f64,
    pub blast: bool,
}

pub struct Assault {
    pub battle: Battle,
    prev: Vec<(i64, i64)>,
    /// Viewer time of the last battle step (ms) and the time now.
    clock: f64,
    now: f64,
    seen: usize,
    /// Per soldier: drawn heading (eased) and its pose change (pose, previous pose, when it changed in shader seconds).
    anim: Vec<(f32, Pose, Pose, f32)>,
    /// Per soldier, smoothed: speed (m/s), direction of travel relative to his facing (rad), and until when
    /// (viewer ms) he still counts as walking, so a pause between steps never drops the legs to a stand.
    motion: Vec<(f32, f32, f64)>,
    /// Per soldier: gait phase integrated from his cadence (strides), and the viewer time it was last advanced.
    gait: Vec<(f64, f64)>,
    pub sounds: Vec<Sound>,
    /// The leader's steps as they start, for the radio.
    pub steps: Vec<(usize, &'static str)>,
    pub over: bool,
}

impl Assault {
    /// The attackers form up 27 m south of the house; the defenders hold their posts. `origin` is the house's
    /// south-west corner (mm).
    pub fn new(origin: (i64, i64), seed: u64, now_ms: f64) -> Assault {
        let battle = scalia_ai::scenario::killhouse_assault(origin, seed);
        let n = battle.soldiers.len();
        let prev = battle.soldiers.iter().map(|s| s.pos).collect();
        Assault { battle, prev, clock: now_ms, now: now_ms, seen: 0, anim: vec![(0.0, Pose::Stand, Pose::Stand, -1000.0); n], motion: vec![(0.0, 0.0, -1.0); n], gait: (0..n).map(|i| ((scalia_core::hash::mix64(i as u64 ^ 0xa55a) % 1000) as f64 / 1000.0, now_ms)).collect(), sounds: Vec::new(), steps: Vec::new(), over: false }
    }

    /// Runs the battle up to `now_ms` (at most half a second of catching up per frame).
    pub fn advance(&mut self, now_ms: f64) {
        self.now = now_ms;
        if now_ms - self.clock > 500.0 {
            self.clock = now_ms - 500.0;
        }
        while self.now - self.clock >= STEP_MS && !self.over {
            self.prev = self.battle.soldiers.iter().map(|s| s.pos).collect();
            self.battle.step(STEP_MS as i32);
            self.clock += STEP_MS;
            for e in &self.battle.log[self.seen..] {
                match e {
                    Event::Shot { from, .. } => self.sounds.push(Sound { x: from.x as f64 / 1000.0, y: from.y as f64 / 1000.0, blast: false }),
                    Event::Explosion { pos } => self.sounds.push(Sound { x: pos.0 as f64 / 1000.0, y: pos.1 as f64 / 1000.0, blast: true }),
                    Event::Order { squad, step } => self.steps.push((*squad, step)),
                    _ => {}
                }
            }
            self.seen = self.battle.log.len();
            let alive = |team| self.battle.soldiers.iter().any(|s| s.team == team && s.alive());
            if !alive(0) || !alive(1) {
                self.over = true;
            }
        }
    }

    /// Soldier `i` between the last two steps (mm).
    pub fn position(&self, i: usize) -> (f64, f64) {
        let alpha = ((self.now - self.clock) / STEP_MS).clamp(0.0, 1.0);
        let (p, q) = (self.prev.get(i).copied().unwrap_or(self.battle.soldiers[i].pos), self.battle.soldiers[i].pos);
        (p.0 as f64 + (q.0 - p.0) as f64 * alpha, p.1 as f64 + (q.1 - p.1) as f64 * alpha)
    }

    /// Instance records (12 floats each) for one team. `place` turns absolute metres into the renderer's
    /// (x, y, z), or None when off screen; `ts` is the shader clock (seconds); `body` the palette colour.
    pub fn instances(&mut self, team: u8, ts: f32, body: f32, mut place: impl FnMut(f64, f64) -> Option<(f32, f32, f32)>) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..self.battle.soldiers.len() {
            if self.battle.soldiers[i].team != team {
                continue;
            }
            let (x, y) = self.position(i);
            let Some((rx, ry, z)) = place(x / 1000.0, y / 1000.0) else { continue };
            let s = &self.battle.soldiers[i];
            let m = &mut self.motion[i];
            let raw = ((s.vel.0 * s.vel.0 + s.vel.1 * s.vel.1) as f64).sqrt() as f32 / STEP_MS as f32;
            if s.moving {
                m.2 = self.now + 250.0;
                let want = heading_of(s.vel) - heading_of(s.facing);
                m.1 += wrap(want - m.1) * 0.3;
            }
            let walking = self.now < m.2;
            m.0 += ((if walking { raw } else { 0.0 }) - m.0) * 0.25;
            let (pose, steps, upper, dir) = pose_of(s, walking, m.0, m.1);
            let a = &mut self.anim[i];
            // Heading eases round to his facing (the battle turns him at 4 rad/s; this only smooths the steps).
            let want = heading_of(s.facing);
            a.0 += wrap(want - a.0) * 0.35;
            if a.1 != pose {
                *a = (a.0, pose, a.1, ts);
            }
            // The phase is integrated here (cadence changes as he winds up and brakes) and handed to the shader as is.
            let g = &mut self.gait[i];
            let dt = ((self.now - g.1) / 1000.0).clamp(0.0, 0.1);
            *g = ((g.0 + steps as f64 * dt).fract(), self.now);
            let (phase, cadence) = (g.0 as f32, if steps > 0.0 { -1.0 } else { 0.0 });
            out.extend_from_slice(&[rx, ry, z, body, a.0, phase, cadence, pose as u32 as f32, a.2 as u32 as f32, a.3, scalia_render::pose::pack_layer(upper, dir), 0.0]);
        }
        out
    }

    /// Who is still standing on each side.
    pub fn alive(&self, team: u8) -> usize {
        self.battle.soldiers.iter().filter(|s| s.team == team && s.alive()).count()
    }
}

/// An angle brought into -π..π.
fn wrap(mut d: f32) -> f32 {
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

/// The renderer's heading for a facing vector (north is (0, 1000)): the angle the figure is turned by.
pub fn heading_of(f: (i64, i64)) -> f32 {
    (-(f.0 as f32)).atan2(f.1 as f32)
}

/// What a soldier looks like doing what he does: the body pose, its cadence (strides per second), the aim layer
/// over the legs, and the direction he moves relative to where he faces (radians, for strafing). `walking`,
/// `speed` (m/s) and `dir` are the smoothed motion: the legs follow his real pace as he winds up and brakes,
/// walking into a run into a sprint.
pub fn pose_of(s: &Soldier, walking: bool, speed: f32, dir: f32) -> (Pose, f32, Option<Pose>, f32) {
    if !s.alive() {
        return (Pose::Prone, 0.0, None, 0.0);
    }
    if let Some((act, _)) = s.act {
        return (if act == Act::Throw { Pose::Throw } else { Pose::Drone }, 0.6, None, 0.0);
    }
    let moving = walking && speed > 0.15;
    let dir = if moving { dir } else { 0.0 };
    let reload = s.reloading_ms > 0;
    let upper = if reload { Some(Pose::Reload) } else if s.aim { Some(Pose::StandAim) } else { None };
    let aiming = s.aim || s.target.is_some();
    match (s.stance, moving) {
        (Stance::Prone, _) => (Pose::Prone, 0.0, None, 0.0),
        (Stance::Crouch, true) => (Pose::CrouchWalk, speed / stride_m_dir(Pose::CrouchWalk, dir), upper, dir),
        (Stance::Crouch, false) => (if reload { Pose::Reload } else if aiming { if s.cover.is_some() { Pose::CoverAim } else { Pose::CrouchAim } } else { Pose::Crouch }, if reload { 0.4 } else { 0.0 }, None, 0.0),
        (Stance::Stand, true) => {
            // The legs go by how fast he really is, not the gait he is heading for.
            let pose = if speed > 4.0 && s.gait == Gait::Sprint { Pose::Sprint } else if speed > 2.3 { Pose::Run } else { Pose::Walk };
            // Sprinting the rifle is across the chest: no aim layer.
            let upper = if pose == Pose::Sprint { None } else { upper };
            (pose, speed / stride_m_dir(pose, dir), upper, dir)
        }
        (Stance::Stand, false) => (if reload { Pose::Reload } else if aiming { Pose::StandAim } else { Pose::Stand }, if reload { 0.4 } else { 0.0 }, None, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soldiers_look_like_what_they_are_doing() {
        let mut s = Soldier::new(0, 0, (0, 0), Stance::Stand);
        assert_eq!(pose_of(&s, false, 0.0, 0.0).0, Pose::Stand);
        s.target = Some(3);
        assert_eq!(pose_of(&s, false, 0.0, 0.0).0, Pose::StandAim);
        // Walking left of his facing at 1.5 m/s, rifle up: walking legs, aiming upper body.
        s.aim = true;
        let (p, steps, upper, dir) = pose_of(&s, true, 1.5, std::f32::consts::FRAC_PI_2);
        assert_eq!((p, upper), (Pose::Walk, Some(Pose::StandAim)));
        assert!(steps > 0.5 && dir == std::f32::consts::FRAC_PI_2, "{steps} {dir}");
        // Told to run but still winding up: walking legs until he is really going, then running ones.
        s.gait = Gait::Run;
        assert_eq!(pose_of(&s, true, 1.2, 0.0).0, Pose::Walk);
        assert_eq!(pose_of(&s, true, 3.4, 0.0).0, Pose::Run);
        s.stance = Stance::Crouch;
        assert_eq!(pose_of(&s, true, 1.0, 0.0).0, Pose::CrouchWalk);
        s.health = 0;
        assert_eq!(pose_of(&s, true, 1.0, 0.0).0, Pose::Prone);
    }

    #[test]
    fn a_settled_man_keeps_still_legs() {
        // Once arrived he stops moving in the battle; his drawn legs must come to a stand and stay there.
        let mut a = Assault::new((80_000, 62_000), 21, 0.0);
        let mut t = 0.0;
        let mut poses = Vec::new();
        while t < 60_000.0 {
            t += 16.7;
            a.advance(t);
            let _ = a.instances(1, (t / 1000.0) as f32, 9.0, |x, y| Some((x as f32, y as f32, 0.0)));
            poses.push(a.anim[ATTACKERS + 7].1);
        }
        // A defender who never moves before contact: never a walking pose in the first 10 s.
        assert!(poses[..600].iter().all(|&p| !matches!(p, Pose::Walk | Pose::Run | Pose::Sprint | Pose::CrouchWalk)), "{:?}", &poses[..600].iter().filter(|p| matches!(p, Pose::Walk)).count());
    }

    #[test]
    fn the_live_assault_runs_and_makes_noise() {
        let mut a = Assault::new((80_000, 62_000), 21, 0.0);
        let mut t = 0.0;
        while !a.over && t < 200_000.0 {
            t += 16.7;
            a.advance(t);
        }
        assert!(a.over && a.alive(1) == 0, "attackers {} defenders {}", a.alive(0), a.alive(1));
        assert!(a.sounds.iter().filter(|s| !s.blast).count() > 20);
        assert!(a.steps.iter().any(|s| s.1 == "stack"), "{:?}", a.steps);
        let mut inst = a.instances(0, 1.0, 6.0, |x, y| Some((x as f32, y as f32, 0.0)));
        assert_eq!(inst.len(), ATTACKERS * 12);
        inst.truncate(12);
        assert!(inst.iter().all(|v| v.is_finite()));
    }
}
