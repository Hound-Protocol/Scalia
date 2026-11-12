//! Goal-oriented action planning (GOAP): the planner behind the AI captains, after F.E.A.R.
//!
//! The world is a set of facts (bits). An action needs some facts true and some false, then sets and clears
//! others, at a cost. `plan` finds the cheapest chain of actions from the current facts to a goal. It is a
//! uniform-cost search over fact sets: small (a few dozen states for a captain) and deterministic, as ties are
//! broken by the order actions were tried in.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

pub type Facts = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: u8,
    /// Facts that must be true.
    pub need: Facts,
    /// Facts that must be false.
    pub need_not: Facts,
    pub add: Facts,
    pub remove: Facts,
    pub cost: u32,
}

impl Action {
    pub fn applies(&self, s: Facts) -> bool {
        s & self.need == self.need && s & self.need_not == 0
    }
    pub fn apply(&self, s: Facts) -> Facts {
        (s | self.add) & !self.remove
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    pub want: Facts,
    pub want_not: Facts,
}

impl Goal {
    pub fn met(&self, s: Facts) -> bool {
        s & self.want == self.want && s & self.want_not == 0
    }
}

/// Plans never search past this many states; a captain's world has far fewer.
const MAX_STATES: usize = 4096;

/// The cheapest sequence of action ids from `start` to `goal`, or None if it cannot be reached.
pub fn plan(start: Facts, goal: Goal, actions: &[Action]) -> Option<Vec<u8>> {
    // Frontier ordered by (cost, insertion order): equal-cost plans resolve the same way every time.
    let mut open = BinaryHeap::new();
    let mut best: BTreeMap<Facts, u32> = BTreeMap::new();
    let mut came: BTreeMap<Facts, (Facts, u8)> = BTreeMap::new();
    let mut seq: u64 = 0;
    open.push(Reverse((0u32, seq, start)));
    best.insert(start, 0);
    while let Some(Reverse((cost, _, s))) = open.pop() {
        if best.get(&s).is_some_and(|&b| b < cost) {
            continue;
        }
        if goal.met(s) {
            let mut steps = Vec::new();
            let mut at = s;
            while at != start {
                let (prev, id) = came[&at];
                steps.push(id);
                at = prev;
            }
            steps.reverse();
            return Some(steps);
        }
        if best.len() > MAX_STATES {
            return None;
        }
        for a in actions.iter().filter(|a| a.applies(s)) {
            let next = a.apply(s);
            let c = cost + a.cost;
            if best.get(&next).is_none_or(|&b| c < b) {
                best.insert(next, c);
                came.insert(next, (s, a.id));
                seq += 1;
                open.push(Reverse((c, seq, next)));
            }
        }
    }
    None
}
