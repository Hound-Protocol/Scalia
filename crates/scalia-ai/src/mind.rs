//! A soldier's mind, as in F.E.A.R.: what he knows (facts), what he wants most right now (goals with live
//! priorities) and how he gets it (a short plan from a library of actions, by the GOAP planner). Every
//! perception tick he rebuilds his facts, re-scores his goals and keeps or replaces his plan; a grenade, a flank
//! or an empty magazine changes the facts and with them the plan. Nothing here moves or shoots: the plan's
//! current action sets controls (where to take cover, whether he may fire, whether to reload, whether to put
//! covering fire down) that movement and fire obey.
//!
//! Adding a behaviour is adding an action (preconditions, effects, cost) and, if it is a new want, a goal.
//! Deterministic: integer priorities and costs, the planner's fixed tie-breaks.

use crate::goap::{self, Action, Goal, Limits, State};

/// Facts (bit numbers in the planner's state).
pub mod fact {
    /// An enemy in his sights.
    pub const TARGET: u32 = 0;
    pub const IN_COVER: u32 = 1;
    /// There is cover in reach worth running for.
    pub const COVER_NEAR: u32 = 2;
    /// Rounds are coming his way.
    pub const UNDER_FIRE: u32 = 3;
    /// He can afford to stand and fight here: in cover, not under fire, the enemy right on top of him, in an
    /// assault (the leader's bounding is the answer to fire), inside a building, or no cover in reach (nothing
    /// better to do).
    pub const SAFE: u32 = 4;
    /// Enough rounds in the magazine to fight with.
    pub const MAG_OK: u32 = 5;
    pub const SPARE_MAGS: u32 = 6;
    /// The squad knows where an enemy is (not in his sights).
    pub const CONTACT: u32 = 7;
    /// Goal facts: fighting the enemy, the order carried on.
    pub const ENGAGED: u32 = 8;
    pub const ORDER_DONE: u32 = 9;
}

/// What a soldier can want.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Want {
    /// Get out of the fire.
    Survive,
    /// Fight the enemy he sees (or knows of).
    Engage,
    /// A full magazine.
    Reload,
    /// Carry out his part of the squad's order.
    Order,
    /// Busy with a casualty (the squad leader's care decides).
    Aid,
    /// Nothing to do.
    Idle,
}

/// The action library.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Act {
    /// Sprint to the best cover in reach, holding fire on the way.
    TakeCover,
    /// A fresh magazine (where it is safe to).
    Reload,
    /// A fresh magazine where he stands, rounds coming in or not (when there is nothing better).
    ReloadExposed,
    /// Fight the enemy in his sights.
    ReturnFire,
    /// Covering fire on where the squad knows an enemy is.
    Suppress,
    /// Do what the squad leader has him doing.
    FollowOrder,
}

impl Act {
    pub fn name(self) -> &'static str {
        match self {
            Act::TakeCover => "take cover",
            Act::Reload => "reload",
            Act::ReloadExposed => "reload in the open",
            Act::ReturnFire => "return fire",
            Act::Suppress => "suppress",
            Act::FollowOrder => "follow order",
        }
    }
}

impl Want {
    pub fn name(self) -> &'static str {
        match self {
            Want::Survive => "SURVIVE",
            Want::Engage => "ENGAGE",
            Want::Reload => "RELOAD",
            Want::Order => "ORDER",
            Want::Aid => "AID",
            Want::Idle => "IDLE",
        }
    }
}

const ACTS: [Act; 6] = [Act::TakeCover, Act::Reload, Act::ReloadExposed, Act::ReturnFire, Act::Suppress, Act::FollowOrder];

fn library() -> Vec<Action> {
    use fact::*;
    vec![
        Action::new("take cover", 2).needs(COVER_NEAR, true).needs(IN_COVER, false).sets(IN_COVER, true).sets(SAFE, true),
        Action::new("reload", 2).needs(SPARE_MAGS, true).needs(MAG_OK, false).needs(SAFE, true).sets(MAG_OK, true),
        Action::new("reload in the open", 6).needs(SPARE_MAGS, true).needs(MAG_OK, false).sets(MAG_OK, true),
        Action::new("return fire", 1).needs(TARGET, true).needs(MAG_OK, true).needs(SAFE, true).sets(ENGAGED, true),
        Action::new("suppress", 3).needs(CONTACT, true).needs(TARGET, false).needs(MAG_OK, true).needs(SAFE, true).sets(ENGAGED, true),
        Action::new("follow order", 1).needs(SAFE, true).sets(ORDER_DONE, true),
    ]
}

/// What he is about now: his top want and the plan for it (the first action is the one being done).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mind {
    pub want: Want,
    pub plan: Vec<Act>,
}

impl Default for Mind {
    fn default() -> Mind {
        Mind { want: Want::Idle, plan: Vec::new() }
    }
}

impl Mind {
    /// "SURVIVE: take cover → reload → return fire".
    pub fn describe(&self) -> String {
        if self.plan.is_empty() {
            return self.want.name().into();
        }
        format!("{}: {}", self.want.name(), self.plan.iter().map(|a| a.name()).collect::<Vec<_>>().join(" → "))
    }
    /// The action being done now.
    pub fn doing(&self) -> Option<Act> {
        self.plan.first().copied()
    }
}

/// What feeds a soldier's goals besides the facts.
#[derive(Copy, Clone, Debug, Default)]
pub struct Situation {
    pub facts: u64,
    /// Rounds coming in (0..1000).
    pub suppression: i32,
    /// The enemy in his sights is very close.
    pub target_close: bool,
    /// The magazine is empty.
    pub mag_empty: bool,
    /// His rifle is up (the leader has him watching): he may put covering fire down.
    pub aim: bool,
    /// Busy with a casualty.
    pub aiding: bool,
}

/// How much he wants each thing now (‰), highest first; wants with nothing behind them score 0.
fn wants(s: &Situation) -> Vec<(i32, Want)> {
    use fact::*;
    let f = |b: u32| s.facts >> b & 1 == 1;
    let mut w = vec![
        (if s.aiding { 980 } else { 0 }, Want::Aid),
        (if f(UNDER_FIRE) && !f(SAFE) { 900 + s.suppression / 10 } else { 0 }, Want::Survive),
        (if s.mag_empty && f(SPARE_MAGS) { 960 } else { 0 }, Want::Reload),
        (if f(TARGET) { if s.target_close { 950 } else { 700 } } else if f(CONTACT) && s.aim { 300 } else { 0 }, Want::Engage),
        (200, Want::Order),
    ];
    w.sort_by(|a, b| b.0.cmp(&a.0));
    w
}

fn goal_of(want: Want) -> Option<Goal> {
    use fact::*;
    match want {
        Want::Survive => Some(Goal::new().want(SAFE, true)),
        Want::Engage => Some(Goal::new().want(ENGAGED, true)),
        Want::Reload => Some(Goal::new().want(MAG_OK, true)),
        Want::Order => Some(Goal::new().want(ORDER_DONE, true)),
        Want::Aid | Want::Idle => None,
    }
}

thread_local! {
    /// The action library, built once.
    static LIBRARY: Vec<Action> = library();
    /// Plans already found, by (facts, want): a handful of distinct situations come up over and over, so
    /// nearly every plan is a lookup. (Deterministic: the planner gives the same plan for the same input.)
    static PLANS: std::cell::RefCell<std::collections::HashMap<(u64, u8), Option<Vec<Act>>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn plan_for(facts: u64, want: Want, goal: Goal) -> Option<Vec<Act>> {
    let key = (facts, want as u8);
    if let Some(p) = PLANS.with(|m| m.borrow().get(&key).cloned()) {
        return p;
    }
    let p = LIBRARY.with(|lib| goap::plan(State(facts), goal, lib, Limits { max_nodes: 256, max_depth: 5 }).map(|p| p.actions.iter().map(|&k| ACTS[k]).collect::<Vec<Act>>()));
    PLANS.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 4_096 {
            m.clear();
        }
        m.insert(key, p.clone());
    });
    p
}

/// Thinks: the most wanted thing he can make a plan for, and that plan. Wants with no plan possible (no
/// cover, no magazines) fall through to the next.
pub fn think(s: &Situation) -> Mind {
    for (score, want) in wants(s) {
        if score <= 0 {
            continue;
        }
        let Some(goal) = goal_of(want) else { return Mind { want, plan: Vec::new() } };
        if goal.met(State(s.facts)) {
            // Already so (safe already, say): nothing to do for it; the next want gets a turn unless it is the
            // order, which then just carries on.
            if want == Want::Order {
                return Mind { want, plan: vec![Act::FollowOrder] };
            }
            continue;
        }
        if let Some(plan) = plan_for(s.facts, want, goal) {
            return Mind { want, plan };
        }
    }
    Mind::default()
}

#[cfg(test)]
mod tests {
    use super::fact::*;
    use super::*;

    fn facts(on: &[u32]) -> u64 {
        on.iter().fold(0, |a, &b| a | 1 << b)
    }

    #[test]
    fn shot_at_in_the_open_with_an_empty_magazine_he_runs_reloads_and_fights() {
        let s = Situation { facts: facts(&[TARGET, COVER_NEAR, UNDER_FIRE, SPARE_MAGS]), suppression: 300, ..Default::default() };
        let m = think(&s);
        assert_eq!(m.want, Want::Survive);
        assert_eq!(m.plan, vec![Act::TakeCover]);
        // In cover now (safe): the empty magazine first, then the fight.
        let s = Situation { facts: facts(&[TARGET, IN_COVER, SAFE, UNDER_FIRE, SPARE_MAGS]), mag_empty: true, ..Default::default() };
        let m = think(&s);
        assert_eq!(m.want, Want::Reload);
        assert_eq!(m.plan, vec![Act::Reload]);
        let s = Situation { facts: facts(&[TARGET, IN_COVER, SAFE, MAG_OK, SPARE_MAGS]), ..Default::default() };
        assert_eq!(think(&s).plan, vec![Act::ReturnFire]);
    }

    #[test]
    fn engaging_from_the_open_under_fire_he_gets_to_cover_first() {
        // A target, rounds coming in, low on rounds, cover near: the plan chains cover, reload, fire.
        let s = Situation { facts: facts(&[TARGET, COVER_NEAR, UNDER_FIRE, SPARE_MAGS]), suppression: 50, ..Default::default() };
        let lib_goal = Goal::new().want(ENGAGED, true);
        let p = goap::plan(State(s.facts), lib_goal, &library(), Limits::default()).unwrap();
        let acts: Vec<Act> = p.actions.iter().map(|&k| ACTS[k]).collect();
        assert_eq!(acts, vec![Act::TakeCover, Act::Reload, Act::ReturnFire]);
    }

    #[test]
    fn with_the_enemy_on_top_of_him_he_fights_where_he_stands() {
        let s = Situation { facts: facts(&[TARGET, COVER_NEAR, UNDER_FIRE, SAFE, MAG_OK, SPARE_MAGS]), target_close: true, suppression: 500, ..Default::default() };
        let m = think(&s);
        assert_eq!((m.want, m.plan), (Want::Engage, vec![Act::ReturnFire]));
    }

    #[test]
    fn no_cover_in_reach_he_fights_from_where_he_is() {
        // Nothing to run for counts as safe (it is the best he can do): he fights.
        let s = Situation { facts: facts(&[TARGET, UNDER_FIRE, SAFE, MAG_OK, SPARE_MAGS]), suppression: 400, ..Default::default() };
        let m = think(&s);
        assert_eq!((m.want, m.plan), (Want::Engage, vec![Act::ReturnFire]));
    }

    #[test]
    fn quiet_and_watching_he_puts_covering_fire_on_a_known_contact() {
        let s = Situation { facts: facts(&[CONTACT, SAFE, MAG_OK, SPARE_MAGS]), aim: true, ..Default::default() };
        let m = think(&s);
        assert_eq!((m.want, m.plan), (Want::Engage, vec![Act::Suppress]));
        assert_eq!(think(&Situation { aim: false, ..s }).want, Want::Order);
    }
}
