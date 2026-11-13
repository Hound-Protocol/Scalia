//! The close battle, soldier by soldier: who can see whom, who shoots, who is hit, who is pinned. Fixed 50 ms
//! steps in soldier order, integer maths and a hashed random stream: the same start gives the same fight on
//! every machine (co-op). The AI layers above decide where soldiers go and how they stand; this decides what
//! happens when they meet.

use crate::sight::{isqrt, Point, SightGrid};
use scalia_core::hash::mix64;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

impl Stance {
    /// Eye height (mm).
    pub fn eye(self) -> i64 {
        match self {
            Stance::Stand => 1_600,
            Stance::Crouch => 1_000,
            Stance::Prone => 350,
        }
    }
    /// Points on the body a shooter tries to see: legs, chest, head (mm above the ground).
    pub fn body(self) -> [i64; 3] {
        match self {
            Stance::Stand => [500, 1_150, 1_600],
            Stance::Crouch => [400, 750, 1_000],
            Stance::Prone => [150, 250, 350],
        }
    }
    /// How big a target he makes (per mille of a standing man).
    fn size(self) -> i64 {
        match self {
            Stance::Stand => 1_000,
            Stance::Crouch => 750,
            Stance::Prone => 450,
        }
    }
}

pub const MAG: i32 = 30;
pub const DAMAGE: i32 = 35;
const SHOT_MS: i32 = 330;
const RELOAD_MS: i32 = 2_500;
const RANGE_MM: i64 = 200_000;
/// Suppression (0..1000) a round adds passing by, and hitting; and how fast it fades per second.
const SUPPRESS_MISS: i32 = 150;
const SUPPRESS_HIT: i32 = 300;
const SUPPRESS_FADE_S: i32 = 400;
/// Perception runs this often.
const PERCEIVE_MS: i32 = 250;
/// Half the view cone, as its cosine (per mille): 120 degrees across.
const CONE_COS: i64 = 500;
/// Anyone this close is noticed whichever way he faces (heard, felt).
const CLOSE_MM: i64 = 3_000;
/// Gunfire is heard this far away.
const HEAR_MM: i64 = 60_000;
/// Contact confidence (per mille) and its fade per second once out of sight, down to a floor (the last known
/// position is never forgotten in a fight).
const CONF_SEEN: i32 = 1_000;
const CONF_SHOT_AT: i32 = 700;
const CONF_HEARD: i32 = 300;
const CONF_FADE_S: i32 = 100;
const CONF_FLOOR: i32 = 100;
/// Turning speed: 4 rad/s, as the cosine and sine (per mille) of the turn in one 50 ms step.
const TURN_COS: i64 = 980;
const TURN_SIN: i64 = 199;

/// What a squad knows of one enemy.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub enemy: usize,
    /// Last known position (where he was when last seen, shot from or heard).
    pub pos: (i64, i64),
    pub seen_ms: i64,
    /// 1000 seen right now .. CONF_FLOOR long lost.
    pub confidence: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Soldier {
    pub team: u8,
    pub squad: u16,
    pub pos: (i64, i64),
    pub stance: Stance,
    pub moving: bool,
    pub health: i32,
    pub ammo: i32,
    pub mags: i32,
    /// 0 calm .. 1000 pinned.
    pub suppression: i32,
    pub cooldown_ms: i32,
    pub reloading_ms: i32,
    /// The enemy he is engaging.
    pub target: Option<usize>,
    /// Never shoots (tests, and soldiers told to hold fire).
    pub passive: bool,
    /// Which way he faces: a unit vector scaled to 1000 (north is (0, 1000)).
    pub facing: (i64, i64),
}

impl Soldier {
    pub fn new(team: u8, squad: u16, pos: (i64, i64), stance: Stance) -> Soldier {
        Soldier { team, squad, pos, stance, moving: false, health: 100, ammo: MAG, mags: 6, suppression: 0, cooldown_ms: 0, reloading_ms: 0, target: None, passive: false, facing: (0, 1_000) }
    }
    pub fn alive(&self) -> bool {
        self.health > 0
    }
    pub fn eye(&self) -> Point {
        Point { x: self.pos.0, y: self.pos.1, z: self.stance.eye() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Shot { shooter: usize, target: usize, from: Point, to: Point, hit: bool },
    Killed { soldier: usize, by: usize },
    Reload { soldier: usize },
}

pub struct Battle {
    pub soldiers: Vec<Soldier>,
    pub sight: SightGrid,
    pub seed: u64,
    pub time_ms: i64,
    /// Everything that happened, in order (the renderer and sound read new entries each frame).
    pub log: Vec<Event>,
    /// Each squad's contacts (indexed by squad id).
    pub contacts: Vec<Vec<Contact>>,
    /// Where each squad last heard friendly gunfire (a fight going on), and when.
    pub heard: Vec<Option<((i64, i64), i64)>>,
    /// Who sees whom right now (row: watcher), refreshed by perception.
    visible: Vec<Vec<bool>>,
    perceive_ms: i32,
}

impl Battle {
    pub fn new(sight: SightGrid, soldiers: Vec<Soldier>, seed: u64) -> Battle {
        let squads = soldiers.iter().map(|s| s.squad as usize + 1).max().unwrap_or(0);
        let n = soldiers.len();
        Battle { soldiers, sight, seed, time_ms: 0, log: Vec::new(), contacts: vec![Vec::new(); squads], heard: vec![None; squads], visible: vec![vec![false; n]; n], perceive_ms: 0 }
    }

    /// Whether soldier `i` sees enemy `j` right now (as of the last perception pass).
    pub fn sees(&self, i: usize, j: usize) -> bool {
        self.visible[i][j]
    }

    /// What soldier `i`'s squad knows of enemy `j`.
    pub fn contact(&self, i: usize, j: usize) -> Option<&Contact> {
        self.contacts[self.soldiers[i].squad as usize].iter().find(|c| c.enemy == j)
    }

    /// Where soldier `i`'s squad last heard friendly gunfire.
    pub fn heard(&self, i: usize) -> Option<(i64, i64)> {
        self.heard[self.soldiers[i].squad as usize].map(|h| h.0)
    }

    /// Records (or refreshes) a squad's contact; a weaker report never overwrites a stronger, fresher one.
    fn report(&mut self, squad: usize, enemy: usize, pos: (i64, i64), confidence: i32) {
        let now = self.time_ms;
        let list = &mut self.contacts[squad];
        match list.iter_mut().find(|c| c.enemy == enemy) {
            Some(c) => {
                if confidence >= c.confidence {
                    *c = Contact { enemy, pos, seen_ms: now, confidence };
                }
            }
            None => list.push(Contact { enemy, pos, seen_ms: now, confidence }),
        }
    }

    /// Who notices whom: inside the view cone (or very close) and in sight. Sightings go to the squad.
    fn perceive(&mut self) {
        let n = self.soldiers.len();
        for i in 0..n {
            for j in 0..n {
                let (me, t) = (&self.soldiers[i], &self.soldiers[j]);
                let mut v = false;
                if me.alive() && t.alive() && me.team != t.team {
                    let (dx, dy) = (t.pos.0 - me.pos.0, t.pos.1 - me.pos.1);
                    let d2 = dx * dx + dy * dy;
                    if d2 <= RANGE_MM * RANGE_MM {
                        let d = isqrt(d2).max(1);
                        let ahead = me.facing.0 * dx + me.facing.1 * dy >= d * CONE_COS;
                        v = (ahead || d2 <= CLOSE_MM * CLOSE_MM) && self.exposure(i, j) > 0;
                    }
                }
                self.visible[i][j] = v;
            }
        }
        for i in 0..n {
            for j in 0..n {
                if self.visible[i][j] {
                    let (sq, pos) = (self.soldiers[i].squad as usize, self.soldiers[j].pos);
                    self.report(sq, j, pos, CONF_SEEN);
                }
            }
        }
    }

    /// How many of the target's three body points the shooter can see (0..3).
    pub fn exposure(&self, shooter: usize, target: usize) -> i64 {
        let (s, t) = (&self.soldiers[shooter], &self.soldiers[target]);
        t.stance.body().iter().filter(|&&z| self.sight.clear(s.eye(), Point { x: t.pos.0, y: t.pos.1, z })).count() as i64
    }

    /// Chance (per mille) that a shot from `shooter` hits `target` now.
    pub fn hit_permille(&self, shooter: usize, target: usize) -> i64 {
        let (s, t) = (&self.soldiers[shooter], &self.soldiers[target]);
        let (dx, dy) = (t.pos.0 - s.pos.0, t.pos.1 - s.pos.1);
        let d_m = isqrt(dx * dx + dy * dy) / 1000;
        let base = (900 - (d_m - 10).max(0) * 600 / 140).clamp(80, 900);
        let moving = if s.moving { 500 } else { 1_000 };
        let calm = 1_000 - s.suppression as i64 / 2;
        let shown = self.exposure(shooter, target) * 1_000 / 3;
        base * t.stance.size() / 1_000 * moving / 1_000 * calm / 1_000 * shown / 1_000
    }

    fn roll(&self, who: usize, n: u64) -> i64 {
        (mix64(self.seed ^ (self.time_ms as u64).wrapping_mul(0x9e37_79b9) ^ (who as u64) << 40 ^ n) % 1_000) as i64
    }

    /// One fixed step of `dt_ms`.
    pub fn step(&mut self, dt_ms: i32) {
        self.time_ms += dt_ms as i64;
        let fade = SUPPRESS_FADE_S * dt_ms / 1_000;
        for s in &mut self.soldiers {
            s.suppression = (s.suppression - fade).max(0);
            s.cooldown_ms -= dt_ms;
        }
        let cfade = CONF_FADE_S * dt_ms / 1_000;
        for list in &mut self.contacts {
            for c in list.iter_mut() {
                c.confidence = (c.confidence - cfade).max(CONF_FLOOR);
            }
        }
        self.perceive_ms -= dt_ms;
        if self.perceive_ms <= 0 {
            self.perceive_ms += PERCEIVE_MS;
            self.perceive();
        }
        for i in 0..self.soldiers.len() {
            if !self.soldiers[i].alive() || self.soldiers[i].passive {
                continue;
            }
            if self.soldiers[i].reloading_ms > 0 {
                self.soldiers[i].reloading_ms -= dt_ms;
                if self.soldiers[i].reloading_ms <= 0 {
                    self.soldiers[i].ammo = MAG;
                }
                continue;
            }
            // Engage the nearest enemy he sees.
            let me = &self.soldiers[i];
            let mut best: Option<(i64, usize)> = None;
            for (j, t) in self.soldiers.iter().enumerate() {
                if !self.visible[i][j] || !t.alive() {
                    continue;
                }
                let (dx, dy) = (t.pos.0 - me.pos.0, t.pos.1 - me.pos.1);
                let d2 = dx * dx + dy * dy;
                if best.map_or(true, |(b, _)| d2 < b) {
                    best = Some((d2, j));
                }
            }
            self.soldiers[i].target = best.map(|b| b.1);
            // Face the fight: the target, else the freshest contact the squad has (shots from behind).
            let look = match self.soldiers[i].target {
                Some(j) => Some(self.soldiers[j].pos),
                None => self.contacts[self.soldiers[i].squad as usize].iter().filter(|c| self.time_ms - c.seen_ms <= 3_000 && c.confidence >= CONF_HEARD).max_by_key(|c| (c.confidence, c.seen_ms)).map(|c| c.pos),
            };
            if let Some(p) = look {
                let me = &mut self.soldiers[i];
                me.facing = turn_toward(me.facing, (p.0 - me.pos.0, p.1 - me.pos.1));
            }
            let Some(j) = self.soldiers[i].target else { continue };
            if self.soldiers[i].cooldown_ms > 0 {
                continue;
            }
            if self.soldiers[i].ammo <= 0 {
                if self.soldiers[i].mags > 0 {
                    self.soldiers[i].mags -= 1;
                    self.soldiers[i].reloading_ms = RELOAD_MS;
                    self.log.push(Event::Reload { soldier: i });
                }
                continue;
            }
            let p = self.hit_permille(i, j);
            let hit = self.roll(i, self.log.len() as u64) < p;
            let (from, t) = (self.soldiers[i].eye(), &self.soldiers[j]);
            let to = Point { x: t.pos.0, y: t.pos.1, z: t.stance.body()[1] };
            self.soldiers[i].ammo -= 1;
            self.soldiers[i].cooldown_ms = SHOT_MS;
            self.log.push(Event::Shot { shooter: i, target: j, from, to, hit });
            // The target's squad learns where the fire came from; everyone near hears it.
            let (shooter_pos, shooter_team) = (self.soldiers[i].pos, self.soldiers[i].team);
            let tsq = self.soldiers[j].squad as usize;
            self.report(tsq, i, shooter_pos, CONF_SHOT_AT);
            for k in 0..self.soldiers.len() {
                let o = &self.soldiers[k];
                let (dx, dy) = (o.pos.0 - shooter_pos.0, o.pos.1 - shooter_pos.1);
                if !o.alive() || dx * dx + dy * dy > HEAR_MM * HEAR_MM {
                    continue;
                }
                let sq = o.squad as usize;
                if o.team == shooter_team {
                    self.heard[sq] = Some((shooter_pos, self.time_ms));
                } else {
                    self.report(sq, i, shooter_pos, CONF_HEARD);
                }
            }
            let t = &mut self.soldiers[j];
            t.suppression = (t.suppression + if hit { SUPPRESS_HIT } else { SUPPRESS_MISS }).min(1_000);
            if hit {
                t.health = t.health.saturating_sub(DAMAGE);
                if !t.alive() {
                    self.log.push(Event::Killed { soldier: j, by: i });
                }
            }
        }
    }
}

/// Turns a facing (unit vector x1000) toward a direction by at most one step's worth (4 rad/s at 50 ms).
fn turn_toward(f: (i64, i64), d: (i64, i64)) -> (i64, i64) {
    let len = isqrt(d.0 * d.0 + d.1 * d.1);
    if len == 0 {
        return f;
    }
    let u = (d.0 * 1_000 / len, d.1 * 1_000 / len);
    let dot = f.0 * u.0 + f.1 * u.1; // x 1e6
    if dot >= TURN_COS * 1_000 {
        return u;
    }
    let side = if f.0 * u.1 - f.1 * u.0 >= 0 { 1 } else { -1 };
    let r = ((f.0 * TURN_COS - side * f.1 * TURN_SIN) / 1_000, (side * f.0 * TURN_SIN + f.1 * TURN_COS) / 1_000);
    let l = isqrt(r.0 * r.0 + r.1 * r.1).max(1);
    (r.0 * 1_000 / l, r.1 * 1_000 / l)
}
