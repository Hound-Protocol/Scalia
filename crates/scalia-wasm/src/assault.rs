//! The killhouse assault, live: a `scalia_ai` battle run at the training house, and how its soldiers are drawn
//! (pose, gait cadence, aim layer) and heard (shots, grenades, the leader's steps).
use scalia_ai::battle::{Act, Bark, Battle, Event, Gait, Soldier, Stance};
pub use scalia_ai::scenario::KILLHOUSE_ATTACKERS as ATTACKERS;
use scalia_render::pose::{stride_m_dir, Pose};

/// Leg changes by pace (m/s): running legs from RUN_FROM, sprinting legs from SPRINT_FROM (sprinters only).
const RUN_FROM: f32 = 1.9;
const SPRINT_FROM: f32 = 3.9;

/// Battle steps are fixed, 50 ms; drawing interpolates between the last two.
pub const STEP_MS: f64 = 50.0;

/// Where the enemy stands inside the house (mm from its south-west corner), as in the static scene.

/// Something to play: a shot or a blast, where it happened (absolute metres).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Sound {
    pub x: f64,
    pub y: f64,
    pub blast: bool,
    /// A footfall (quiet, short range); louder when running.
    pub step: Option<f32>,
    /// Viewer time (ms) it happens: rounds in a burst fall between steps.
    pub at: f64,
}

/// A round or a blast to draw: viewer time, muzzle (or blast centre) and impact, in absolute millimetres above
/// the ground (z is height over the terrain).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Fx {
    pub at: f64,
    pub from: (i64, i64, i64),
    pub to: (i64, i64, i64),
    pub hit: bool,
    pub blast: bool,
    pub tracer: bool,
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
    /// Per soldier: the legs he is on (walk, run, sprint), kept until his pace clearly calls for another.
    legs: Vec<Pose>,
    /// Per soldier: gait phase integrated from his cadence (strides), and the viewer time it was last advanced.
    gait: Vec<(f64, f64)>,
    pub sounds: Vec<Sound>,
    pub fx: Vec<Fx>,
    /// The leader's steps as they start, for the radio.
    pub steps: Vec<(usize, &'static str)>,
    pub over: bool,
    /// A training assault on its own (the K demo): it ends when one side is gone. The testing ground's
    /// field battle never ends.
    demo: bool,
    /// Squads on this ground (spawned), and squads whose order is done (once each, for radio reports).
    squads: Vec<u16>,
    pub done: Vec<u16>,
    reported: Vec<(u16, bool)>,
}

/// The enemy squad stationed in the training house.
pub const ENEMY_SQUAD: u16 = 200;
/// The K demo's attacking squad.
pub const DEMO_SQUAD: u16 = 199;

impl Assault {
    /// The attackers form up 27 m south of the house; the defenders hold their posts. `origin` is the house's
    /// south-west corner (mm).
    pub fn new(origin: (i64, i64), seed: u64, now_ms: f64) -> Assault {
        let mut a = Assault::from_battle(scalia_ai::scenario::killhouse_assault(origin, seed), now_ms);
        a.demo = true;
        a
    }

    /// The testing ground: its map, the enemy squad on its posts in the training house (holding), no one else
    /// until squads are spawned.
    pub fn field(map: scalia_ai::save::MapSpec, house: (i64, i64), seed: u64, now_ms: f64) -> Assault {
        let soldiers = scalia_ai::scenario::KILLHOUSE_DEFENDERS.iter().map(|&(x, y)| Soldier::new(1, ENEMY_SQUAD, (house.0 + x, house.1 + y), Stance::Stand)).collect();
        let mut b = Battle::on_map(map, soldiers, seed);
        b.submit(0, scalia_ai::save::Command::Squad { squad: ENEMY_SQUAD as usize, order: scalia_ai::leader::Order::Hold });
        let mut a = Assault::from_battle(b, now_ms);
        a.squads.push(ENEMY_SQUAD);
        a
    }

    fn from_battle(battle: Battle, now_ms: f64) -> Assault {
        let mut a = Assault { battle, prev: Vec::new(), clock: now_ms, now: now_ms, seen: 0, anim: Vec::new(), motion: Vec::new(), legs: Vec::new(), gait: Vec::new(), sounds: Vec::new(), fx: Vec::new(), steps: Vec::new(), over: false, demo: false, squads: Vec::new(), done: Vec::new(), reported: Vec::new() };
        a.sync_tables();
        a
    }

    /// Grows the per-soldier drawing tables to the battle's soldiers (new ones join between steps).
    fn sync_tables(&mut self) {
        for i in self.prev.len()..self.battle.soldiers.len() {
            self.prev.push(self.battle.soldiers[i].pos);
            self.anim.push((0.0, Pose::Stand, Pose::Stand, -1000.0));
            self.motion.push((0.0, 0.0, -1.0));
            self.legs.push(Pose::Walk);
            self.gait.push(((scalia_core::hash::mix64(i as u64 ^ 0xa55a) % 1000) as f64 / 1000.0, self.now));
        }
    }

    /// Brings a squad onto this ground: a soldier at each position (they join at the next step).
    pub fn spawn_squad(&mut self, squad: u16, team: u8, at: &[(i64, i64)]) {
        let t = self.battle.time_ms;
        for &pos in at {
            self.battle.submit(t, scalia_ai::save::Command::Spawn { team, squad, pos, stance: Stance::Stand });
        }
        self.squads.push(squad);
    }

    pub fn has_squad(&self, squad: u16) -> bool {
        self.squads.contains(&squad)
    }

    /// Gives a squad an order (at the next step).
    pub fn order(&mut self, squad: u16, order: scalia_ai::leader::Order) {
        self.battle.submit(self.battle.time_ms, scalia_ai::save::Command::Squad { squad: squad as usize, order });
        self.reported.retain(|r| r.0 != squad);
    }

    /// Where a squad's living soldiers are, on average (mm).
    pub fn centre(&self, squad: u16) -> Option<(i64, i64)> {
        let ms: Vec<(i64, i64)> = self.battle.soldiers.iter().filter(|s| s.squad == squad && s.alive()).map(|s| s.pos).collect();
        (!ms.is_empty()).then(|| (ms.iter().map(|p| p.0).sum::<i64>() / ms.len() as i64, ms.iter().map(|p| p.1).sum::<i64>() / ms.len() as i64))
    }

    /// One line per squad on this ground: id, alive, centre (m), the leader's current step, and how many men
    /// are moving (for the console: scaliaField()).
    pub fn describe(&self) -> String {
        let mut out = format!("t={} ms\n", self.battle.time_ms);
        for &sq in &self.squads {
            let ms: Vec<&Soldier> = self.battle.soldiers.iter().filter(|s| s.squad == sq).collect();
            let c = self.centre(sq).unwrap_or((0, 0));
            let moving = ms.iter().filter(|s| s.moving).count();
            out += &format!("squad {sq}: {}/{} alive at ({:.1}, {:.1}) m, step '{}', {} moving, done {}\n", self.squad_alive(sq), ms.len(), c.0 as f64 / 1000.0, c.1 as f64 / 1000.0, self.battle.squad_phase(sq as usize), moving, self.battle.squad_done(sq as usize));
        }
        out
    }

    /// Living soldiers of a squad.
    pub fn squad_alive(&self, squad: u16) -> usize {
        // Still in the fight: alive and on their feet.
        self.battle.soldiers.iter().filter(|s| s.squad == squad && s.alive() && !s.down()).count()
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
            self.sync_tables();
            for &sq in &self.squads {
                let done = self.battle.squad_done(sq as usize);
                match self.reported.iter_mut().find(|r| r.0 == sq) {
                    Some(r) => {
                        if done && !r.1 {
                            self.done.push(sq);
                        }
                        r.1 = done;
                    }
                    None => {
                        if done {
                            self.done.push(sq);
                        }
                        self.reported.push((sq, done));
                    }
                }
            }
            self.clock += STEP_MS;
            // Battle time T is drawn at viewer time T + offset (drawing runs one step behind, interpolating).
            let offset = self.clock - self.battle.time_ms as f64 + STEP_MS;
            for e in &self.battle.log[self.seen..] {
                match *e {
                    Event::Shot { from, to, hit, at_ms, shooter, .. } => {
                        let at = at_ms as f64 + offset;
                        self.sounds.push(Sound { x: from.x as f64 / 1000.0, y: from.y as f64 / 1000.0, blast: false, step: None, at });
                        // The muzzle: 0.6 m out towards the target, at shoulder height.
                        let (dx, dy) = (to.x - from.x, to.y - from.y);
                        let d = (((dx * dx + dy * dy) as f64).sqrt() as i64).max(1);
                        let muzzle = (from.x + dx * 600 / d, from.y + dy * 600 / d, from.z - 250);
                        // A miss lands short, long or wide: in the dirt a few metres around him.
                        let r = scalia_core::hash::mix64(at_ms as u64 ^ (shooter as u64) << 32);
                        let impact = if hit { (to.x, to.y, to.z) } else { (to.x + (r % 4_001) as i64 - 2_000, to.y + ((r >> 16) % 4_001) as i64 - 2_000, 0) };
                        // One round in four is a tracer.
                        let tracer = (r >> 40) % 4 == 0;
                        self.fx.push(Fx { at, from: muzzle, to: impact, hit, blast: false, tracer });
                    }
                    Event::Explosion { pos } => {
                        let at = self.battle.time_ms as f64 + offset;
                        self.sounds.push(Sound { x: pos.0 as f64 / 1000.0, y: pos.1 as f64 / 1000.0, blast: true, step: None, at });
                        self.fx.push(Fx { at, from: (pos.0, pos.1, 0), to: (pos.0, pos.1, 0), hit: false, blast: true, tracer: false });
                    }
                    Event::Order { squad, step } => self.steps.push((squad, step)),
                    // What his men shout, as steps of their squad ("bark:" marks them for the note line).
                    Event::Bark { soldier, bark } => {
                        let word = match bark {
                            Bark::TakingCover => "bark:taking cover",
                            Bark::Contact => "bark:contact",
                            Bark::Reloading => "bark:reloading",
                            Bark::ManDown => "bark:man down",
                            Bark::Frag => "bark:frag out",
                        };
                        self.steps.push((self.battle.soldiers[soldier].squad as usize, word));
                    }
                    _ => {}
                }
            }
            self.seen = self.battle.log.len();
            let alive = |team| self.battle.soldiers.iter().any(|s| s.team == team && s.alive() && !s.down());
            if self.demo && (!alive(0) || !alive(1)) {
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
    pub fn instances(&mut self, team: u8, ts: f32, body: impl Fn(u16) -> f32, mut place: impl FnMut(f64, f64) -> Option<(f32, f32, f32)>) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..self.battle.soldiers.len() {
            if self.battle.soldiers[i].team != team {
                continue;
            }
            let (x, y) = self.position(i);
            let Some((rx, ry, z)) = place(x / 1000.0, y / 1000.0) else { continue };
            let s = &self.battle.soldiers[i];
            let s_squad = s.squad;
            let m = &mut self.motion[i];
            // His own pace (the battle's wind-up and braking), not the step's distance: being nudged aside by a
            // neighbour must not speed up his legs.
            let raw = s.pace() as f32 / 1000.0;
            if s.moving {
                m.2 = self.now + 250.0;
                let want = heading_of(s.vel) - heading_of(s.facing);
                m.1 += wrap(want - m.1) * 0.3;
            }
            let walking = self.now < m.2;
            m.0 += ((if walking { raw } else { 0.0 }) - m.0) * 0.25;
            let (mut pose, mut steps, upper, dir) = pose_of(s, walking, m.0, m.1);
            // Walk, run and sprint legs change only when the pace is clearly past the line (no flicker).
            if matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint) {
                let held = self.legs[i];
                let v = m.0;
                let keep = match held {
                    Pose::Walk => v < RUN_FROM + 0.25,
                    Pose::Run => v >= RUN_FROM - 0.25 && !(pose == Pose::Sprint),
                    Pose::Sprint => v >= SPRINT_FROM - 0.3 && s.gait == Gait::Sprint,
                    _ => false,
                };
                if keep && held != pose {
                    pose = held;
                    steps = v / stride_m_dir(pose, dir);
                }
                self.legs[i] = pose;
            }
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
            let before = g.0;
            *g = ((g.0 + steps as f64 * dt).fract(), self.now);
            // A footfall at each half of the stride (feet on the ground): heard nearby.
            let after = g.0;
            let crossed = |at: f64| if after >= before { before < at && after >= at } else { before < at || after >= at };
            if steps > 0.0 && matches!(pose, Pose::Walk | Pose::Run | Pose::Sprint | Pose::CrouchWalk) && (crossed(0.0001) || crossed(0.5)) {
                let loud = match pose { Pose::Sprint => 1.0, Pose::Run => 0.8, Pose::CrouchWalk => 0.35, _ => 0.5 };
                self.sounds.push(Sound { x: x / 1000.0, y: y / 1000.0, blast: false, step: Some(loud), at: self.now });
            }
            let (phase, cadence) = (g.0 as f32, if steps > 0.0 { -1.0 } else { 0.0 });
            out.extend_from_slice(&[rx, ry, z, body(s_squad), a.0, phase, cadence, pose as u32 as f32, a.2 as u32 as f32, a.3, scalia_render::pose::pack_layer(upper, dir), 0.0]);
        }
        out
    }

    /// Who is still standing on each side.
    pub fn alive(&self, team: u8) -> usize {
        self.battle.soldiers.iter().filter(|s| s.team == team && s.alive() && !s.down()).count()
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
        return (Pose::Dead, 0.0, None, 0.0);
    }
    // Down: on his back, writhing (weakly once treated).
    if s.down() {
        return (Pose::Wounded, if s.stable() { 0.12 } else { 0.3 }, None, 0.0);
    }
    // Dragging a casualty: crouched, backwards, hands on him.
    if s.dragging().is_some() {
        let moving = walking && speed > 0.15;
        return if moving { (Pose::CrouchWalk, speed / stride_m_dir(Pose::CrouchWalk, dir), Some(Pose::Drag), dir) } else { (Pose::Drag, 0.0, None, 0.0) };
    }
    if let Some((act, _)) = s.act {
        return (
            match act {
                Act::Throw => Pose::Throw,
                Act::Drone => Pose::Drone,
                // Filling and laying sandbags: down on one knee, working.
                Act::Build => Pose::Crouch,
                Act::Aid => Pose::Aid,
            },
            if act == Act::Aid { 1.1 } else { 0.6 },
            None,
            0.0,
        );
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
            let pose = if speed >= SPRINT_FROM && s.gait == Gait::Sprint { Pose::Sprint } else if speed >= RUN_FROM { Pose::Run } else { Pose::Walk };
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
        // Sprinting legs need a sprint order and the pace for it.
        assert_eq!(pose_of(&s, true, 4.3, 0.0).0, Pose::Run, "fast, but not told to sprint");
        // Told to run but still winding up: walking legs until he is really going, then running ones.
        s.gait = Gait::Run;
        assert_eq!(pose_of(&s, true, 1.2, 0.0).0, Pose::Walk);
        assert_eq!(pose_of(&s, true, 3.4, 0.0).0, Pose::Run);
        s.stance = Stance::Crouch;
        assert_eq!(pose_of(&s, true, 1.0, 0.0).0, Pose::CrouchWalk);
        s.health = 0;
        assert_eq!(pose_of(&s, true, 1.0, 0.0).0, Pose::Dead);
    }

    #[test]
    fn a_squad_spawned_on_the_field_goes_where_it_is_told_and_reports_once() {
        use scalia_ai::leader::{Mode, Order};
        let map = scalia_ai::save::MapSpec::killhouse((80_000, 62_000));
        let mut a = Assault::field(map, (80_000, 62_000), 3, 0.0);
        let start: Vec<(i64, i64)> = (0..6).map(|k| (60_000 + k * 1_200, 30_000)).collect();
        a.spawn_squad(4, 0, &start);
        a.order(4, Order::Move { to: (60_000, 50_000), mode: Mode::Plain });
        let mut t = 0.0;
        let mut steps = 0;
        while t < 60_000.0 {
            t += 16.7;
            a.advance(t);
            // Drawing advances the stride (and its footfalls).
            let _ = a.instances(0, (t / 1000.0) as f32, |_| 6.0, |x, y| Some((x as f32, y as f32, 0.0)));
            steps += a.sounds.drain(..).filter(|s| s.step.is_some()).count();
        }
        assert!(steps > 50, "footfalls while they walk: {steps}");
        assert_eq!(a.squad_alive(4), 6);
        assert_eq!(a.done, vec![4], "one report, from the squad that arrived");
        let c = a.centre(4).unwrap();
        assert!((c.0 - 60_000).abs() < 2_500 && (c.1 - 50_000).abs() < 2_500, "{c:?}");
        let inst = a.instances(0, 1.0, |sq| sq as f32, |x, y| Some((x as f32, y as f32, 0.0)));
        assert_eq!(inst.len(), 6 * 12);
        assert_eq!(inst[3], 4.0, "colour by squad");
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
            let _ = a.instances(1, (t / 1000.0) as f32, |_| 9.0, |x, y| Some((x as f32, y as f32, 0.0)));
            // Until he is sent somewhere (to a wounded squadmate, say).
            if matches!(a.battle.soldiers[ATTACKERS + 7].task, scalia_ai::battle::Task::Move { .. }) || a.battle.soldiers[ATTACKERS + 7].cover.is_some() {
                break;
            }
            poses.push(a.anim[ATTACKERS + 7].1);
        }
        // A defender who does not move before contact: never a walking pose.
        assert!(poses.len() > 30 && poses.iter().all(|&p| !matches!(p, Pose::Walk | Pose::Run | Pose::Sprint | Pose::CrouchWalk)), "{:?}", &poses.iter().filter(|p| matches!(p, Pose::Walk)).count());
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
        // (Whether it comes to a door stack depends on how the fight from outside goes.)
        assert!(a.steps.iter().any(|s| s.1 == "approach"), "{:?}", a.steps);
        let mut inst = a.instances(0, 1.0, |_| 6.0, |x, y| Some((x as f32, y as f32, 0.0)));
        assert_eq!(inst.len(), ATTACKERS * 12);
        inst.truncate(12);
        assert!(inst.iter().all(|v| v.is_finite()));
    }
}
