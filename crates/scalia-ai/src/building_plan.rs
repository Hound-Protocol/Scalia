//! Deterministic building-plan options. Each provider contributes feasible actions to the shared GOAP planner;
//! execution remains in the squad leader.

use crate::goap::{self, Action, Goal, Limits, State};

/// One step of a building assault.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Method {
    /// Before the assault, out of contact: the squad kneels in a half circle in front of its leader and he
    /// goes through the plan, man by man (positions, roles, how they will take it).
    Brief,
    Approach,
    Drone,
    Stack,
    /// Slicing the pie: from the stack the point man steps out in an arc in front of the door, a slice at a
    /// time, rifle on the room, seeing it bit by bit; his number two covers from the stack.
    Pie,
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
            Method::Pie => "pie",
            Method::Brief => "briefing",
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
    /// Out of contact: nobody under fire, no enemy seen lately (time for a briefing).
    pub quiet: bool,
}

pub(crate) const AT: u32 = 0;
pub(crate) const INTEL: u32 = 1;
pub(crate) const ENTRY_HOT: u32 = 2;
pub(crate) const STACKED: u32 = 3;
pub(crate) const INSIDE: u32 = 4;
pub(crate) const CLEARED: u32 = 5;
pub(crate) const HOLDING: u32 = 6;
/// The entry room has been looked into from outside (pied).
pub(crate) const PIED: u32 = 7;
/// The squad has been briefed.
pub(crate) const BRIEFED: u32 = 8;
pub(crate) const QUIET: u32 = 9;

/// An entrance and the tactical evidence used to compare it with other available entrances.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct EntryOption {
    pub index: usize,
    pub route_cost_mm: i64,
    /// Sum of confidence for valid contacts in the room behind the entrance.
    pub reported_threat: i64,
}

/// Prefer less reported threat, then a shorter route, then stable source order.
pub(crate) fn choose_entry(options: &[EntryOption]) -> Option<usize> {
    options
        .iter()
        .min_by_key(|o| (o.reported_threat, o.route_cost_mm, o.index))
        .map(|o| o.index)
}

type OptionProvider = fn(&Situation, &mut Vec<(Action, Method)>);

const OPTION_PROVIDERS: &[OptionProvider] = &[
    add_preparation_options,
    add_staging_options,
    add_entry_options,
    add_objective_options,
    add_recon_options,
    add_breach_options,
];

fn add_preparation_options(_: &Situation, out: &mut Vec<(Action, Method)>) {
    out.extend([
        (
            Action::new("brief", 1)
                .needs(QUIET, true)
                .needs(BRIEFED, false)
                .needs(AT, false)
                .sets(BRIEFED, true),
            Method::Brief,
        ),
        (
            Action::new("approach", 2)
                .needs(BRIEFED, true)
                .sets(AT, true),
            Method::Approach,
        ),
        (
            Action::new("approach unbriefed", 4)
                .needs(BRIEFED, false)
                .sets(AT, true),
            Method::Approach,
        ),
    ]);
}

fn add_staging_options(_: &Situation, out: &mut Vec<(Action, Method)>) {
    out.extend([
        (
            Action::new("stack", 1).needs(AT, true).sets(STACKED, true),
            Method::Stack,
        ),
        // Nobody goes through a door without looking through it first.
        (
            Action::new("pie", 1)
                .needs(STACKED, true)
                .needs(PIED, false)
                .sets(PIED, true),
            Method::Pie,
        ),
    ]);
}

fn add_entry_options(s: &Situation, out: &mut Vec<(Action, Method)>) {
    let ratio = s.strength / s.enemies_known.max(1);
    let storm_cost = if ratio >= 3 {
        2
    } else if ratio >= 2 {
        3
    } else {
        7
    };
    out.extend([
        (
            Action::new("storm", storm_cost)
                .needs(PIED, true)
                .needs(INTEL, true)
                .needs(ENTRY_HOT, false)
                .sets(INSIDE, true),
            Method::Storm,
        ),
        (
            Action::new("slow entry", 6)
                .needs(PIED, true)
                .needs(ENTRY_HOT, false)
                .sets(INSIDE, true),
            Method::SlowEntry,
        ),
        // Going in slowly on top of someone waiting in the entry room is the worst way in.
        (
            Action::new("slow entry, hot", 10)
                .needs(PIED, true)
                .needs(ENTRY_HOT, true)
                .sets(INSIDE, true),
            Method::SlowEntry,
        ),
    ]);
}

fn add_objective_options(_: &Situation, out: &mut Vec<(Action, Method)>) {
    out.extend([
        (
            Action::new("clear", 3)
                .needs(INSIDE, true)
                .sets(CLEARED, true),
            Method::Clear,
        ),
        (
            Action::new("consolidate", 1)
                .needs(CLEARED, true)
                .sets(HOLDING, true),
            Method::Consolidate,
        ),
    ]);
}

fn add_recon_options(s: &Situation, out: &mut Vec<(Action, Method)>) {
    if s.have_drone {
        out.push((
            Action::new("drone", 2)
                .needs(AT, true)
                .needs(STACKED, false)
                .needs(INTEL, false)
                .sets(INTEL, true),
            Method::Drone,
        ));
    }
}

fn add_breach_options(s: &Situation, out: &mut Vec<(Action, Method)>) {
    if s.have_grenade {
        out.push((
            Action::new("grenade", 1)
                .needs(STACKED, true)
                .needs(ENTRY_HOT, true)
                .sets(ENTRY_HOT, false),
            Method::Grenade,
        ));
    }
}

fn options(s: &Situation) -> Vec<(Action, Method)> {
    let mut out = Vec::new();
    for provider in OPTION_PROVIDERS {
        provider(s, &mut out);
    }
    out
}

pub(crate) fn facts(s: &Situation) -> State {
    let mut v = 0u64;
    if s.intel {
        v |= 1 << INTEL;
    }
    if s.enemy_in_entry {
        v |= 1 << ENTRY_HOT;
    }
    if s.quiet {
        v |= 1 << QUIET;
    }
    State(v)
}

/// The leader's plan for taking a building in this situation, from the supplied progress state.
pub(crate) fn plan_from(s: &Situation, start: State) -> Vec<Method> {
    let acts = options(s);
    let actions: Vec<Action> = acts.iter().map(|a| a.0.clone()).collect();
    let goal = Goal::new().want(HOLDING, true);
    match goap::plan(start, goal, &actions, Limits::default()) {
        Some(p) => p.actions.iter().map(|&i| acts[i].1).collect(),
        None => vec![
            Method::Approach,
            Method::Stack,
            Method::Pie,
            Method::SlowEntry,
            Method::Clear,
            Method::Consolidate,
        ],
    }
}

/// The leader's plan for taking a building, starting from the situation's current facts.
pub fn choose(s: &Situation) -> Vec<Method> {
    plan_from(s, facts(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrance_choice_uses_reported_threat_then_route_and_stable_order() {
        let options = [
            EntryOption {
                index: 0,
                route_cost_mm: 2_000,
                reported_threat: 900,
            },
            EntryOption {
                index: 1,
                route_cost_mm: 12_000,
                reported_threat: 0,
            },
            EntryOption {
                index: 2,
                route_cost_mm: 8_000,
                reported_threat: 0,
            },
            EntryOption {
                index: 3,
                route_cost_mm: 8_000,
                reported_threat: 0,
            },
        ];
        assert_eq!(choose_entry(&options), Some(2));
        assert_eq!(
            choose_entry(&options[..2]),
            Some(1),
            "lower reported threat takes priority over route length"
        );
        assert_eq!(choose_entry(&[]), None);
    }
}
