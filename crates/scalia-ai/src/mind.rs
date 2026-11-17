//! A soldier's mind: two-stage deterministic decision core.
//!
//! Stage 1 (`score_intents`) uses integer tactical scoring (permille utilities over exposure, suppression,
//! injuries, ammo, contact confidence and aid status) to choose *what* the soldier should pursue right now.
//! Stage 2 (GOAP) plans *how* to achieve that intent from the action library with bounded deterministic
//! search. Nothing here moves or shoots directly: the plan's current action sets explicit controls (cover,
//! break-contact, reload, fire and suppression permissions) that `Battle` obeys.

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
    /// Genuine protection or low danger: in cover, indoors, close-quarters engagement, or not under fire.
    /// Absence of cover under fire sets `EXPOSED`, never `SAFE`.
    pub const SAFE: u32 = 4;
    /// Enough rounds in the magazine to fight with.
    pub const MAG_OK: u32 = 5;
    pub const SPARE_MAGS: u32 = 6;
    /// The squad knows where an enemy is (not in his sights).
    pub const CONTACT: u32 = 7;
    /// Goal facts: fighting the enemy, the order carried on.
    pub const ENGAGED: u32 = 8;
    pub const ORDER_DONE: u32 = 9;
    /// Under fire in the open with no reachable cover nearby.
    pub const EXPOSED: u32 = 10;
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
    /// Sprint to the best cover in reach, holding fire on the way (except point-blank self-defense).
    TakeCover,
    /// A fresh magazine (where it is safe to).
    Reload,
    /// A fresh magazine where he stands, rounds coming in or not (when there is nothing better).
    ReloadExposed,
    /// Fight the enemy in his sights.
    ReturnFire,
    /// Covering fire on where the squad knows an enemy is.
    Suppress,
    /// Under fire with no cover nearby: return/suppress fire while breaking out toward cover or an exit.
    BreakContact,
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
            Act::BreakContact => "break contact",
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

    fn rank(self) -> u8 {
        match self {
            Want::Aid => 0,
            Want::Reload => 1,
            Want::Survive => 2,
            Want::Engage => 3,
            Want::Order => 4,
            Want::Idle => 5,
        }
    }
}

const ACTS: [Act; 7] = [
    Act::TakeCover,
    Act::Reload,
    Act::ReloadExposed,
    Act::ReturnFire,
    Act::Suppress,
    Act::BreakContact,
    Act::FollowOrder,
];

fn library() -> Vec<Action> {
    use fact::*;
    vec![
        Action::new("take cover", 2).needs(COVER_NEAR, true).needs(IN_COVER, false).sets(IN_COVER, true).sets(SAFE, true),
        Action::new("reload", 2).needs(SPARE_MAGS, true).needs(MAG_OK, false).needs(SAFE, true).sets(MAG_OK, true),
        Action::new("reload in the open", 6).needs(SPARE_MAGS, true).needs(MAG_OK, false).sets(MAG_OK, true),
        Action::new("return fire", 1).needs(TARGET, true).needs(MAG_OK, true).needs(SAFE, true).sets(ENGAGED, true),
        Action::new("suppress", 3).needs(CONTACT, true).needs(TARGET, false).needs(MAG_OK, true).needs(SAFE, true).sets(ENGAGED, true),
        Action::new("break contact", 4).needs(EXPOSED, true).sets(SAFE, true).sets(ENGAGED, true),
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
    /// "SURVIVE: take cover -> reload -> return fire".
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

/// What feeds a soldier's tactical scoring and GOAP assessment.
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
    /// Best usable contact confidence (0..1000) informing engagement/suppression urgency.
    pub threat_confidence: i32,
    /// Injury severity (0 uninjured .. 1000 near-incapacitated).
    pub injury_permille: i32,
}

/// Deterministic integer tactical scoring (permille), highest first; wants scoring <= 0 are inactive.
pub fn score_intents(s: &Situation) -> Vec<(i32, Want)> {
    use fact::*;
    let f = |b: u32| s.facts >> b & 1 == 1;
    let sup = s.suppression.clamp(0, 1_000);
    let injury = s.injury_permille.clamp(0, 1_000);
    let conf = s.threat_confidence.clamp(0, 1_000);
    let survive = if f(UNDER_FIRE) && !f(SAFE) { 900 + sup / 10 + injury / 20 } else { 0 };
    let engage = if f(TARGET) {
        if s.target_close { 950 } else { 700 + conf / 20 }
    } else if f(CONTACT) && s.aim {
        300 + conf / 10
    } else {
        0
    };
    let mut w = vec![
        (if s.aiding { 980 } else { 0 }, Want::Aid),
        (survive, Want::Survive),
        (if s.mag_empty && f(SPARE_MAGS) { 960 } else { 0 }, Want::Reload),
        (engage, Want::Engage),
        (200, Want::Order),
    ];
    w.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.rank().cmp(&b.1.rank())));
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

/// Thinks: integer scoring picks the winning want, and GOAP plans how to fulfill it.
/// Wants with no feasible plan fall through deterministically to the next want.
pub fn think(s: &Situation) -> Mind {
    for (score, want) in score_intents(s) {
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
    fn exposed_under_fire_with_no_cover_he_breaks_contact_rather_than_faking_safe() {
        // No cover in reach is honest exposure (`!SAFE` and `EXPOSED`), never falsely `SAFE`.
        let s = Situation { facts: facts(&[TARGET, UNDER_FIRE, EXPOSED, MAG_OK, SPARE_MAGS]), suppression: 400, ..Default::default() };
        let m = think(&s);
        assert_eq!((m.want, m.plan), (Want::Survive, vec![Act::BreakContact]));

        // When protected (e.g. indoors or in cover), SAFE holds and he fights directly.
        let safe_s = Situation { facts: facts(&[TARGET, UNDER_FIRE, SAFE, MAG_OK, SPARE_MAGS]), suppression: 400, ..Default::default() };
        assert_eq!((think(&safe_s).want, think(&safe_s).plan), (Want::Engage, vec![Act::ReturnFire]));
    }

    #[test]
    fn exposed_and_empty_under_heavy_suppression_prioritizes_survival_over_open_reload() {
        let low_sup = Situation { facts: facts(&[UNDER_FIRE, EXPOSED, SPARE_MAGS]), mag_empty: true, suppression: 200, ..Default::default() };
        assert_eq!((think(&low_sup).want, think(&low_sup).plan), (Want::Reload, vec![Act::ReloadExposed]));

        let pinned = Situation { facts: facts(&[UNDER_FIRE, EXPOSED, SPARE_MAGS]), mag_empty: true, suppression: 700, ..Default::default() };
        assert_eq!((think(&pinned).want, think(&pinned).plan), (Want::Survive, vec![Act::BreakContact]));
    }

    #[test]
    fn quiet_and_watching_he_puts_covering_fire_on_a_known_contact() {
        let s = Situation { facts: facts(&[CONTACT, SAFE, MAG_OK, SPARE_MAGS]), aim: true, threat_confidence: 800, ..Default::default() };
        let m = think(&s);
        assert_eq!((m.want, m.plan), (Want::Engage, vec![Act::Suppress]));
        assert_eq!(think(&Situation { aim: false, ..s }).want, Want::Order);
    }

    #[test]
    fn integer_scoring_breaks_ties_deterministically() {
        let s = Situation { facts: facts(&[UNDER_FIRE, EXPOSED, SPARE_MAGS]), mag_empty: true, suppression: 600, injury_permille: 0, ..Default::default() };
        // Both Reload (960) and Survive (900 + 60 = 960) tie at 960; Reload rank wins deterministically.
        assert_eq!(score_intents(&s)[0], (960, Want::Reload));
        // Any injury breaks the tie cleanly toward survival.
        let hurt = Situation { injury_permille: 200, ..s };
        assert_eq!(score_intents(&hurt)[0], (970, Want::Survive));
    }
}
