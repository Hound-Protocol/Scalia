//! The chain of command and the AI captain's doctrine.
//!
//! Every unit answers to the player (the battalion commander) until it is attached to a captain. A captain is his
//! own rank: he takes a goal from the player ("take that building", "hold that ridge"), plans the steps with GOAP
//! (`goap`), and leads his squads through them from a little behind. `step_orders` turns each step into orders for
//! the squads; `captain_post` says where the captain stands while they carry them out. Positions are millimetres.

use crate::goap::{self, Action, Facts, Goal};
use crate::groups::GroupId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct Command {
    captains: BTreeSet<GroupId>,
    superior: BTreeMap<GroupId, GroupId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// Squads can only be attached to a captain.
    NotACaptain,
    /// Captains are not attached to anyone: they answer to the commander.
    CaptainsAnswerToYou,
}

impl Command {
    pub fn add_captain(&mut self, c: GroupId) {
        self.superior.remove(&c);
        self.captains.insert(c);
    }
    pub fn is_captain(&self, g: GroupId) -> bool {
        self.captains.contains(&g)
    }
    pub fn captains(&self) -> Vec<GroupId> {
        self.captains.iter().copied().collect()
    }
    /// Puts a squad under a captain (moving it from any captain it had).
    pub fn attach(&mut self, squad: GroupId, captain: GroupId) -> Result<(), CommandError> {
        if self.captains.contains(&squad) {
            return Err(CommandError::CaptainsAnswerToYou);
        }
        if !self.captains.contains(&captain) {
            return Err(CommandError::NotACaptain);
        }
        self.superior.insert(squad, captain);
        Ok(())
    }
    /// The squad answers to the commander directly again.
    pub fn detach(&mut self, squad: GroupId) {
        self.superior.remove(&squad);
    }
    /// The captain a squad answers to; None means the commander.
    pub fn superior(&self, squad: GroupId) -> Option<GroupId> {
        self.superior.get(&squad).copied()
    }
    pub fn squads_of(&self, captain: GroupId) -> Vec<GroupId> {
        self.superior.iter().filter(|(_, &c)| c == captain).map(|(&s, _)| s).collect()
    }
}

/// What the commander asks of a captain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Take the place: get there and clear it.
    Take,
    /// Hold the place: get there and dig in.
    Hold,
}

/// The captain's actions (GOAP action ids).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Move to the line of departure, short of the objective, squads spread across it.
    Approach,
    /// One squad takes cover and lays down fire on the objective; the others wait, low.
    Suppress,
    /// Under the covering fire, the other squads rush the objective.
    Assault,
    /// No enemy known: the squads move onto the objective at a walk, ready.
    Advance,
    /// No enemy known: walk straight onto the place to hold it.
    Occupy,
    /// Take up positions in cover on the objective.
    DigIn,
}

const STEPS: [Step; 6] = [Step::Approach, Step::Suppress, Step::Assault, Step::Advance, Step::Occupy, Step::DigIn];

// Facts in the captain's world.
/// The radar shows the enemy at the objective (whatever the radar shows, the units know).
pub const ENEMY: Facts = 1;
pub const STAGED: Facts = 2;
pub const SUPPRESSING: Facts = 4;
pub const ON_OBJECTIVE: Facts = 8;
pub const CLEARED: Facts = 16;
pub const DUG_IN: Facts = 32;

/// The doctrine: what each step needs, does and costs.
pub fn doctrine() -> [Action; 6] {
    let a = |s: Step, need, need_not, add, remove, cost| Action { id: s as u8, need, need_not, add, remove, cost };
    [
        a(Step::Approach, 0, STAGED | ON_OBJECTIVE, STAGED, 0, 2),
        a(Step::Suppress, STAGED | ENEMY, SUPPRESSING, SUPPRESSING, 0, 1),
        a(Step::Assault, STAGED | SUPPRESSING, 0, ON_OBJECTIVE | CLEARED, SUPPRESSING | STAGED, 3),
        a(Step::Advance, STAGED, ENEMY, ON_OBJECTIVE | CLEARED, STAGED, 2),
        a(Step::Occupy, 0, ENEMY | ON_OBJECTIVE, ON_OBJECTIVE, STAGED, 3),
        a(Step::DigIn, ON_OBJECTIVE, DUG_IN, DUG_IN, 0, 1),
    ]
}

fn goal_of(kind: Kind) -> Goal {
    match kind {
        Kind::Take => Goal { want: ON_OBJECTIVE | CLEARED, want_not: 0 },
        Kind::Hold => Goal { want: ON_OBJECTIVE | DUG_IN, want_not: 0 },
    }
}

/// A captain's current task: the goal, where, and the plan he is working through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub kind: Kind,
    pub objective: (i64, i64),
    pub plan: Vec<Step>,
    pub at: usize,
    pub facts: Facts,
}

impl Task {
    /// Plans the task; None if the doctrine has no way to reach the goal.
    pub fn new(kind: Kind, objective: (i64, i64), enemy_known: bool) -> Option<Task> {
        let facts = if enemy_known { ENEMY } else { 0 };
        let plan = plan_steps(facts, kind)?;
        Some(Task { kind, objective, plan, at: 0, facts })
    }
    pub fn current(&self) -> Option<Step> {
        self.plan.get(self.at).copied()
    }
    /// The current step is done: its effects become facts and the next step begins.
    pub fn advance(&mut self) {
        if let Some(s) = self.current() {
            self.facts = doctrine()[s as usize].apply(self.facts);
            self.at += 1;
        }
    }
    pub fn done(&self) -> bool {
        self.at >= self.plan.len()
    }
    /// The radar picture changed: plan the rest again from what is already done. False if no plan remains.
    pub fn replan(&mut self, enemy_known: bool) -> bool {
        let facts = if enemy_known { self.facts | ENEMY } else { self.facts & !ENEMY };
        match plan_steps(facts, self.kind) {
            Some(rest) => {
                self.facts = facts;
                self.plan.truncate(self.at);
                self.plan.extend(rest);
                true
            }
            None => false,
        }
    }
}

fn plan_steps(facts: Facts, kind: Kind) -> Option<Vec<Step>> {
    let ids = goap::plan(facts, goal_of(kind), &doctrine())?;
    Some(ids.into_iter().map(|i| STEPS[i as usize]).collect())
}

/// How a squad should stand once it is where it was sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Stand,
    Crouch,
    Cover,
    CoverAim,
    StandAim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SquadOrder {
    pub squad: GroupId,
    pub to: (i64, i64),
    pub stance: Stance,
    pub run: bool,
}

/// How far short of the objective the line of departure lies (mm).
pub const STAGE_MM: f64 = 40_000.0;
/// Spacing between squads across the line of departure (mm).
pub const SPREAD_MM: f64 = 12_000.0;
/// Spacing between assaulting squads on the objective (mm).
pub const ASSAULT_SPREAD_MM: f64 = 8_000.0;
/// How far the captain stays behind his squads (mm).
pub const POST_BACK_MM: f64 = 15_000.0;

fn mean(points: impl Iterator<Item = (i64, i64)>) -> (f64, f64) {
    let (mut x, mut y, mut n) = (0.0, 0.0, 0.0);
    for p in points {
        x += p.0 as f64;
        y += p.1 as f64;
        n += 1.0;
    }
    if n == 0.0 { (0.0, 0.0) } else { (x / n, y / n) }
}

/// Unit vector from `from` towards `to` (east if they coincide).
fn toward(from: (f64, f64), to: (i64, i64)) -> (f64, f64) {
    let (dx, dy) = (to.0 as f64 - from.0, to.1 as f64 - from.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 { (1.0, 0.0) } else { (dx / len, dy / len) }
}

fn round(p: (f64, f64)) -> (i64, i64) {
    (p.0.round() as i64, p.1.round() as i64)
}

/// Orders for every squad for one step, given the objective and where each squad is now. The lowest-numbered
/// squad is the support squad; squads keep their left-to-right order across the line so they never cross.
pub fn step_orders(step: Step, objective: (i64, i64), squads: &[(GroupId, (i64, i64))]) -> Vec<SquadOrder> {
    let mut sq: Vec<(GroupId, (i64, i64))> = squads.to_vec();
    sq.sort_by_key(|s| s.0);
    if sq.is_empty() {
        return Vec::new();
    }
    let m = mean(sq.iter().map(|s| s.1));
    let u = toward(m, objective);
    let side = (-u.1, u.0);
    // Left to right across the approach (ties by squad number).
    let across = |set: &[(GroupId, (i64, i64))]| {
        let mut v = set.to_vec();
        v.sort_by(|a, b| {
            let pa = a.1 .0 as f64 * side.0 + a.1 .1 as f64 * side.1;
            let pb = b.1 .0 as f64 * side.0 + b.1 .1 as f64 * side.1;
            pa.total_cmp(&pb).then(a.0.cmp(&b.0))
        });
        v
    };
    let line = |set: &[(GroupId, (i64, i64))], at: (f64, f64), spread: f64, stance: Stance, run: bool| {
        let n = set.len() as f64;
        across(set)
            .into_iter()
            .enumerate()
            .map(|(k, (g, _))| {
                let off = (k as f64 - (n - 1.0) / 2.0) * spread;
                SquadOrder { squad: g, to: round((at.0 + side.0 * off, at.1 + side.1 * off)), stance, run }
            })
            .collect::<Vec<_>>()
    };
    let o = (objective.0 as f64, objective.1 as f64);
    let stay = |g: GroupId, at: (i64, i64), stance: Stance| SquadOrder { squad: g, to: at, stance, run: false };
    match step {
        Step::Approach => {
            let dist = ((o.0 - m.0).powi(2) + (o.1 - m.1).powi(2)).sqrt();
            let back = STAGE_MM.min(dist);
            line(&sq, (o.0 - u.0 * back, o.1 - u.1 * back), SPREAD_MM, Stance::Crouch, false)
        }
        Step::Suppress => {
            let mut out = vec![stay(sq[0].0, sq[0].1, Stance::CoverAim)];
            out.extend(sq[1..].iter().map(|&(g, p)| stay(g, p, Stance::Crouch)));
            out
        }
        Step::Assault => {
            let rest = if sq.len() > 1 { &sq[1..] } else { &sq[..] };
            line(rest, o, ASSAULT_SPREAD_MM, Stance::StandAim, true)
        }
        Step::Advance => line(&sq, o, ASSAULT_SPREAD_MM, Stance::Crouch, false),
        Step::Occupy => line(&sq, o, SPREAD_MM, Stance::Crouch, false),
        Step::DigIn => sq.iter().map(|&(g, p)| stay(g, p, Stance::Cover)).collect(),
    }
}

/// Where the captain stands: a little behind where his squads are going, on the line from them to the objective.
pub fn captain_post(objective: (i64, i64), targets: &[(i64, i64)]) -> (i64, i64) {
    let m = mean(targets.iter().copied());
    let u = toward(m, objective);
    round((m.0 - u.0 * POST_BACK_MM, m.1 - u.1 * POST_BACK_MM))
}
