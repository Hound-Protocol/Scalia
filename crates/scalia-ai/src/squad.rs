//! An example squad-level domain: the kind of decisions a squad leader makes. Used by the benchmark and as a
//! starting point for the real one; the facts and costs here are placeholders until the AI design is decided.

use crate::goap::{Action, Goal};

pub const ENEMY_KNOWN: u32 = 0;
pub const ENEMY_SUPPRESSED: u32 = 1;
pub const IN_COVER: u32 = 2;
pub const AT_OBJECTIVE: u32 = 3;
pub const FLANK_OPEN: u32 = 4;
pub const ON_FLANK: u32 = 5;
pub const AMMO_LOW: u32 = 6;
pub const WOUNDED: u32 = 7;
pub const ARTILLERY_READY: u32 = 8;
pub const AREA_SHELLED: u32 = 9;
pub const REGROUPED: u32 = 10;
pub const ENEMY_BROKEN: u32 = 11;

pub fn actions() -> Vec<Action> {
    vec![
        Action::new("scout", 3).sets(ENEMY_KNOWN, true),
        Action::new("take cover", 2).sets(IN_COVER, true),
        Action::new("suppress", 3).needs(ENEMY_KNOWN, true).needs(IN_COVER, true).needs(AMMO_LOW, false).sets(ENEMY_SUPPRESSED, true),
        Action::new("call artillery", 4).needs(ENEMY_KNOWN, true).needs(ARTILLERY_READY, true).sets(AREA_SHELLED, true).sets(ARTILLERY_READY, false),
        Action::new("find flank", 4).needs(ENEMY_KNOWN, true).sets(FLANK_OPEN, true),
        Action::new("move to flank", 5).needs(FLANK_OPEN, true).sets(ON_FLANK, true).sets(IN_COVER, false),
        Action::new("bound forward", 6).needs(ENEMY_SUPPRESSED, true).sets(AT_OBJECTIVE, true).sets(IN_COVER, false),
        Action::new("assault from flank", 4).needs(ON_FLANK, true).needs(ENEMY_SUPPRESSED, true).sets(ENEMY_BROKEN, true),
        Action::new("assault after shelling", 3).needs(AREA_SHELLED, true).sets(ENEMY_BROKEN, true),
        Action::new("occupy", 2).needs(ENEMY_BROKEN, true).sets(AT_OBJECTIVE, true),
        Action::new("resupply", 5).sets(AMMO_LOW, false),
        Action::new("treat wounded", 4).needs(IN_COVER, true).sets(WOUNDED, false),
        Action::new("regroup", 2).needs(IN_COVER, true).sets(REGROUPED, true),
        Action::new("frontal charge", 12).needs(ENEMY_KNOWN, true).sets(AT_OBJECTIVE, true).sets(WOUNDED, true),
    ]
}

/// Take the objective with the wounded cared for.
pub fn take_objective() -> Goal {
    Goal::new().want(AT_OBJECTIVE, true).want(WOUNDED, false)
}
