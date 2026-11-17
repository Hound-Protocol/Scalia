//! The killhouse assault, live: a `scalia_ai` battle run at the training house, and how its soldiers are drawn
//! (pose, gait cadence, aim layer) and heard (shots, grenades, the leader's steps).
use scalia_ai::battle::{Act, Bark, Battle, Event, Gait, Soldier, Stance};
pub use scalia_ai::scenario::KILLHOUSE_ATTACKERS as ATTACKERS;
use scalia_render::pose::Pose;

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
    /// Which weapon sound (`Cue`), when it is a gun's: shot, or a piece of its handling.
    pub cue: Cue,
}

/// A gun's sounds: each side carries its own rifle (ours an AR-15, the enemy an AK-47), so you hear who is
/// firing; the handling (magazine out, in, the bolt) and the click of an empty rifle are heard close by only.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Cue {
    /// Not a gun's (a blast or a footfall: see the other fields).
    #[default]
    None,
    Shot { enemy: bool },
    MagOut { enemy: bool },
    MagIn { enemy: bool },
    Bolt { enemy: bool },
    Dry,
}

impl Cue {
    /// The sound record's kind for `take_sounds` (and game.js `playSounds`): 0 our shot, 4 theirs, 5-7 our
    /// magazine out, in, bolt, 8-10 theirs, 11 the empty click; None for everything else.
    pub fn kind(self) -> Option<f32> {
        Some(match self {
            Cue::None => return None,
            Cue::Shot { enemy } => if enemy { 4.0 } else { 0.0 },
            Cue::MagOut { enemy } => if enemy { 8.0 } else { 5.0 },
            Cue::MagIn { enemy } => if enemy { 9.0 } else { 6.0 },
            Cue::Bolt { enemy } => if enemy { 10.0 } else { 7.0 },
            Cue::Dry => 11.0,
        })
    }
    /// Handling sounds carry only a few metres.
    pub fn quiet(self) -> bool {
        !matches!(self, Cue::None | Cue::Shot { .. })
    }
}

/// Off-screen soldiers keep their animation state warm at 5 Hz; visible soldiers update every render pass.
const HIDDEN_ANIMATION_INTERVAL_MS: f64 = 200.0;

/// Whether this soldier's animator should advance, and by how much. Visibility always wins; hidden
/// soldiers tick at a low rate so returning to the PDA view never reveals a frozen pose.
fn animation_dt(last_ms: f64, now_ms: f64, visible: bool) -> Option<f32> {
    let elapsed = (now_ms - last_ms).max(0.0);
    if visible || elapsed >= HIDDEN_ANIMATION_INTERVAL_MS {
        Some((elapsed / 1000.0).clamp(0.0, 0.2) as f32)
    } else {
        None
    }
}

/// How hard a round past him makes a man flinch (a hit is 1).
const NEAR_MISS_FLINCH: f32 = 0.55;

/// A flinch of `power` that started at `at`, at `now`: in fast (60 ms), then easing off (about 0.3 s for a near
/// miss, longer for a hit).
fn flinch_amount(at: f64, power: f32, now: f64) -> f32 {
    let age = now - at;
    if !(0.0..2_000.0).contains(&age) || power <= 0.0 {
        return 0.0;
    }
    let tau = 200.0 + 250.0 * power as f64;
    let k = if age < 60.0 { age / 60.0 } else { (-(age - 60.0) / tau).exp() };
    (power as f64 * k) as f32
}

/// How long one round's push lingers (ms, time constant).
const RECOIL_MS: f64 = 160.0;

/// Recoil pressure (0 to 1) at `now` from the rounds' worth `heat` built up as of the last round at `fired`.
fn recoil_pressure(fired: f64, heat: f32, now: f64) -> f32 {
    let age = now - fired;
    if !(0.0..1_500.0).contains(&age) {
        return 0.0;
    }
    let h = heat * (-age / RECOIL_MS).exp() as f32;
    (1.0 - (-0.6 * h).exp()).min(0.99)
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
    /// Which way the shooter's barrel pointed (unit, world), for the muzzle flash.
    pub bore: Option<[f32; 3]>,
}

pub struct Assault {
    pub battle: Battle,
    prev: Vec<(i64, i64)>,
    /// Viewer time of the last battle step (ms) and the time now.
    clock: f64,
    now: f64,
    seen: usize,
    /// Per soldier: his animator (anim.rs, the same library the commander uses), and the viewer time it was
    /// last stepped.
    figs: Vec<crate::anim::Animator>,
    /// Per soldier: last animation update (viewer ms). The field view and PDA map render separately;
    /// per-team clocks made the second pass receive dt=0, freezing soldiers visible only on the PDA.
    fig_at: Vec<f64>,
    /// Per soldier: where his muzzle was last drawn (mm, z over the ground), for flashes and tracers.
    muzzle: Vec<Option<(i64, i64, i64)>>,
    /// Per soldier: which way his barrel pointed when last drawn (unit, world; zero = unknown).
    bore: Vec<[f32; 3]>,
    /// Per soldier: when he last fired (viewer ms, as `now`), for the rifle's kick.
    fired: Vec<f64>,
    /// Per soldier: when fire last made him flinch (viewer ms) and how hard (1 hit, less for a near miss).
    shaken: Vec<(f64, f32)>,
    /// Per soldier: recoil pressure as of his last round (rounds' worth, each fading over `RECOIL_MS`).
    heat: Vec<f32>,
    pub sounds: Vec<Sound>,
    pub fx: Vec<Fx>,
    /// The leader's steps as they start, for the radio.
    pub steps: Vec<(usize, &'static str)>,
    pub over: bool,
    /// Squads selected by the player: their men are outlined.
    pub selected: Vec<u16>,
    /// The AI lab's event feed: every leader step and shout, stamped with battle time and squad.
    pub lab_log: Vec<String>,
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
        Assault::field_with(map, house, seed, now_ms, true)
    }

    /// The testing ground, with or without the enemy squad in the training house (the AI lab clears it for
    /// scenarios in the open).
    pub fn field_with(map: scalia_ai::save::MapSpec, house: (i64, i64), seed: u64, now_ms: f64, defenders: bool) -> Assault {
        let posts: &[(i64, i64)] = if defenders { &scalia_ai::scenario::KILLHOUSE_DEFENDERS } else { &[] };
        let soldiers = posts.iter().map(|&(x, y)| Soldier::new(1, ENEMY_SQUAD, (house.0 + x, house.1 + y), Stance::Stand)).collect();
        let mut b = Battle::on_map(map, soldiers, seed);
        b.submit(0, scalia_ai::save::Command::Squad { squad: ENEMY_SQUAD as usize, order: scalia_ai::leader::Order::Hold });
        let mut a = Assault::from_battle(b, now_ms);
        a.squads.push(ENEMY_SQUAD);
        a
    }

    fn from_battle(battle: Battle, now_ms: f64) -> Assault {
        let mut a = Assault { battle, prev: Vec::new(), clock: now_ms, now: now_ms, seen: 0, figs: Vec::new(), fig_at: Vec::new(), muzzle: Vec::new(), bore: Vec::new(), fired: Vec::new(), heat: Vec::new(), shaken: Vec::new(), sounds: Vec::new(), fx: Vec::new(), steps: Vec::new(), over: false, selected: Vec::new(), lab_log: Vec::new(), demo: false, squads: Vec::new(), done: Vec::new(), reported: Vec::new() };
        a.sync_tables();
        a
    }

    /// Grows the per-soldier drawing tables to the battle's soldiers (new ones join between steps).
    fn sync_tables(&mut self) {
        // Counted on a table only this grows: `prev` is refilled every step (to every soldier, including one who
        // joined between steps, like the commander), so it cannot say who is still missing.
        for i in self.figs.len()..self.battle.soldiers.len() {
            if self.prev.len() <= i {
                self.prev.push(self.battle.soldiers[i].pos);
            }
            let stride = (scalia_core::hash::mix64(i as u64 ^ 0xa55a) % 1000) as f32 / 1000.0;
            self.figs.push(crate::anim::Animator::with_phase(heading_of(self.battle.soldiers[i].facing), stride));
            self.fig_at.push(self.now);
            self.muzzle.push(None);
            self.bore.push([0.0; 3]);
            self.fired.push(f64::NEG_INFINITY);
            self.heat.push(0.0);
            self.shaken.push((f64::NEG_INFINITY, 0.0));
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
                    Event::Shot { from, to, hit, at_ms, shooter, target } => {
                        let at = at_ms as f64 + offset;
                        self.sounds.push(Sound { x: from.x as f64 / 1000.0, y: from.y as f64 / 1000.0, blast: false, step: None, at, cue: Cue::Shot { enemy: self.battle.soldiers[shooter].team != 0 } });
                        // From the end of his barrel as he was last drawn (any weapon, any pose); if he has not been
                        // drawn yet, 0.6 m out towards the target at shoulder height.
                        let (dx, dy) = (to.x - from.x, to.y - from.y);
                        let d = (((dx * dx + dy * dy) as f64).sqrt() as i64).max(1);
                        let muzzle = self.muzzle.get(shooter).copied().flatten().unwrap_or((from.x + dx * 600 / d, from.y + dy * 600 / d, from.z - 250));
                        // A miss lands short, long or wide: in the dirt a few metres around him.
                        let r = scalia_core::hash::mix64(at_ms as u64 ^ (shooter as u64) << 32);
                        // A round at nobody (the commander firing into the open) lands where it flew to.
                        let impact = if hit || target == shooter { (to.x, to.y, to.z) } else { (to.x + (r % 4_001) as i64 - 2_000, to.y + ((r >> 16) % 4_001) as i64 - 2_000, 0) };
                        // One round in four is a tracer.
                        let tracer = (r >> 40) % 4 == 0;
                        // The man shot at flinches: hard when hit, less for a round past him. Not at himself (the
                        // commander firing into the open).
                        if target != shooter {
                            let power = if hit { 1.0 } else { NEAR_MISS_FLINCH };
                            if let Some(f) = self.shaken.get_mut(target) {
                                if flinch_amount(f.0, f.1, at) < power {
                                    *f = (at, power);
                                }
                            }
                        }
                        if let (Some(f), Some(h)) = (self.fired.get_mut(shooter), self.heat.get_mut(shooter)) {
                            *h = *h * (-(at - *f).max(0.0) / RECOIL_MS).exp() as f32 + 1.0;
                            *f = at;
                        }
                        let bore = self.bore.get(shooter).copied().filter(|b| b.iter().any(|c| *c != 0.0));
                        self.fx.push(Fx { at, from: muzzle, to: impact, hit, blast: false, tracer, bore });
                    }
                    // A magazine change, heard through: the old one out, the fresh one in about halfway, the bolt
                    // sent home near the end (over however long his change takes).
                    Event::Reload { soldier } => {
                        let s = &self.battle.soldiers[soldier];
                        let (at, len, enemy) = (self.battle.time_ms as f64 + offset, s.reloading_ms.max(600) as f64, s.team != 0);
                        let (x, y) = (s.pos.0 as f64 / 1000.0, s.pos.1 as f64 / 1000.0);
                        for (frac, cue) in [(0.05, Cue::MagOut { enemy }), (0.5, Cue::MagIn { enemy }), (0.82, Cue::Bolt { enemy })] {
                            self.sounds.push(Sound { x, y, blast: false, step: None, at: at + len * frac, cue });
                        }
                    }
                    Event::Explosion { pos } => {
                        let at = self.battle.time_ms as f64 + offset;
                        self.sounds.push(Sound { x: pos.0 as f64 / 1000.0, y: pos.1 as f64 / 1000.0, blast: true, step: None, at, cue: Cue::None });
                        self.fx.push(Fx { at, from: (pos.0, pos.1, 0), to: (pos.0, pos.1, 0), hit: false, blast: true, tracer: false, bore: None });
                    }
                    Event::Briefed { leader, soldier, role } => {
                        let sq = self.battle.soldiers[leader].squad as usize;
                        self.lab_log.push(format!("{:>5.1}s  {} #{leader} to #{soldier}: \"you're {role}\"", self.battle.time_ms as f64 / 1000.0, lab_squad(sq)));
                    }
                    Event::Order { squad, step } => {
                        self.steps.push((squad, step));
                        self.lab_log.push(format!("{:>5.1}s  {}  {step}", self.battle.time_ms as f64 / 1000.0, lab_squad(squad)));
                    }
                    // What his men shout, as steps of their squad ("bark:" marks them for the note line).
                    Event::Bark { soldier, bark } => {
                        let word = match bark {
                            Bark::TakingCover => "bark:taking cover",
                            Bark::Contact => "bark:contact",
                            Bark::Reloading => "bark:reloading",
                            Bark::ManDown => "bark:man down",
                            Bark::Frag => "bark:frag out",
                            Bark::Smoke => "bark:smoke out",
                            Bark::TakingCommand => "bark:taking command",
                        };
                        let sq = self.battle.soldiers[soldier].squad as usize;
                        self.steps.push((sq, word));
                        self.lab_log.push(format!("{:>5.1}s  {} #{soldier}  \"{}\"", self.battle.time_ms as f64 / 1000.0, lab_squad(sq), &word[5..]));
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
        // Anyone who joined since the last step (the commander does, between steps) gets his tables first.
        self.sync_tables();
        let mut out = Vec::new();
        for i in 0..self.battle.soldiers.len() {
            // The commander's man is drawn as the commander.
            if self.battle.soldiers[i].team != team || self.battle.soldiers[i].player {
                continue;
            }
            let (x, y) = self.position(i);
            let placed = place(x / 1000.0, y / 1000.0);
            let s = &self.battle.soldiers[i];
            let Some(dt) = animation_dt(self.fig_at[i], self.now, placed.is_some()) else { continue };
            self.fig_at[i] = self.now;
            let d = drive_of(s, dt, ts);
            let kick = self.kick(i);
            let flinch = if s.alive() && !s.down() { self.flinch(i) } else { 0.0 };
            let fig = &mut self.figs[i];
            let f = fig.step(&d);
            if let Some(loud) = f.footfall.filter(|_| placed.is_some()) {
                self.sounds.push(Sound { x: x / 1000.0, y: y / 1000.0, blast: false, step: Some(loud), at: self.now, cue: Cue::None });
            }
            let Some((rx, ry, z)) = placed else { continue };
            // The squad's leader is flagged (+32 on the colour) so the shader gives him his red beret.
            let leader = if self.battle.leader_of(s.squad as usize) == Some(i) { 32.0 } else { 0.0 };
            // A selected squad's men carry the selection tag (+64 per level) for the outline: stable, hurt, killed.
            let tag = if self.selected.contains(&s.squad) { 64.0 * select_level(readiness(s.alive(), s.down(), s.health)) } else { 0.0 };
            out.extend_from_slice(&fig.record(&f, (rx, ry, z), body(s.squad) + leader + tag, kick, flinch, 0.0));
            // Where his muzzle is now, posed exactly as drawn: rounds fired from here.
            let on_feet = s.alive() && !s.down();
            let (tip, bore) = fig.barrel(&f, (x as f32, y as f32));
            self.muzzle[i] = tip.filter(|_| on_feet).map(|w| (w[0] as i64, w[1] as i64, w[2] as i64));
            self.bore[i] = if on_feet { bore } else { [0.0; 3] };
        }
        out
    }

    /// A gun sound not from the battle's log (the commander's empty click), at soldier `i`, now.
    pub fn cue(&mut self, i: usize, cue: Cue) {
        if let Some(s) = self.battle.soldiers.get(i) {
            self.sounds.push(Sound { x: s.pos.0 as f64 / 1000.0, y: s.pos.1 as f64 / 1000.0, blast: false, step: None, at: self.now, cue });
        }
    }

    /// How hard soldier `i` is flinching from fire now (0 to 1).
    pub fn flinch(&self, i: usize) -> f32 {
        self.shaken.get(i).map_or(0.0, |&(at, power)| flinch_amount(at, power, self.now))
    }
    /// Soldier `i`'s recoil pressure now (0 to 1): each round adds to it and it fades over `RECOIL_MS`, so a burst
    /// is one continuous push that builds, not a flip per round.
    pub fn kick(&self, i: usize) -> f32 {
        recoil_pressure(self.fired.get(i).copied().unwrap_or(f64::NEG_INFINITY), self.heat.get(i).copied().unwrap_or(0.0), self.now)
    }
    /// Where soldier `i`'s muzzle was last drawn (mm, z over the ground).
    pub fn muzzle_of(&self, i: usize) -> Option<(i64, i64, i64)> {
        self.muzzle.get(i).copied().flatten()
    }
    /// Where soldier `i`'s muzzle is (mm, z over the ground) when he is drawn elsewhere (the commander).
    pub fn set_muzzle(&mut self, i: usize, at: Option<(i64, i64, i64)>, bore: [f32; 3]) {
        self.sync_tables();
        if let Some(m) = self.muzzle.get_mut(i) {
            *m = at;
        }
        if let Some(b) = self.bore.get_mut(i) {
            *b = bore;
        }
    }

    /// Who is still standing on each side.
    pub fn alive(&self, team: u8) -> usize {
        self.battle.soldiers.iter().filter(|s| s.team == team && s.alive() && !s.down()).count()
    }
}


/// The renderer's heading for a facing vector (north is (0, 1000)): the angle the figure is turned by.
/// How the AI lab names a squad in its feed.
pub fn lab_squad(squad: usize) -> String {
    match squad as u16 {
        ENEMY_SQUAD => "RED".into(),
        DEMO_SQUAD => "BLUE".into(),
        LAB_RED => "RED-2".into(),
        s => format!("SQ{s}"),
    }
}
/// A second enemy squad the AI lab puts in the open.
pub const LAB_RED: u16 = 201;

/// What a soldier is doing, in a few words, for the AI lab's labels over heads.
pub fn lab_state(b: &Battle, i: usize) -> String {
    use scalia_ai::battle::Task;
    let s = &b.soldiers[i];
    if !s.alive() {
        return "KIA".into();
    }
    let mut w: Vec<String> = Vec::new();
    if s.down() {
        w.push(if s.stable() { "DOWN stable".into() } else { "DOWN bleeding".into() });
    } else {
        if b.leader_of(s.squad as usize) == Some(i) {
            w.push("LEADER".into());
        }
        // His mind first: what he wants and the plan for it.
        w.push(s.mind.describe());
        w.push(match s.task {
            Task::Idle => "idle".into(),
            Task::Move { gait, .. } => format!("move {}", format!("{gait:?}").to_lowercase()),
            Task::Hold { .. } => "hold".into(),
        });
        if s.cover.is_some() {
            w.push("COVER".into());
        }
        if s.target.is_some() {
            w.push("firing".into());
        }
        if let Some((act, _)) = s.act {
            w.push(format!("{act:?}").to_lowercase());
        }
        if s.dragging().is_some() {
            w.push("dragging".into());
        }
        if s.suppression > 0 {
            w.push(format!("sup {}", s.suppression / 10));
        }
    }
    if s.health < 100 {
        w.push(format!("hp {}", s.health));
    }
    w.join(" · ")
}

pub fn heading_of(f: (i64, i64)) -> f32 {
    (-(f.0 as f32)).atan2(f.1 as f32)
}

/// What a soldier is doing, for his animator (anim.rs): his real pace and where it takes him, where he faces,
/// his stance and rifle, and any action that takes him over (a throw, first aid, a step the leader has him on).
pub fn drive_of(s: &Soldier, dt: f32, ts: f32) -> crate::anim::Drive {
    use crate::anim::{Drive, Face, Rifle, Stance as S};
    let mut d = Drive::new(dt, ts);
    d.dead = !s.alive();
    d.down = s.down();
    // His own pace (winding up and braking; still there for a moment when told to stop).
    let pace = if s.alive() && !s.down() { s.momentum() as f32 / 1000.0 } else { 0.0 };
    let h = heading_of(s.vel);
    let moving = s.moving && pace > 0.05;
    d.vel = if pace > 0.05 { (-h.sin() * pace, h.cos() * pace) } else { (0.0, 0.0) };
    // Stopping hard (told to stop with pace on him) shows as a braking skid.
    d.want = Some(if s.moving { d.vel } else { (0.0, 0.0) });
    d.sprint = s.gait == Gait::Sprint;
    d.in_cover = s.cover.is_some();
    d.stance = match s.stance { Stance::Stand => S::Stand, Stance::Crouch => S::Crouch, Stance::Prone => S::Prone };
    let reload = s.reloading_ms > 0;
    let aiming = s.aim || s.target.is_some();
    d.rifle = if reload { Rifle::Reload } else if aiming && !(pace >= RUN_FROM && s.target.is_none()) { Rifle::Aim } else { Rifle::Ready };
    let facing = heading_of(s.facing);
    d.face = if aiming { Face::Aim { heading: facing } } else { Face::Turn { heading: facing, top: if moving { 6.0 - 2.0 * (pace / SPRINT_FROM).min(1.0) } else { 5.0 } } };
    // Dragging a casualty: crouched, backing off, hands on him.
    if s.dragging().is_some() {
        d.stance = S::Crouch;
        d.action = Some((Pose::Drag, 0.0, Some(Pose::Drag)));
    } else if let Some((act, _)) = s.act {
        let p = match act {
            Act::Throw => Pose::Throw,
            Act::Drone => Pose::Drone,
            // Filling and laying sandbags: down on one knee, working.
            Act::Build => Pose::Crouch,
            Act::Aid => Pose::Aid,
            // Opening a door: the free hand out to the handle.
            Act::Open => Pose::Signal,
        };
        d.action = Some((p, if act == Act::Aid { 1.1 } else { 0.6 }, None));
    } else if !moving && !reload && s.cover.is_none() {
        // Standing still at a step the leader has him on: how that step looks (the stack, a peek round an edge,
        // sweeping a room); in cover the cover poses come first.
        d.action = match s.cue {
            scalia_ai::battle::Cue::Stack => Some((Pose::Stack, 0.0, None)),
            scalia_ai::battle::Cue::Peek(right) => Some((if right { Pose::LeanRight } else { Pose::LeanLeft }, 0.0, None)),
            scalia_ai::battle::Cue::Sweep => Some((Pose::Sweep, 0.3, None)),
            // Listening to the briefing, down on a knee.
            scalia_ai::battle::Cue::Kneel => Some((Pose::Crouch, 0.0, None)),
            // The leader talking his men through it, with his hands.
            scalia_ai::battle::Cue::Brief => Some((Pose::Signal, 0.45, None)),
            scalia_ai::battle::Cue::None => None,
        };
    }
    d
}

#[cfg(test)]
mod tests {
    #[test]
    fn fire_makes_a_man_flinch_quickly_and_settle() {
        let f = |ms: f64, p: f32| super::flinch_amount(0.0, p, ms);
        assert_eq!(f(-10.0, 1.0), 0.0);
        assert!(f(60.0, 1.0) > 0.95, "in fast");
        assert!(f(60.0, super::NEAR_MISS_FLINCH) < f(60.0, 1.0), "a near miss less than a hit");
        assert!(f(500.0, 1.0) > f(500.0, super::NEAR_MISS_FLINCH), "a hit shakes him longer");
        assert!(f(1_500.0, 1.0) < 0.05, "settled after a second and a half");
    }

    #[test]
    fn visible_pda_soldiers_update_even_after_another_view_pass() {
        assert_eq!(super::animation_dt(1_000.0, 1_000.0, true), Some(0.0));
    }

    #[test]
    fn hidden_soldiers_tick_at_five_hz_and_resume_with_bounded_delta() {
        let interval = super::HIDDEN_ANIMATION_INTERVAL_MS;
        assert_eq!(super::animation_dt(1_000.0, 1_000.0 + interval - 1.0, false), None);
        assert_eq!(super::animation_dt(1_000.0, 1_000.0 + interval, false), Some(0.2));
        assert_eq!(super::animation_dt(1_000.0, 2_000.0, false), Some(0.2));
    }

    #[test]
    fn recoil_is_one_push_that_builds_through_a_burst() {
        // 10 rounds at 800 rpm (75 ms apart), sampled every 5 ms.
        let (mut fired, mut heat) = (f64::NEG_INFINITY, 0.0f32);
        let mut samples = Vec::new();
        for t in (0..1_400).step_by(5) {
            let t = t as f64;
            if t % 75.0 == 0.0 && t < 750.0 {
                heat = heat * (-(t - fired).max(0.0) / super::RECOIL_MS).exp() as f32 + 1.0;
                fired = t;
            }
            samples.push(super::recoil_pressure(fired, heat, t));
        }
        let at = |ms: usize| samples[ms / 5];
        assert!(at(0) > 0.3, "the first round pushes in at once: {}", at(0));
        assert!(at(700) > at(100), "pressure builds through the burst: {} -> {}", at(100), at(700));
        // Late in the burst the sawtooth between rounds is shallow: a continuous push, not a flip per round.
        let late = &samples[600 / 5..740 / 5];
        let (lo, hi) = late.iter().fold((1.0f32, 0.0f32), |(l, h), v| (l.min(*v), h.max(*v)));
        assert!(hi - lo < 0.2, "ripple {lo}..{hi}");
        assert!(at(1_395) < 0.05, "it eases off after: {}", at(1_395));
    }

    use super::*;

    #[test]
    fn soldiers_look_like_what_they_are_doing() {
        use crate::anim::Animator;
        // A few frames of the shared animator on his battle state, at `pace` m/s heading `vel`.
        let look = |s: &Soldier| {
            let mut a = Animator::new(heading_of(s.facing));
            let mut f = a.step(&drive_of(s, 1.0 / 60.0, 0.0));
            for k in 1..20 {
                f = a.step(&drive_of(s, 1.0 / 60.0, k as f32 / 60.0));
            }
            f
        };
        let mut s = Soldier::new(0, 0, (0, 0), Stance::Stand);
        s.facing = (0, 1_000);
        assert_eq!(look(&s).pose, Pose::Ready);
        s.target = Some(3);
        assert_eq!(look(&s).pose, Pose::StandAim);
        // Walking left of his facing, rifle up: walking legs, aiming upper body, moving sideways.
        s.aim = true;
        s.moving = true;
        s.vel = (-1_000, 0);
        s.set_momentum(1_500);
        let f = look(&s);
        assert_eq!((f.pose, f.upper), (Pose::Walk, Some(Pose::StandAim)));
        assert!((f.dir - std::f32::consts::FRAC_PI_2).abs() < 0.05, "{}", f.dir);
        // Sprinting legs need a sprint order and the pace for it.
        s.target = None;
        s.aim = false;
        s.vel = (0, 1_000);
        s.set_momentum(4_300);
        assert_eq!(look(&s).pose, Pose::Run, "fast, but not told to sprint");
        s.gait = Gait::Sprint;
        assert_eq!(look(&s).pose, Pose::Sprint);
        s.stance = Stance::Crouch;
        s.set_momentum(1_000);
        assert_eq!(look(&s).pose, Pose::CrouchWalk);
        s.health = 0;
        assert_eq!(look(&s).pose, Pose::FallForward, "killed: he goes down first");
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
        assert_eq!(inst[3] % 32.0, 4.0, "colour by squad (the leader carries +32 for his beret)");
        assert_eq!(inst[3], 36.0, "the first man is the leader");
    }

    #[test]
    fn a_settled_man_keeps_still_legs() {
        // Keep this animation check free of combat reactions: it tests a defender holding a post, not incoming fire.
        let mut a = Assault::new((80_000, 62_000), 21, 0.0);
        for s in &mut a.battle.soldiers {
            s.passive = true;
            s.health = i32::MAX;
        }
        let mut t = 0.0;
        let mut poses = Vec::new();
        let mut post = None;
        while t < 60_000.0 {
            t += 16.7;
            a.advance(t);
            let _ = a.instances(1, (t / 1000.0) as f32, |_| 9.0, |x, y| Some((x as f32, y as f32, 0.0)));
            let s = &a.battle.soldiers[ATTACKERS + 7];
            // Once he has walked to his post and stopped: from then on, until he is sent elsewhere.
            let scalia_ai::battle::Task::Hold { at, .. } = s.task else { if post.is_some() { break } else { continue } };
            if post.is_some_and(|p| p != at) || s.cover.is_some() || !s.alive() || s.down() {
                break;
            }
            if post.is_none() {
                if s.moving || (s.pos.0 - at.0).abs() + (s.pos.1 - at.1).abs() > 300 {
                    continue;
                }
                post = Some(at);
                // Give the drawn legs half a second to come to a stand.
                for _ in 0..30 {
                    t += 16.7;
                    a.advance(t);
                    let _ = a.instances(1, (t / 1000.0) as f32, |_| 9.0, |x, y| Some((x as f32, y as f32, 0.0)));
                }
                continue;
            }
            poses.push(a.figs[ATTACKERS + 7].shown());
        }
        // A defender standing at his post: never a walking pose.
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

/// A soldier's readiness for the squad tags: 0 stable, 1 wounded (hit but on his feet), 2 incapacitated (down),
/// 3 deceased.
/// The selection outline's level for a readiness: 1 green (stable), 2 yellow (wounded or incapacitated), 3 red
/// (killed).
pub fn select_level(readiness: u8) -> f32 {
    match readiness {
        0 => 1.0,
        1 | 2 => 2.0,
        _ => 3.0,
    }
}

pub fn readiness(alive: bool, down: bool, health: i32) -> u8 {
    match (alive, down) {
        (false, _) => 3,
        (true, true) => 2,
        _ if health < 100 => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::readiness;
    #[test]
    fn readiness_names_each_state() {
        assert_eq!(readiness(true, false, 100), 0);
        assert_eq!(readiness(true, false, 70), 1);
        assert_eq!(readiness(true, true, 30), 2);
        assert_eq!(readiness(false, false, 0), 3);
    }
}
