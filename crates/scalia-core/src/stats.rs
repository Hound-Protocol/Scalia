//! Per-soldier stats for the zoomed view: stored experience plus skills derived
//! from the seed, so a million soldiers need no skill storage.

use crate::groups::GroupId;
use crate::hash::{hash4, range_i64, STREAM_ACCURACY, STREAM_COMPOSURE, STREAM_NAME, STREAM_SPEED};
use crate::units::Health;
use crate::world::World;

pub const SKILL_BASE: i64 = 40;
pub const XP_CAP: i64 = 1000;
pub const XP_PER_POINT: i64 = 50;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SoldierStats {
    pub slot: u32,
    pub group: GroupId,
    pub xp: u16,
    pub kills: u16,
    pub health: Health,
    pub accuracy: u8,
    pub speed: u8,
    pub composure: u8,
    pub name_seed: u64,
}

fn skill(seed: u64, gid: GroupId, slot: u32, stream: u64, xp: u16) -> u8 {
    let h = hash4(seed, gid.0 as u64, slot as u64, stream);
    let v = SKILL_BASE + range_i64(h, 0, 41) + (xp as i64).min(XP_CAP) / XP_PER_POINT;
    v.min(100) as u8
}

/// `None` if the slot is out of range. Dead soldiers still have stats.
pub fn soldier_stats(w: &World, gid: GroupId, slot: u32) -> Option<SoldierStats> {
    if slot >= w.groups.people(gid) {
        return None;
    }
    let (start, _) = w.groups.roster_range(gid);
    let rec = w.roster.rec(start, slot);
    Some(SoldierStats {
        slot,
        group: gid,
        xp: rec.xp,
        kills: rec.kills,
        health: rec.health,
        accuracy: skill(w.seed, gid, slot, STREAM_ACCURACY, rec.xp),
        speed: skill(w.seed, gid, slot, STREAM_SPEED, rec.xp),
        composure: skill(w.seed, gid, slot, STREAM_COMPOSURE, rec.xp),
        name_seed: hash4(w.seed, gid.0 as u64, slot as u64, STREAM_NAME),
    })
}
