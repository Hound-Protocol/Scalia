//! Order-fixed hashes of the whole state and of derived individuals, used by the
//! determinism tests and the native-versus-WebAssembly check.

use crate::derive::{derive_visible, Derived, ViewBox};
use crate::groups::GroupId;
use crate::hash::mix64;
use crate::position::Pos;
use crate::process::{Process, ProcessKind};
use crate::time::WorldMs;
use crate::world::World;

fn fold(h: u64, v: u64) -> u64 {
    mix64(h ^ v)
}

fn fold_pos(h: u64, p: Pos) -> u64 {
    let h = fold(h, p.region.0 as u64);
    let h = fold(h, p.x as u64);
    let h = fold(h, p.y as u64);
    fold(h, p.z as u64)
}

fn fold_process(h: u64, p: &Process) -> u64 {
    let h = match p.kind {
        ProcessKind::Idle => fold(h, 0),
        ProcessKind::March { from, to, speed_mm_s } => {
            fold(fold_pos(fold_pos(fold(h, 1), from), to), speed_mm_s as u64)
        }
        ProcessKind::Board { ship, layout, interval_ms } => {
            let h = fold_pos(fold(h, 2), ship);
            let h = fold(h, layout.ramps as u64);
            let h = fold(h, layout.doors as u64);
            let h = fold(h, layout.corridor_width_cm as u64);
            let h = fold(h, layout.seat_cols as u64);
            fold(h, interval_ms)
        }
        ProcessKind::Flight { from, to } => fold_pos(fold_pos(fold(h, 3), from), to),
    };
    fold(fold(h, p.start.0), p.duration_ms)
}

pub fn state_hash(w: &World) -> u64 {
    let mut h = fold(fold(0, w.seed), w.now.0);
    for i in 0..w.groups.len() {
        let id = GroupId(i as u32);
        h = fold_pos(h, w.groups.pos(id));
        for c in w.groups.composition(id).0 {
            h = fold(h, c as u64);
        }
        let n = w.groups.needs(id);
        for m in [n.food, n.sleep, n.morale, n.upkeep] {
            h = fold(h, m.0 as u64);
        }
        h = fold_process(h, w.groups.process(id));
        let (start, len) = w.groups.roster_range(id);
        h = fold(fold(h, start as u64), len as u64);
        for slot in 0..len {
            let r = w.roster.rec(start, slot);
            h = fold(h, r.xp as u64 | (r.kills as u64) << 16 | (r.health as u64) << 32);
            h = fold(h, r.flags as u64 | (r.reserved as u64) << 8);
        }
    }
    h
}

pub fn derive_hash(w: &World, t: WorldMs, view: &ViewBox, cap: usize) -> u64 {
    let mut out: Vec<Derived> = Vec::new();
    derive_visible(w, t, view, cap, &mut out);
    let mut h = fold(0, out.len() as u64);
    for d in &out {
        h = fold(h, d.slot as u64 | (d.group.0 as u64) << 32);
        h = fold_pos(h, d.pos);
        h = fold(h, d.phase as u64);
    }
    h
}
