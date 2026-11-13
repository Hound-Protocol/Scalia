//! The squad leader: turns the commander's intent into a plan and runs it as drills.
//!
//! The plan comes from the GOAP planner over facts about the building (are we there, do we know what is inside,
//! is someone waiting in the entry room, are we stacked, inside, has it been cleared). Action costs are set from
//! the situation before every plan (a drone is worth it when blind, storming when we outnumber them, a grenade
//! when the entry room is occupied), and the leader plans again whenever a step finishes, so new intel changes
//! what happens next. Each step is a drill that hands the soldiers their tasks; the soldiers' own survival
//! (cover when pinned, fighting what they see) runs underneath it.

use crate::battle::{Act, Battle, Event, Gait, Task};
use crate::goap::{self, Action, Goal, Limits, State};
use crate::sight::isqrt;
use crate::tactics::Building;
use scalia_core::killhouse::Killhouse;

/// The commander's intent for a squad.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Order {
    None,
    /// Take the building and hold it.
    Capture,
    /// Hold the building (or wherever they stand): defend.
    Hold,
}

/// One step of a building assault.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug)]
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
}

impl Brain {
    pub fn new() -> Brain {
        Brain { order: Order::None, plan: Vec::new(), done: 0, phase_ms: 0, entry: None, legs: Vec::new(), leg: 0, leg_ms: 0, rooms: Vec::new(), room: 0, room_ms: 0, drone_done: false, grenade_used: false, blast_at: None, holding: false }
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
        self.brains[squad] = Brain { order, ..Brain::new() };
    }

    /// The squad has done what it was told (the building taken and held).
    pub fn squad_done(&self, squad: usize) -> bool {
        self.brains.get(squad).is_some_and(|b| b.holding)
    }

    /// The step the squad is on (for labels and tests).
    pub fn squad_phase(&self, squad: usize) -> &'static str {
        self.brains.get(squad).and_then(|b| b.plan.first()).map_or("", |m| m.name())
    }

    fn members(&self, squad: usize) -> Vec<usize> {
        self.soldiers.iter().enumerate().filter(|(_, s)| s.squad as usize == squad && s.alive()).map(|(i, _)| i).collect()
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
        let inside: Vec<(i64, i64)> = self.contacts[squad].iter().filter(|c| c.confidence >= 300 && self.soldiers[c.enemy].alive() && k.room_at(c.pos.0, c.pos.1).is_some()).map(|c| c.pos).collect();
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
    pub(crate) fn lead(&mut self) {
        for squad in 0..self.brains.len() {
            match self.brains[squad].order {
                Order::None => {}
                Order::Hold => self.lead_hold(squad),
                Order::Capture => {
                    if self.building.is_some() && !self.members(squad).is_empty() {
                        self.lead_capture(squad);
                    }
                }
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
        for i in self.members(squad) {
            if self.soldiers[i].task == Task::Idle {
                let (p, f) = (self.soldiers[i].pos, self.soldiers[i].facing);
                self.soldiers[i].task = Task::Hold { at: p, face: (p.0 + f.0 * 5, p.1 + f.1 * 5) };
            }
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
        let stack = plan.entries[e].stack.clone();
        let door = plan.entries[e].door;
        let d = k.doors[door];
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
                // Fire teams by rank: team (leg % 2) bounds to the next point, the other watches the building.
                let goal = legs[leg];
                let mut arrived = true;
                for (rank, &i) in ms.iter().enumerate() {
                    if rank % 2 == leg % 2 {
                        let off = ((rank as i64 / 2) - 1) * 1_500;
                        let spot = (goal.0 + off, goal.1 - (rank as i64 % 2) * 1_000);
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
                        if t.team != self.soldiers[op].team && t.alive() && k.room_at(t.pos.0, t.pos.1).is_some() {
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
                        // Rear security behind the stack, facing away from the house.
                        let back = (stack[3].0 + (stack[3].0 - d.x_mm).signum() * 1_500 * (rank as i64 - 3), stack[3].1 + (stack[3].1 - d.y_mm).signum() * 1_500 * (rank as i64 - 3));
                        self.soldiers[i].task = Task::Hold { at: back, face: (2 * back.0 - kc.0, 2 * back.1 - kc.1) };
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
                let hostile = self.soldiers.iter().any(|t| t.alive() && t.team != self.soldiers[ms[0]].team && k.room_at(t.pos.0, t.pos.1) == Some(room));
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
