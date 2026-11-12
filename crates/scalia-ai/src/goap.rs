//! Goal-oriented action planning (GOAP), as in F.E.A.R.: an agent has a goal (facts it wants true or false),
//! a set of actions with preconditions, effects and costs, and finds the cheapest ordered list of actions that
//! makes the goal true. Forward A* over world states.
//!
//! Deterministic for the shared server: integer costs, states as 64-bit fact sets, and ties broken by insertion
//! order (lower action index first), so the same inputs always give the same plan on every machine. Searches are
//! bounded by a node and a depth budget, so a plan never costs more than a known amount of time.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Up to 64 boolean facts; bit `i` is fact `i`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct State(pub u64);

impl State {
    pub fn get(self, fact: u32) -> bool {
        self.0 >> fact & 1 == 1
    }
}

/// A set of required fact values: (mask of facts that matter, their wanted values).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Goal {
    pub mask: u64,
    pub value: u64,
}

impl Goal {
    pub fn new() -> Goal {
        Goal::default()
    }

    pub fn want(mut self, fact: u32, on: bool) -> Goal {
        self.mask |= 1 << fact;
        if on {
            self.value |= 1 << fact;
        } else {
            self.value &= !(1 << fact);
        }
        self
    }

    pub fn met(&self, s: State) -> bool {
        s.0 & self.mask == self.value
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub name: &'static str,
    pub cost: u32,
    pub pre: Goal,
    pub eff: Goal,
}

impl Action {
    pub fn new(name: &'static str, cost: u32) -> Action {
        Action { name, cost: cost.max(1), pre: Goal::new(), eff: Goal::new() }
    }

    /// Precondition: `fact` must be `on`.
    pub fn needs(mut self, fact: u32, on: bool) -> Action {
        self.pre = self.pre.want(fact, on);
        self
    }

    /// Effect: `fact` becomes `on`.
    pub fn sets(mut self, fact: u32, on: bool) -> Action {
        self.eff = self.eff.want(fact, on);
        self
    }

    pub fn applicable(&self, s: State) -> bool {
        self.pre.met(s)
    }

    pub fn apply(&self, s: State) -> State {
        State(s.0 & !self.eff.mask | self.eff.value)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Limits {
    /// Most states taken off the open list before giving up.
    pub max_nodes: u32,
    /// Longest plan considered.
    pub max_depth: u32,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { max_nodes: 4_096, max_depth: 12 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Indices into the action list, in order.
    pub actions: Vec<usize>,
    pub cost: u32,
}

struct Node {
    state: State,
    depth: u32,
    parent: Option<(usize, usize)>, // (node index, action index)
}

/// The cheapest plan from `start` to `goal`, or `None` if there is none within the limits.
pub fn plan(start: State, goal: Goal, actions: &[Action], limits: Limits) -> Option<Plan> {
    // Admissible heuristic: an unmet goal needs at least one more action.
    let min_cost = actions.iter().map(|a| a.cost).min().unwrap_or(1);
    let h = |s: State| if goal.met(s) { 0 } else { min_cost };

    let mut nodes = vec![Node { state: start, depth: 0, parent: None }];
    let mut best: HashMap<State, u32> = HashMap::from([(start, 0)]);
    // Ordered by (f, g, sequence): cheapest estimate first; among equals, the earliest found.
    let mut open = BinaryHeap::from([Reverse((h(start), 0u32, 0usize))]);
    let mut expanded = 0;

    while let Some(Reverse((_, g, id))) = open.pop() {
        if best.get(&nodes[id].state).is_some_and(|&b| b < g) {
            continue; // a cheaper way to this state was found after this entry was queued
        }
        let state = nodes[id].state;
        if goal.met(state) {
            let mut out = Vec::new();
            let mut at = id;
            while let Some((parent, action)) = nodes[at].parent {
                out.push(action);
                at = parent;
            }
            out.reverse();
            return Some(Plan { actions: out, cost: g });
        }
        expanded += 1;
        if expanded > limits.max_nodes || nodes[id].depth >= limits.max_depth {
            if expanded > limits.max_nodes {
                return None;
            }
            continue;
        }
        let depth = nodes[id].depth + 1;
        for (ai, a) in actions.iter().enumerate() {
            if !a.applicable(state) {
                continue;
            }
            let next = a.apply(state);
            if next == state {
                continue; // no progress
            }
            let ng = g.saturating_add(a.cost);
            if best.get(&next).is_some_and(|&b| b <= ng) {
                continue;
            }
            best.insert(next, ng);
            nodes.push(Node { state: next, depth, parent: Some((id, ai)) });
            open.push(Reverse((ng.saturating_add(h(next)), ng, nodes.len() - 1)));
        }
    }
    None
}
