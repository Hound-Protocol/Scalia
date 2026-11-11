//! Individuals are a pure function of (group state, time, seed): the server sends
//! groups, and anyone who needs to see people derives them here.

use crate::boarding::board_times;
use crate::groups::GroupId;
use crate::hash::{hash4, range_i64, STREAM_FORMATION, STREAM_FORMATION_Y};
use crate::position::{isqrt, Pos, RegionId};
use crate::process::{march_pos, Process, ProcessKind};
use crate::roster::FLAG_PERSISTENT;
use crate::time::WorldMs;
use crate::units::Health;
use crate::world::World;

pub const FORMATION_SPACING_MM: i64 = 900;
pub const FORMATION_JITTER_MM: i64 = 150;
pub const RAMP_BASE_DX_MM: i64 = -6000;
pub const RAMP_TOP_DZ_MM: i64 = 2500;
pub const SEAT_PITCH_MM: i64 = 600;
pub const VIEW_MARGIN_MM: i64 = 100_000;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Phase {
    Standing = 0,
    Marching = 1,
    Queued = 2,
    Walking = 3,
    Climbing = 4,
    Seated = 5,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Derived {
    pub slot: u32,
    pub group: GroupId,
    pub pos: Pos,
    pub phase: Phase,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ViewBox {
    pub region: RegionId,
    pub min_x: i64,
    pub max_x: i64,
    pub min_y: i64,
    pub max_y: i64,
}

fn grid(n_slots: u32) -> (u32, u32) {
    let n = n_slots.max(1);
    let r = isqrt(n as u128) as u32;
    let cols = if r * r < n { r + 1 } else { r };
    (cols, (n + cols - 1) / cols)
}

/// Offset of a soldier from the group's position, in millimetres. Slot identity is
/// stable: dead slots leave gaps rather than reshuffling the formation.
pub fn formation_offset(seed: u64, gid: GroupId, slot: u32, n_slots: u32) -> (i64, i64) {
    let (cols, rows) = grid(n_slots);
    let (col, row) = ((slot % cols) as i64, (slot / cols) as i64);
    let jx = range_i64(
        hash4(seed, gid.0 as u64, slot as u64, STREAM_FORMATION),
        -FORMATION_JITTER_MM,
        FORMATION_JITTER_MM + 1,
    );
    let jy = range_i64(
        hash4(seed, gid.0 as u64, slot as u64, STREAM_FORMATION_Y),
        -FORMATION_JITTER_MM,
        FORMATION_JITTER_MM + 1,
    );
    let half = FORMATION_SPACING_MM / 2;
    (
        col * FORMATION_SPACING_MM - (cols as i64 - 1) * half + jx,
        row * FORMATION_SPACING_MM - (rows as i64 - 1) * half + jy,
    )
}

fn blend(from: Pos, to: Pos, num: u64, den: u64) -> Pos {
    from.lerp(to, num, den).unwrap_or(to)
}

fn anchor(w: &World, gid: GroupId, p: &Process, t: WorldMs) -> Pos {
    match p.kind {
        ProcessKind::March { .. } => march_pos(p, t).unwrap_or_else(|| w.groups.pos(gid)),
        _ => w.groups.pos(gid),
    }
}

fn derive_slot(
    w: &World,
    gid: GroupId,
    slot: u32,
    t: WorldMs,
    base: Pos,
    roster_start: u32,
    n: u32,
    p: &Process,
) -> Option<Derived> {
    let rec = w.roster.rec(roster_start, slot);
    if rec.health != Health::Healthy || rec.flags & FLAG_PERSISTENT != 0 {
        return None;
    }
    let (dx, dy) = formation_offset(w.seed, gid, slot, n);
    let spot = base.offset(dx, dy, 0);
    let (pos, phase) = match p.kind {
        ProcessKind::Idle => (spot, Phase::Standing),
        ProcessKind::March { .. } => {
            let moving = t >= p.start && t.since(p.start) < p.duration_ms;
            (spot, if moving { Phase::Marching } else { Phase::Standing })
        }
        ProcessKind::Board { ship, layout, interval_ms } => {
            let bt = board_times(w.seed, gid, slot, interval_ms);
            let el = t.since(p.start);
            let ramp_base = ship.offset(RAMP_BASE_DX_MM, 0, 0);
            let ramp_top = ship.offset(0, 0, RAMP_TOP_DZ_MM);
            let cols = layout.seat_cols.max(1) as u32;
            let seat = ship.offset(
                (slot % cols) as i64 * SEAT_PITCH_MM,
                (slot / cols) as i64 * SEAT_PITCH_MM,
                RAMP_TOP_DZ_MM,
            );
            if t < p.start || el < bt.start_walk_ms {
                (spot, Phase::Queued)
            } else if el < bt.reach_ramp_ms {
                let (n, d) = (el - bt.start_walk_ms, bt.reach_ramp_ms - bt.start_walk_ms);
                (blend(spot, ramp_base, n, d), Phase::Walking)
            } else if el < bt.top_ramp_ms {
                let (n, d) = (el - bt.reach_ramp_ms, bt.top_ramp_ms - bt.reach_ramp_ms);
                (blend(ramp_base, ramp_top, n, d), Phase::Climbing)
            } else if el < bt.seated_ms {
                let (n, d) = (el - bt.top_ramp_ms, bt.seated_ms - bt.top_ramp_ms);
                (blend(ramp_top, seat, n, d), Phase::Walking)
            } else {
                (seat, Phase::Seated)
            }
        }
    };
    Some(Derived { slot, group: gid, pos, phase })
}

/// One soldier at time `t`; `None` unless healthy and not a persistent individual.
pub fn derive_soldier(w: &World, gid: GroupId, slot: u32, t: WorldMs) -> Option<Derived> {
    let n = w.groups.people(gid);
    if slot >= n {
        return None;
    }
    let p = *w.groups.process(gid);
    let base = anchor(w, gid, &p, t);
    let (rs, _) = w.groups.roster_range(gid);
    derive_slot(w, gid, slot, t, base, rs, n, &p)
}

fn in_view(p: Pos, v: &ViewBox) -> bool {
    p.region == v.region
        && p.x >= v.min_x.saturating_sub(VIEW_MARGIN_MM)
        && p.x <= v.max_x.saturating_add(VIEW_MARGIN_MM)
        && p.y >= v.min_y.saturating_sub(VIEW_MARGIN_MM)
        && p.y <= v.max_y.saturating_add(VIEW_MARGIN_MM)
}

/// Fills `out` with every derivable soldier in groups near the view, up to `cap`.
pub fn derive_visible(w: &World, t: WorldMs, view: &ViewBox, cap: usize, out: &mut Vec<Derived>) {
    out.clear();
    for i in 0..w.groups.len() {
        if out.len() >= cap {
            break;
        }
        let gid = GroupId(i as u32);
        let n = w.groups.people(gid);
        if n == 0 {
            continue;
        }
        let p = *w.groups.process(gid);
        let base = anchor(w, gid, &p, t);
        let near_ship = matches!(p.kind, ProcessKind::Board { ship, .. } if in_view(ship, view));
        if !in_view(base, view) && !near_ship {
            continue;
        }
        let (rs, _) = w.groups.roster_range(gid);
        for slot in 0..n {
            if out.len() >= cap {
                break;
            }
            if let Some(d) = derive_slot(w, gid, slot, t, base, rs, n, &p) {
                out.push(d);
            }
        }
    }
}
