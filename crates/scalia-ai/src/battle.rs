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
}

impl Soldier {
    pub fn new(team: u8, squad: u16, pos: (i64, i64), stance: Stance) -> Soldier {
        Soldier { team, squad, pos, stance, moving: false, health: 100, ammo: MAG, mags: 6, suppression: 0, cooldown_ms: 0, reloading_ms: 0, target: None, passive: false }
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
}

impl Battle {
    pub fn new(sight: SightGrid, soldiers: Vec<Soldier>, seed: u64) -> Battle {
        Battle { soldiers, sight, seed, time_ms: 0, log: Vec::new() }
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
            // Engage the nearest enemy he can see.
            let me = &self.soldiers[i];
            let mut best: Option<(i64, usize)> = None;
            for (j, t) in self.soldiers.iter().enumerate() {
                if t.team == me.team || !t.alive() {
                    continue;
                }
                let (dx, dy) = (t.pos.0 - me.pos.0, t.pos.1 - me.pos.1);
                let d2 = dx * dx + dy * dy;
                if d2 > RANGE_MM * RANGE_MM || best.is_some_and(|(b, _)| d2 >= b) {
                    continue;
                }
                if self.exposure(i, j) > 0 {
                    best = Some((d2, j));
                }
            }
            self.soldiers[i].target = best.map(|b| b.1);
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
