//! The squad leader: turns the commander's intent into a plan and runs it as drills.
//!
//! The plan comes from the GOAP planner over facts about the building (are we there, do we know what is inside,
//! is someone waiting in the entry room, are we stacked, inside, has it been cleared). Action costs are set from
//! the situation before every plan (a drone is worth it when blind, storming when we outnumber them, a grenade
//! when the entry room is occupied), and the leader plans again whenever a step finishes, so new intel changes
//! what happens next. Each step is a drill that hands the soldiers their tasks; the soldiers' own survival
//! (cover when pinned, fighting what they see) runs underneath it.

use crate::battle::{Act, Battle, Event, Gait, Stance, Task, BODY_MM, DIRS_24, SPACING_MM};
use crate::goap::{self, Action, Goal, Limits, State};
use crate::sight::isqrt;
use crate::tactics::{Building, Height, STACK_GAP_MM};
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{Prop, PropKind};

/// The commander's intent for a squad.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Order {
    None,
    /// Take the building and hold it.
    Capture,
    /// Hold the building (or wherever they stand): defend.
    Hold,
    /// Go to a point and make it safe there (cover within 10 m, else a perimeter), in the given manner.
    Move { to: (i64, i64), mode: Mode },
}

/// How a squad moves (the commander's choice on the radio).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    /// Just go there.
    Plain,
    /// Possible hostile presence: slow, weapons up, scanning sectors and corners.
    Caution,
    /// Hostile presence confirmed: expect contact, bound and be ready to assault.
    Confirmed,
    /// Hold the area: fighting positions and fortification.
    Garrison,
}

/// Garrison: sandbag loads a squad carries, one 2 m wall each (0.7 m thick: low cover, 1 m high), and the time
/// to fill and lay one.
pub const SANDBAG_KIT: u8 = 4;
const SANDBAG_LEN_MM: i64 = 2_000;
const SANDBAG_THICK_MM: i64 = 700;
const BUILD_MS: i64 = 6_000;
/// Casualty care: first aid takes this long; enemies this close make the helper drag the wounded man into
/// cover first, looking for it this far around him; the dragged man ends this far behind the cover spot.
const AID_MS: i64 = 8_000;
const DRAG_THREAT_MM: i64 = 120_000;
const DRAG_MM: i64 = 15_000;
const DRAG_REACH: i64 = 700;
/// Hostiles confirmed: the length of one bound.
const BOUND_MM: i64 = 12_000;
/// With caution: corners this close ahead are watched.
const CORNER_MM: i64 = 8_000;

/// The way a man watches while moving with caution: by rank, front, right, left, rear, then the diagonals;
/// each sweeps 30 degrees either side, turning every 1.5 s. (`DIRS_24` steps are 15 degrees.)
fn sector(pos: (i64, i64), to: (i64, i64), rank: usize, time_ms: i64) -> (i64, i64) {
    const OFFSETS: [i64; 8] = [0, -6, 6, 12, -3, 3, -9, 9];
    let ahead = (to.0 - pos.0, to.1 - pos.1);
    let base = (0..24).max_by_key(|&k| DIRS_24[k].0 * ahead.0 + DIRS_24[k].1 * ahead.1).unwrap_or(0) as i64;
    let sweep = if (time_ms / 1_500 + rank as i64) % 2 == 0 { 2 } else { -2 };
    DIRS_24[(base + OFFSETS[rank % OFFSETS.len()] + sweep).rem_euclid(24) as usize]
}

/// Around the spot they were sent to: cover within this reach is taken on arrival.
pub const SECURE_MM: i64 = 10_000;
/// Close enough to the spot to count as there when he cannot get onto it (it is taken or blocked).
const ARRIVE_NEAR_MM: i64 = 4_500;

/// One step of a building assault.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Method {
    Approach,
    Drone,
    Stack,
    Grenade,
    Storm,
    SlowEntry,
    Clear,
    Consolidate,
}

impl Method {
    pub fn name(self) -> &'static str {
        match self {
            Method::Approach => "approach",
            Method::Drone => "drone",
            Method::Stack => "stack",
            Method::Grenade => "grenade",
            Method::Storm => "storm",
            Method::SlowEntry => "slow entry",
            Method::Clear => "clear",
            Method::Consolidate => "consolidate",
        }
    }
}

/// What the leader knows when he plans.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Situation {
    /// We know what is inside (a drone pass, or contacts in the building).
    pub intel: bool,
    /// Someone is known to be in the room behind the entry door.
    pub enemy_in_entry: bool,
    pub have_drone: bool,
    pub have_grenade: bool,
    /// Our men able to fight.
    pub strength: i64,
    /// Enemies known to be inside.
    pub enemies_known: i64,
}

const AT: u32 = 0;
const INTEL: u32 = 1;
const ENTRY_HOT: u32 = 2;
const STACKED: u32 = 3;
const INSIDE: u32 = 4;
const CLEARED: u32 = 5;
const HOLDING: u32 = 6;

/// The steps from `start` (facts already true) to holding the building, cheapest first.
fn plan_from(s: &Situation, start: State) -> Vec<Method> {
    let ratio = s.strength / s.enemies_known.max(1);
    let storm_cost = if ratio >= 3 { 2 } else if ratio >= 2 { 3 } else { 7 };
    let mut acts: Vec<(Action, Method)> = vec![
        (Action::new("approach", 2).sets(AT, true), Method::Approach),
        (Action::new("stack", 1).needs(AT, true).sets(STACKED, true), Method::Stack),
        (Action::new("storm", storm_cost).needs(STACKED, true).needs(INTEL, true).needs(ENTRY_HOT, false).sets(INSIDE, true), Method::Storm),
        (Action::new("slow entry", 6).needs(STACKED, true).needs(ENTRY_HOT, false).sets(INSIDE, true), Method::SlowEntry),
        // Going in slowly on top of someone waiting in the entry room is the worst way in.
        (Action::new("slow entry, hot", 10).needs(STACKED, true).needs(ENTRY_HOT, true).sets(INSIDE, true), Method::SlowEntry),
        (Action::new("clear", 3).needs(INSIDE, true).sets(CLEARED, true), Method::Clear),
        (Action::new("consolidate", 1).needs(CLEARED, true).sets(HOLDING, true), Method::Consolidate),
    ];
    if s.have_drone {
        acts.push((Action::new("drone", 2).needs(AT, true).needs(STACKED, false).needs(INTEL, false).sets(INTEL, true), Method::Drone));
    }
    if s.have_grenade {
        acts.push((Action::new("grenade", 1).needs(STACKED, true).needs(ENTRY_HOT, true).sets(ENTRY_HOT, false), Method::Grenade));
    }
    let actions: Vec<Action> = acts.iter().map(|a| a.0.clone()).collect();
    let goal = Goal::new().want(HOLDING, true);
    match goap::plan(start, goal, &actions, Limits::default()) {
        Some(p) => p.actions.iter().map(|&i| acts[i].1).collect(),
        None => vec![Method::Approach, Method::Stack, Method::SlowEntry, Method::Clear, Method::Consolidate],
    }
}

fn facts(s: &Situation) -> State {
    let mut v = 0u64;
    if s.intel {
        v |= 1 << INTEL;
    }
    if s.enemy_in_entry {
        v |= 1 << ENTRY_HOT;
    }
    State(v)
}

/// The leader's plan for taking a building in this situation, from the start.
pub fn choose(s: &Situation) -> Vec<Method> {
    plan_from(s, facts(s))
}

/// A squad leader's working state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Brain {
    pub order: Order,
    plan: Vec<Method>,
    /// Facts achieved so far (AT, STACKED, INSIDE, CLEARED, HOLDING).
    done: u64,
    phase_ms: i64,
    entry: Option<usize>,
    legs: Vec<(i64, i64)>,
    leg: usize,
    leg_ms: i64,
    rooms: Vec<usize>,
    room: usize,
    room_ms: i64,
    drone_done: bool,
    grenade_used: bool,
    blast_at: Option<(i64, (i64, i64))>,
    holding: bool,
    /// Move order: arrived and security set.
    secured: bool,
    /// Sandbag loads the squad carries (kept across orders), and bags being laid: (done at ms, x, y, turns).
    kit: u8,
    builds: Vec<(i64, i64, i64, u8, usize)>,
    /// Casualty care in progress: (wounded, helper, step 0 going / 1 dragging / 2 treating, until ms, drag to).
    aid: Vec<(usize, usize, u8, i64, (i64, i64))>,
}

impl Brain {
    pub fn new() -> Brain {
        Brain { order: Order::None, plan: Vec::new(), done: 0, phase_ms: 0, entry: None, legs: Vec::new(), leg: 0, leg_ms: 0, rooms: Vec::new(), room: 0, room_ms: 0, drone_done: false, grenade_used: false, blast_at: None, holding: false, secured: false, kit: SANDBAG_KIT, builds: Vec::new(), aid: Vec::new() }
    }
}

impl Default for Brain {
    fn default() -> Brain {
        Brain::new()
    }
}

/// How often leaders think.
pub const LEAD_MS: i64 = 250;
const LEG_MM: i64 = 12_000;
const LEG_TIMEOUT_MS: i64 = 12_000;
const STACK_TIMEOUT_MS: i64 = 20_000;
const DRONE_MS: i64 = 6_000;
const THROW_MS: i64 = 1_200;
const FUSE_MS: i64 = 1_500;
const BLAST_MM: i64 = 4_000;
const ROOM_TIMEOUT_MS: i64 = 25_000;

fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    isqrt((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2))
}

/// A room's corners, a metre in from its walls.
fn corners(k: &Killhouse, r: usize) -> [(i64, i64); 4] {
    let room = k.rooms[r];
    let (a, b) = ((room.min.0 + 1_000, room.min.1 + 1_000), (room.max.0 - 1_000, room.max.1 - 1_000));
    [a, (b.0, a.1), b, (a.0, b.1)]
}

impl Battle {
    /// Puts a building on the battlefield for the leaders to plan around.
    pub fn set_building(&mut self, k: Killhouse) {
        let plan = Building::of(&k);
        self.building = Some((k, plan));
    }

    pub fn building(&self) -> Option<&Killhouse> {
        self.building.as_ref().map(|b| &b.0)
    }

    /// Gives a squad the commander's intent.
    pub fn command(&mut self, squad: usize, order: Order) {
        while self.brains.len() <= squad {
            self.brains.push(Brain::new());
        }
        // The sandbags they carry stay with them whatever the order.
        let (kit, aid) = (self.brains[squad].kit, std::mem::take(&mut self.brains[squad].aid));
        self.brains[squad] = Brain { order, kit, aid, ..Brain::new() };
    }

    /// The squad has done what it was told (the building taken and held, or arrived and secured).
    pub fn squad_done(&self, squad: usize) -> bool {
        self.brains.get(squad).is_some_and(|b| b.holding || b.secured)
    }

    /// Hostiles confirmed: two teams leapfrog towards the spot in bounds of BOUND_MM, one moving while the other
    /// is down with weapons on the objective. Returns true once the last bound is done (then all close in).
    fn bound(&mut self, squad: usize, ms: &[usize], to: (i64, i64)) -> bool {
        let b = &mut self.brains[squad];
        if b.legs.is_empty() {
            // Bound points from where the squad is to the spot.
            let n = ms.len() as i64;
            let c = (ms.iter().map(|&i| self.soldiers[i].pos.0).sum::<i64>() / n, ms.iter().map(|&i| self.soldiers[i].pos.1).sum::<i64>() / n);
            let d = dist(c, to);
            let k = (d / BOUND_MM).max(1);
            let b = &mut self.brains[squad];
            b.legs = (1..=k).map(|j| (c.0 + (to.0 - c.0) * j / k, c.1 + (to.1 - c.1) * j / k)).collect();
            b.leg = 0;
        }
        let b = &self.brains[squad];
        // The last leg is the spot itself: once a team has reached it, the bounding is over.
        if b.leg >= b.legs.len() * 2 {
            return true;
        }
        let (leg, team) = (b.leg / 2, b.leg % 2);
        let goal = b.legs[leg.min(b.legs.len() - 1)];
        let mut moved = true;
        for (rank, &i) in ms.iter().enumerate() {
            let s = &mut self.soldiers[i];
            s.aim = true;
            s.watch = None;
            if rank % 2 == team {
                if !matches!(s.task, Task::Move { to: t, .. } if t == goal) {
                    s.task = Task::Move { to: goal, gait: Gait::Run };
                    s.posture = None;
                }
                let there = s.settled || dist(s.pos, goal) <= ARRIVE_NEAR_MM && !s.moving;
                // At the end of his bound he gets down and covers.
                if there {
                    s.posture = Some(Stance::Crouch);
                }
                moved &= there;
            } else if !matches!(s.task, Task::Hold { .. }) {
                // Down where he is, covering the bound.
                let p = s.pos;
                s.task = Task::Hold { at: p, face: to };
                s.posture = Some(Stance::Crouch);
            }
        }
        if moved {
            self.brains[squad].leg += 1;
            self.log.push(Event::Order { squad, step: "bound" });
        }
        false
    }

    /// The nearest high-cover corner within CORNER_MM ahead of each man (towards the spot) is watched by him.
    fn watch_corners(&mut self, ms: &[usize], to: (i64, i64)) {
        let Some(cover) = self.cover.as_ref() else { return };
        let mut watched: Vec<(i64, i64)> = Vec::new();
        for &i in ms {
            let s = &self.soldiers[i];
            let ahead = (to.0 - s.pos.0, to.1 - s.pos.1);
            let corner = cover
                .spots
                .iter()
                .filter_map(|sp| sp.corner)
                .filter(|c| !watched.contains(c) && dist(*c, s.pos) <= CORNER_MM && (c.0 - s.pos.0) * ahead.0 + (c.1 - s.pos.1) * ahead.1 > 0)
                .min_by_key(|c| (dist(*c, s.pos), c.0, c.1));
            if let Some(c) = corner {
                watched.push(c);
                let p = self.soldiers[i].pos;
                self.soldiers[i].watch = Some((c.0 - p.0, c.1 - p.1));
            }
        }
    }

    /// Move: everyone heads for the spot (each gets his own place round it); once all are there, security:
    /// each man takes the nearest free cover within SECURE_MM of the spot, and the rest form a ring round it,
    /// low, each facing out over his own sector.
    fn lead_move(&mut self, squad: usize, to: (i64, i64), mode: Mode) {
        let ms = self.members(squad);
        if ms.is_empty() || self.brains[squad].secured {
            return;
        }
        let far = ms.iter().map(|&i| dist(self.soldiers[i].pos, to)).max().unwrap_or(0);
        if mode == Mode::Confirmed && !self.bound(squad, &ms, to) {
            return;
        }
        let gait = match mode {
            Mode::Caution => Gait::Careful,
            _ if far > 40_000 && mode == Mode::Plain => Gait::Run,
            _ => Gait::Walk,
        };
        let mut all_there = true;
        for (rank, &i) in ms.iter().enumerate() {
            let t = self.time_ms;
            let s = &mut self.soldiers[i];
            let going = matches!(s.task, Task::Move { to: t, .. } if t == to);
            if !going {
                s.task = Task::Move { to, gait };
                s.posture = None;
                s.aim = mode != Mode::Plain;
            }
            // With caution each man watches his own sector (front, flanks, rear), sweeping it slowly.
            s.watch = if mode == Mode::Caution { Some(sector(s.pos, to, rank, t)) } else { None };
            let there = s.settled || dist(s.pos, to) <= ARRIVE_NEAR_MM && !s.moving;
            // Expecting trouble, the first men there get down while the rest come in.
            if there && mode != Mode::Plain {
                s.posture = Some(Stance::Crouch);
            }
            all_there &= there;
        }
        // Moving with caution: a corner close ahead is watched by whoever is nearest it.
        if mode == Mode::Caution {
            self.watch_corners(&ms, to);
        }
        if !all_there {
            return;
        }
        for &i in &ms {
            self.soldiers[i].watch = None;
        }
        self.brains[squad].secured = true;
        self.log.push(Event::Order { squad, step: if mode == Mode::Garrison { "garrison" } else { "secure" } });
        // Garrisoning a building: its windows and doors, facing out.
        if mode == Mode::Garrison && self.building.as_ref().is_some_and(|(k, _)| k.room_at(to.0, to.1).is_some()) {
            self.man_building(&ms, to);
            return;
        }
        // Cover first: nearest free spot to each man, within reach of the place, spaced apart.
        let spots: Vec<(usize, (i64, i64), (i64, i64), Height)> = match self.cover.as_ref() {
            Some(c) => c.spots.iter().enumerate().filter(|(_, sp)| dist(sp.pos, to) <= SECURE_MM).map(|(k, sp)| (k, sp.pos, sp.toward, sp.height)).collect(),
            None => Vec::new(),
        };
        let mut taken: Vec<(i64, i64)> = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        for &i in &ms {
            let me = self.soldiers[i].pos;
            let pick = spots
                .iter()
                .filter(|sp| taken.iter().all(|&t| dist(t, sp.1) >= SPACING_MM))
                .min_by_key(|sp| (dist(sp.1, me), sp.0));
            match pick {
                Some(&(_, pos, toward, height)) => {
                    taken.push(pos);
                    let s = &mut self.soldiers[i];
                    s.task = Task::Hold { at: pos, face: (pos.0 + toward.0 * 5, pos.1 + toward.1 * 5) };
                    s.posture = Some(if height == Height::Low { Stance::Crouch } else { Stance::Stand });
                    s.gait = Gait::Walk;
                }
                None => open.push(i),
            }
        }
        // In the open: a ring round the spot, sectors evenly spread, each man to the nearest free slot.
        let n = open.len() as i64;
        if n == 0 {
            return;
        }
        let radius = (n * 700).max(3_000);
        let mut slots: Vec<((i64, i64), (i64, i64))> = (0..n)
            .map(|k| {
                let d = DIRS_24[(k * 24 / n) as usize];
                let at = (to.0 + d.0 * radius / 1_000, to.1 + d.1 * radius / 1_000);
                (at, d)
            })
            .filter(|(at, _)| self.nav().is_none_or(|nv| nv.walkable_at(at.0, at.1, BODY_MM)))
            .collect();
        let mut ring = Vec::new();
        for i in open {
            let me = self.soldiers[i].pos;
            let Some(k) = (0..slots.len()).min_by_key(|&k| (dist(slots[k].0, me), k)) else { break };
            let (at, d) = slots.remove(k);
            let s = &mut self.soldiers[i];
            s.task = Task::Hold { at, face: (at.0 + d.0 * 10, at.1 + d.1 * 10) };
            s.posture = Some(Stance::Crouch);
            s.gait = Gait::Walk;
            ring.push((i, at, d));
        }
        if mode == Mode::Garrison {
            self.fortify(squad, &ring);
        }
    }

    /// Garrison in the open: men on the ring without cover lay a sandbag wall just outside their spot, across
    /// their sector, while the kit lasts.
    fn fortify(&mut self, squad: usize, ring: &[(usize, (i64, i64), (i64, i64))]) {
        for &(i, at, d) in ring {
            if self.brains[squad].kit == 0 {
                break;
            }
            let c = (at.0 + d.0 * 900 / 1_000, at.1 + d.1 * 900 / 1_000);
            // Across his line of fire: along x when he faces north or south.
            let turns = if d.1.abs() >= d.0.abs() { 0 } else { 1 };
            let half = if turns == 0 { (SANDBAG_LEN_MM / 2, 0) } else { (0, SANDBAG_LEN_MM / 2) };
            let clear = [c, (c.0 - half.0, c.1 - half.1), (c.0 + half.0, c.1 + half.1)].iter().all(|p| self.nav().is_none_or(|n| n.walkable_at(p.0, p.1, 0)));
            if !clear {
                continue;
            }
            let done = self.time_ms + BUILD_MS + dist(self.soldiers[i].pos, at) * 1_000 / 1_500;
            self.soldiers[i].act = Some((Act::Build, done));
            let b = &mut self.brains[squad];
            b.kit -= 1;
            b.builds.push((done, c.0, c.1, turns, i));
        }
    }

    /// Garrison a building: men to the outer windows (standing, over the sill) nearest the spot, then the outer
    /// doors (low, just inside), each facing out.
    fn man_building(&mut self, ms: &[usize], to: (i64, i64)) {
        let Some((k, _)) = self.building.clone() else { return };
        let inside = |p: (i64, i64)| k.room_at(p.0, p.1).is_some();
        let mut posts: Vec<((i64, i64), (i64, i64), Stance)> = Vec::new();
        for w in &k.windows {
            let n = if w.along_x { (0, 1_000) } else { (1_000, 0) };
            let (a, b) = ((w.x_mm + n.0 * 700 / 1_000, w.y_mm + n.1 * 700 / 1_000), (w.x_mm - n.0 * 700 / 1_000, w.y_mm - n.1 * 700 / 1_000));
            let (post, out) = match (inside(a), inside(b)) {
                (true, false) => (a, (-n.0, -n.1)),
                (false, true) => (b, n),
                _ => continue,
            };
            posts.push((post, out, Stance::Stand));
        }
        for d in &k.doors {
            let outer = matches!(d.rooms, (Some(_), None) | (None, Some(_)));
            if !outer {
                continue;
            }
            let n = if d.along_x { (0, 1_000) } else { (1_000, 0) };
            let side = if d.along_x { (1_000, 0) } else { (0, 1_000) };
            let off = d.width_mm / 2 + 500;
            for sgn in [1, -1] {
                let a = (d.x_mm + sgn * n.0 * 900 / 1_000 + side.0 * off / 1_000, d.y_mm + sgn * n.1 * 900 / 1_000 + side.1 * off / 1_000);
                if inside(a) {
                    posts.push((a, (-sgn * n.0, -sgn * n.1), Stance::Crouch));
                }
            }
        }
        posts.sort_by_key(|p| (dist(p.0, to), p.0));
        for &i in ms {
            let me = self.soldiers[i].pos;
            let s = &mut self.soldiers[i];
            if posts.is_empty() {
                s.task = Task::Hold { at: to, face: me };
                s.posture = Some(Stance::Crouch);
                continue;
            }
            // The nearest of the first few posts (closest to where he was sent).
            let pick = (0..posts.len().min(ms.len() + 2)).min_by_key(|&k| (dist(posts[k].0, me), k)).unwrap_or(0);
            let (at, out, stance) = posts.remove(pick);
            s.task = Task::Hold { at, face: (at.0 + out.0 * 10, at.1 + out.1 * 10) };
            s.posture = Some(stance);
            s.gait = Gait::Walk;
        }
    }

    /// The step the squad is on (for labels and tests).
    pub fn squad_phase(&self, squad: usize) -> &'static str {
        self.brains.get(squad).and_then(|b| b.plan.first()).map_or("", |m| m.name())
    }

    fn members(&self, squad: usize) -> Vec<usize> {
        // Those able to carry out orders: alive, not down, not busy with a casualty.
        self.soldiers.iter().enumerate().filter(|(_, s)| s.squad as usize == squad && s.alive() && !s.down() && s.aiding.is_none()).map(|(i, _)| i).collect()
    }

    fn centre(&self, ms: &[usize]) -> (i64, i64) {
        let n = ms.len().max(1) as i64;
        let (sx, sy) = ms.iter().fold((0, 0), |a, &i| (a.0 + self.soldiers[i].pos.0, a.1 + self.soldiers[i].pos.1));
        (sx / n, sy / n)
    }

    /// What the squad's leader knows when he plans.
    fn situation(&self, squad: usize) -> Situation {
        let (k, plan) = self.building.as_ref().expect("a building");
        let brain = &self.brains[squad];
        let inside: Vec<(i64, i64)> = self.contacts[squad].iter().filter(|c| c.confidence >= 300 && self.soldiers[c.enemy].alive() && !self.soldiers[c.enemy].down() && k.room_at(c.pos.0, c.pos.1).is_some()).map(|c| c.pos).collect();
        let entry_room = brain.entry.map(|e| plan.clear_order(plan.entries[e].door)[0]);
        Situation {
            intel: brain.drone_done || !inside.is_empty(),
            enemy_in_entry: entry_room.is_some_and(|r| inside.iter().any(|p| k.room_at(p.0, p.1) == Some(r))),
            have_drone: !brain.drone_done,
            have_grenade: !brain.grenade_used,
            strength: self.members(squad).len() as i64,
            enemies_known: inside.len() as i64,
        }
    }

    fn replan(&mut self, squad: usize) {
        let s = self.situation(squad);
        let start = State(facts(&s).0 | self.brains[squad].done);
        let plan = plan_from(&s, start);
        let b = &mut self.brains[squad];
        let changed = b.plan.first() != plan.first();
        b.plan = plan;
        if changed {
            b.phase_ms = 0;
            if let Some(&m) = b.plan.first() {
                self.log.push(Event::Order { squad, step: m.name() });
            }
        }
    }

    fn finish(&mut self, squad: usize, fact: Option<u32>) {
        if let Some(f) = fact {
            self.brains[squad].done |= 1 << f;
        }
        self.brains[squad].plan.clear();
        self.replan(squad);
    }

    /// Every leader thinks (called every LEAD_MS of battle time).
    /// Casualty care: every man down and untreated gets the nearest fit squadmate. He goes to him; if the enemy is
    /// near and the wounded man lies in their sight, he drags him backwards into the nearest cover from them;
    /// then he treats him (AID_MS), which stops the bleeding. A helper who is hit is replaced.
    fn care(&mut self) {
        let wounded: Vec<usize> = (0..self.soldiers.len()).filter(|&i| self.soldiers[i].down() && !self.soldiers[i].stable).collect();
        for &w in &wounded {
            let squad = self.soldiers[w].squad as usize;
            while self.brains.len() <= squad {
                self.brains.push(Brain::new());
            }
            // Drop care whose helper can no longer give it.
            let soldiers = &mut self.soldiers;
            self.brains[squad].aid.retain(|a| {
                let ok = soldiers[a.1].alive() && !soldiers[a.1].down() && soldiers[a.0].alive();
                if !ok && soldiers[a.1].aiding == Some(a.0) {
                    soldiers[a.1].aiding = None;
                    soldiers[a.1].dragging = None;
                }
                ok
            });
            if self.brains[squad].aid.iter().any(|a| a.0 == w) {
                continue;
            }
            let wp = self.soldiers[w].pos;
            let helper = self
                .soldiers
                .iter()
                .enumerate()
                .filter(|(j, o)| *j != w && o.squad as usize == squad && o.alive() && !o.down() && o.aiding.is_none() && !o.passive)
                .min_by_key(|(j, o)| (dist(o.pos, wp), *j))
                .map(|(j, _)| j);
            if let Some(h) = helper {
                self.soldiers[h].aiding = Some(w);
                self.brains[squad].aid.push((w, h, 0, 0, wp));
                self.log.push(Event::Order { squad, step: "casualty" });
            }
        }
        // Each care in progress, one step at a time.
        for squad in 0..self.brains.len() {
            let mut list = std::mem::take(&mut self.brains[squad].aid);
            list.retain_mut(|a| self.give_aid(a));
            self.brains[squad].aid.extend(list);
        }
    }

    /// One look at a casualty being cared for; false once it is done (or can no longer be done).
    fn give_aid(&mut self, a: &mut (usize, usize, u8, i64, (i64, i64))) -> bool {
        let (w, h) = (a.0, a.1);
        if !self.soldiers[w].alive() || !self.soldiers[h].alive() || self.soldiers[h].down() {
            self.soldiers[h].aiding = None;
            self.soldiers[h].dragging = None;
            return false;
        }
        let wp = self.soldiers[w].pos;
        match a.2 {
            0 => {
                if dist(self.soldiers[h].pos, wp) > 1_400 {
                    self.soldiers[h].task = Task::Move { to: wp, gait: Gait::Run };
                    return true;
                }
                // At his side: drag him out of the enemy's sight first, if they are near and can see him.
                let squad = self.soldiers[w].squad as usize;
                let threat = self.contacts[squad].iter().filter(|c| c.confidence >= crate::battle::CONF_HEARD && self.soldiers[c.enemy].alive() && !self.soldiers[c.enemy].down() && dist(c.pos, wp) <= DRAG_THREAT_MM).min_by_key(|c| (dist(c.pos, wp), c.enemy)).map(|c| c.pos);
                let exposed = threat.is_some_and(|t| self.sight.clear(crate::sight::Point { x: t.0, y: t.1, z: 1_500 }, crate::sight::Point { x: wp.0, y: wp.1, z: 300 }));
                let spot = if exposed { threat.and_then(|t| self.drag_cover(wp, t)) } else { None };
                match spot {
                    Some(to) => {
                        // He walks backwards to a point past the cover, so the man in front of him ends behind it.
                        let (dx, dy) = (to.0 - wp.0, to.1 - wp.1);
                        let d = dist(to, wp).max(1);
                        let dest = (to.0 + dx * DRAG_REACH / d, to.1 + dy * DRAG_REACH / d);
                        a.2 = 1;
                        a.4 = dest;
                        let s = &mut self.soldiers[h];
                        s.dragging = Some(w);
                        s.task = Task::Move { to: dest, gait: Gait::Crouch };
                        s.aim = false;
                        self.log.push(Event::Order { squad, step: "drag" });
                    }
                    None => self.start_aid(a),
                }
                true
            }
            1 => {
                let s = &self.soldiers[h];
                if s.settled || dist(s.pos, a.4) <= 600 && !s.moving {
                    self.soldiers[h].dragging = None;
                    self.start_aid(a);
                }
                true
            }
            _ => {
                if self.time_ms >= a.3 {
                    self.soldiers[w].stable = true;
                    let s = &mut self.soldiers[h];
                    s.aiding = None;
                    s.task = Task::Idle;
                    return false;
                }
                true
            }
        }
    }

    fn start_aid(&mut self, a: &mut (usize, usize, u8, i64, (i64, i64))) {
        a.2 = 2;
        a.3 = self.time_ms + AID_MS;
        let face = self.soldiers[a.0].pos;
        let s = &mut self.soldiers[a.1];
        let p = s.pos;
        s.task = Task::Hold { at: p, face };
        s.act = Some((Act::Aid, a.3));
        s.posture = Some(Stance::Crouch);
    }

    /// The nearest cover from `threat` within reach of a wounded man at `wp`: a spot whose prop blocks the
    /// threat's sight of a man lying there.
    fn drag_cover(&self, wp: (i64, i64), threat: (i64, i64)) -> Option<(i64, i64)> {
        let cover = self.cover.as_ref()?;
        cover
            .spots
            .iter()
            .filter(|sp| dist(sp.pos, wp) <= DRAG_MM)
            .filter(|sp| !self.sight.clear(crate::sight::Point { x: threat.0, y: threat.1, z: 1_500 }, crate::sight::Point { x: sp.pos.0, y: sp.pos.1, z: 300 }))
            .min_by_key(|sp| (dist(sp.pos, wp), sp.pos))
            .map(|sp| sp.pos)
    }

    pub(crate) fn lead(&mut self) {
        self.care();
        for squad in 0..self.brains.len() {
            match self.brains[squad].order {
                Order::None => {}
                Order::Hold => self.lead_hold(squad),
                Order::Move { to, mode } => self.lead_move(squad, to, mode),
                Order::Capture => {
                    if self.building.is_some() && !self.members(squad).is_empty() {
                        self.lead_capture(squad);
                    }
                }
            }
        }
        // Sandbags being laid: in place once the work is done.
        for squad in 0..self.brains.len() {
            let now = self.time_ms;
            let (done, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut self.brains[squad].builds).into_iter().partition(|b| b.0 <= now);
            self.brains[squad].builds = wait;
            for (_, x, y, turns, builder) in done {
                // Only if he lived to finish it (the bags of a man killed at work are lost).
                if !self.soldiers[builder].alive() {
                    continue;
                }
                self.place_prop(Prop { kind: PropKind::Wall { length_mm: SANDBAG_LEN_MM, thickness_mm: SANDBAG_THICK_MM }, x_mm: x, y_mm: y, quarter_turns: turns });
            }
        }
        // Grenades in the air.
        for squad in 0..self.brains.len() {
            if let Some((at, p)) = self.brains[squad].blast_at {
                if self.time_ms >= at {
                    self.brains[squad].blast_at = None;
                    self.explode(p);
                }
            }
        }
    }

    /// Defenders: each man holds where he stands, watching the way he faced when told.
    fn lead_hold(&mut self, squad: usize) {
        let ms = self.members(squad);
        for &i in &ms {
            if self.soldiers[i].task == Task::Idle {
                let (p, f) = (self.soldiers[i].pos, self.soldiers[i].facing);
                self.soldiers[i].task = Task::Hold { at: p, face: (p.0 + f.0 * 5, p.1 + f.1 * 5) };
            }
        }
        self.fall_back(squad, &ms);
    }

    /// A defender whose room holds more known enemies than friends gives it up: he runs through an
    /// inner door into a neighbouring room with no known enemy (the one with most friends) and
    /// holds just inside it, rifle on the door he came through.
    fn fall_back(&mut self, squad: usize, ms: &[usize]) {
        let Some((k, _)) = self.building.as_ref() else { return };
        let foes_in = |b: &Battle, r: usize| b.contacts[squad].iter().filter(|c| b.soldiers[c.enemy].alive() && !b.soldiers[c.enemy].down() && k.room_at(c.pos.0, c.pos.1) == Some(r)).count();
        let friends_in = |b: &Battle, r: usize| ms.iter().filter(|&&j| k.room_at(b.soldiers[j].pos.0, b.soldiers[j].pos.1) == Some(r)).count();
        let mut moves = Vec::new();
        for &i in ms {
            let s = &self.soldiers[i];
            let Some(r) = k.room_at(s.pos.0, s.pos.1) else { continue };
            // Already on his way out of this room.
            if let Task::Hold { at, .. } = s.task {
                if k.room_at(at.0, at.1) != Some(r) {
                    continue;
                }
            }
            let foes = foes_in(self, r);
            if foes == 0 || friends_in(self, r) > foes {
                continue;
            }
            let mut best: Option<(usize, (i64, i64), usize)> = None;
            for d in &k.doors {
                let (Some(a), Some(b2)) = d.rooms else { continue };
                let next = if a == r { b2 } else if b2 == r { a } else { continue };
                if foes_in(self, next) > 0 {
                    continue;
                }
                let f = friends_in(self, next);
                if best.is_none_or(|b| f > b.2) {
                    best = Some((next, (d.x_mm, d.y_mm), f));
                }
            }
            let Some((next, door, _)) = best else { continue };
            let c = k.rooms[next].centre();
            let (dx, dy) = (c.0 - door.0, c.1 - door.1);
            let len = dist(c, door).max(1);
            let at = (door.0 + dx * 1_800.min(len) / len, door.1 + dy * 1_800.min(len) / len);
            moves.push((i, at, door));
        }
        if !moves.is_empty() {
            self.log.push(Event::Order { squad, step: "fall back" });
        }
        for (i, at, door) in moves {
            let s = &mut self.soldiers[i];
            s.task = Task::Hold { at, face: door };
            s.gait = Gait::Run;
            s.cover = None;
        }
    }

    fn explode(&mut self, p: (i64, i64)) {
        self.log.push(Event::Explosion { pos: p });
        for s in &mut self.soldiers {
            let d = dist(s.pos, p);
            if s.alive() && d < BLAST_MM * 2 {
                s.suppression = 1_000;
                if d < BLAST_MM {
                    s.health = s.health.saturating_sub((120 * (BLAST_MM - d) / BLAST_MM) as i32 + 20);
                }
            }
        }
        for i in 0..self.soldiers.len() {
            if !self.soldiers[i].alive() && !self.dead_logged[i] {
                self.dead_logged[i] = true;
                self.log.push(Event::Killed { soldier: i, by: usize::MAX });
            }
        }
    }

    fn lead_capture(&mut self, squad: usize) {
        if self.brains[squad].holding {
            return;
        }
        if self.brains[squad].plan.is_empty() {
            self.replan(squad);
        }
        self.brains[squad].phase_ms += LEAD_MS;
        let ms = self.members(squad);
        let (k, plan) = self.building.clone().expect("a building");
        let (k, plan) = (&k, &plan);
        let kc = (k.origin.0 + 10_000, k.origin.1 + 7_000);
        // Entry: the outside door nearest the squad.
        if self.brains[squad].entry.is_none() {
            let c = self.centre(&ms);
            let e = (0..plan.entries.len()).min_by_key(|&e| dist(plan.entries[e].stack[0], c)).expect("an entry");
            self.brains[squad].entry = Some(e);
        }
        let e = self.brains[squad].entry.expect("chosen");
        let door = plan.entries[e].door;
        let d = k.doors[door];
        // The stack goes on whichever side of the door has wall to hug: mirrored if the planned side runs
        // past the corner (unwalkable, or out round the end of the wall).
        let mut stack = plan.entries[e].stack.clone();
        let walk = |p: &(i64, i64)| self.nav().is_none_or(|n| n.walkable_at(p.0, p.1, 0));
        if !stack.iter().all(walk) {
            let flip: Vec<(i64, i64)> = stack.iter().map(|&(x, y)| if d.along_x { (2 * d.x_mm - x, y) } else { (x, 2 * d.y_mm - y) }).collect();
            if flip.iter().all(walk) {
                stack = flip;
            }
        }
        let entry_room = plan.clear_order(door)[0];
        let step = self.brains[squad].plan.first().copied();
        let phase_ms = self.brains[squad].phase_ms;
        match step {
            None => {}
            Some(Method::Approach) => {
                if self.brains[squad].legs.is_empty() {
                    let from = self.centre(&ms);
                    let total = dist(from, stack[0]);
                    let n = (total / LEG_MM).max(1);
                    self.brains[squad].legs = (1..=n).map(|k| (from.0 + (stack[0].0 - from.0) * k / n, from.1 + (stack[0].1 - from.1) * k / n)).collect();
                    self.brains[squad].leg = 0;
                    self.brains[squad].leg_ms = 0;
                }
                self.brains[squad].leg_ms += LEAD_MS;
                let (leg, legs) = (self.brains[squad].leg, self.brains[squad].legs.clone());
                if leg >= legs.len() {
                    self.brains[squad].legs.clear();
                    self.finish(squad, Some(AT));
                    return;
                }
                // Fire teams by rank: team (leg % 2) bounds to the next point in a wedge pointing along the
                // way in, the other watches the building from where it is.
                let goal = legs[leg];
                let from = if leg == 0 { self.centre(&ms) } else { legs[leg - 1] };
                let (fx, fy) = (goal.0 - from.0, goal.1 - from.1);
                let len = isqrt(fx * fx + fy * fy).max(1);
                let (ux, uy) = (fx * 1_000 / len, fy * 1_000 / len);
                let mut arrived = true;
                let mut t = 0i64;
                for (rank, &i) in ms.iter().enumerate() {
                    if rank % 2 == leg % 2 {
                        let side = if t % 2 == 0 { -1 } else { 1 };
                        let lat = side * (700 + (t / 2) * 1_300);
                        let back = t * 700;
                        t += 1;
                        let spot = (goal.0 - ux * back / 1_000 - uy * lat / 1_000, goal.1 - uy * back / 1_000 + ux * lat / 1_000);
                        self.soldiers[i].task = Task::Move { to: spot, gait: Gait::Run };
                        self.soldiers[i].aim = false;
                        if dist(self.soldiers[i].pos, spot) > 1_200 {
                            arrived = false;
                        }
                    } else {
                        let p = self.soldiers[i].pos;
                        self.soldiers[i].task = Task::Hold { at: p, face: kc };
                        self.soldiers[i].aim = true;
                    }
                }
                if arrived || self.brains[squad].leg_ms >= LEG_TIMEOUT_MS {
                    self.brains[squad].leg += 1;
                    self.brains[squad].leg_ms = 0;
                }
            }
            Some(Method::Drone) => {
                let op = *ms.last().expect("someone");
                if phase_ms == LEAD_MS {
                    let p = self.soldiers[op].pos;
                    self.soldiers[op].task = Task::Hold { at: p, face: kc };
                    self.soldiers[op].act = Some((Act::Drone, self.time_ms + DRONE_MS));
                }
                if phase_ms >= DRONE_MS {
                    // The drone has looked into every room: everyone inside is now known.
                    for j in 0..self.soldiers.len() {
                        let t = &self.soldiers[j];
                        if t.team != self.soldiers[op].team && t.alive() && !t.down() && k.room_at(t.pos.0, t.pos.1).is_some() {
                            let pos = t.pos;
                            self.report_pub(squad, j, pos, 900);
                        }
                    }
                    self.brains[squad].drone_done = true;
                    self.finish(squad, None);
                }
            }
            Some(Method::Stack) => {
                let mut ready = true;
                for (rank, &i) in ms.iter().enumerate() {
                    if rank < 4 {
                        self.soldiers[i].task = Task::Move { to: stack[rank], gait: Gait::Crouch };
                        self.soldiers[i].aim = rank == 0;
                        if dist(self.soldiers[i].pos, stack[rank]) > 700 {
                            ready = false;
                        }
                    } else {
                        // Rear security: a short arc past the end of the stack, one man on along the wall, the next
                        // a step out from it, watching outward and down the flank.
                        let r = rank as i64 - 4;
                        let along = ((stack[3].0 - stack[2].0).signum(), (stack[3].1 - stack[2].1).signum());
                        let out = ((stack[0].0 - d.x_mm).signum() * (1 - along.0.abs()), (stack[0].1 - d.y_mm).signum() * (1 - along.1.abs()));
                        let a = STACK_GAP_MM * (1 + r / 2);
                        let o = (r % 2) * 1_300;
                        let back = (stack[3].0 + along.0 * a + out.0 * o, stack[3].1 + along.1 * a + out.1 * o);
                        let look = if r % 2 == 0 { (out.0 * 10_000 + along.0 * 4_000, out.1 * 10_000 + along.1 * 4_000) } else { (along.0 * 10_000 + out.0 * 3_000, along.1 * 10_000 + out.1 * 3_000) };
                        self.soldiers[i].task = Task::Hold { at: back, face: (back.0 + look.0, back.1 + look.1) };
                        self.soldiers[i].aim = true;
                    }
                }
                if ready || phase_ms >= STACK_TIMEOUT_MS {
                    self.finish(squad, Some(STACKED));
                }
            }
            Some(Method::Grenade) => {
                let thrower = ms[0];
                if phase_ms == LEAD_MS {
                    self.soldiers[thrower].act = Some((Act::Throw, self.time_ms + THROW_MS));
                    self.brains[squad].grenade_used = true;
                    let target = k.rooms[entry_room].centre();
                    self.brains[squad].blast_at = Some((self.time_ms + THROW_MS + FUSE_MS, target));
                    self.log.push(Event::Bark { soldier: thrower, bark: crate::battle::Bark::Frag });
                }
                if phase_ms >= THROW_MS + FUSE_MS + 500 {
                    self.finish(squad, None);
                }
            }
            Some(m @ (Method::Storm | Method::SlowEntry)) => {
                let spots = corners(k, entry_room);
                let mut inside = true;
                for (rank, &i) in ms.iter().enumerate().take(4) {
                    let start_ms = if m == Method::SlowEntry { rank as i64 * 2_500 } else { 0 };
                    if phase_ms >= start_ms {
                        let gait = if m == Method::Storm { Gait::Run } else { Gait::Crouch };
                        self.soldiers[i].task = Task::Move { to: spots[rank], gait };
                        self.soldiers[i].aim = true;
                    }
                    if k.room_at(self.soldiers[i].pos.0, self.soldiers[i].pos.1) != Some(entry_room) {
                        inside = false;
                    }
                }
                if inside || phase_ms >= ROOM_TIMEOUT_MS {
                    // Clear every room, the entry room first (getting in does not mean it is clear).
                    self.brains[squad].rooms = plan.clear_order(door);
                    self.brains[squad].room = 0;
                    self.finish(squad, Some(INSIDE));
                }
            }
            Some(Method::Clear) => {
                let rooms = self.brains[squad].rooms.clone();
                let r = self.brains[squad].room;
                self.brains[squad].room_ms += LEAD_MS;
                if r >= rooms.len() {
                    self.finish(squad, Some(CLEARED));
                    return;
                }
                let room = rooms[r];
                let spots = corners(k, room);
                // Two men take the room; the rest wait where they are, watching it.
                let mut in_room = 0;
                for (rank, &i) in ms.iter().enumerate() {
                    if rank < 2 {
                        self.soldiers[i].task = Task::Move { to: spots[rank * 2], gait: Gait::Walk };
                        self.soldiers[i].aim = true;
                        if k.room_at(self.soldiers[i].pos.0, self.soldiers[i].pos.1) == Some(room) {
                            in_room += 1;
                        }
                    } else if !matches!(self.soldiers[i].task, Task::Hold { .. }) || rank < 4 {
                        let p = self.soldiers[i].pos;
                        self.soldiers[i].task = Task::Hold { at: p, face: k.rooms[room].centre() };
                        self.soldiers[i].aim = true;
                    }
                }
                let hostile = self.soldiers.iter().any(|t| t.alive() && !t.down() && t.team != self.soldiers[ms[0]].team && k.room_at(t.pos.0, t.pos.1) == Some(room));
                // A room is clear when nobody hostile is left in it; it is never given up while someone is
                // (if the pair falls, the next two in line take over: ranks follow the living).
                if !hostile && (in_room >= 2.min(ms.len()) && self.brains[squad].room_ms >= 2_000 || self.brains[squad].room_ms >= ROOM_TIMEOUT_MS) {
                    self.log.push(Event::Order { squad, step: "room clear" });
                    self.brains[squad].room += 1;
                    self.brains[squad].room_ms = 0;
                }
            }
            Some(Method::Consolidate) => {
                // One or two men per room, watching the doors.
                for (rank, &i) in ms.iter().enumerate() {
                    let room = rank % k.rooms.len();
                    let at = corners(k, room)[rank / k.rooms.len() % 4];
                    let face = k.doors.iter().filter(|d| d.rooms.0 == Some(room) || d.rooms.1 == Some(room)).map(|d| (d.x_mm, d.y_mm)).next().unwrap_or(kc);
                    self.soldiers[i].task = Task::Hold { at, face };
                    self.soldiers[i].aim = true;
                }
                self.brains[squad].done |= 1 << HOLDING;
                self.brains[squad].holding = true;
                self.log.push(Event::Order { squad, step: "holding" });
            }
        }
    }
}
