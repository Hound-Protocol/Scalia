//! Layout-limited boarding. Everything is a pure function of (seed, group, slot,
//! interval), so any query time gives the same answer however often it is asked.

use crate::groups::GroupId;
use crate::hash::{hash4, range_i64, STREAM_SEAT, STREAM_WALK};
use crate::position::Pos;
use crate::process::{Process, ProcessError, ProcessKind};
use crate::time::WorldMs;

pub const RAMP_MILLI_PER_S: u64 = 800;
pub const DOOR_MILLI_PER_S: u64 = 1200;
pub const CORRIDOR_MILLI_PER_CM: u64 = 15;
pub const WALK_BASE_MS: u64 = 12_000;
pub const RAMP_CLIMB_MS: u64 = 6_000;
pub const SEAT_BASE_MS: u64 = 8_000;
pub const JITTER_PERCENT: i64 = 20;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ShipLayout {
    pub ramps: u16,
    pub doors: u16,
    pub corridor_width_cm: u16,
    pub seat_cols: u16,
}

impl ShipLayout {
    /// Soldiers per second times 1000: the narrowest of ramps, doors and corridor.
    pub fn throughput_milli_per_s(&self) -> Result<u64, ProcessError> {
        if self.seat_cols == 0 {
            return Err(ProcessError::ZeroSeatCols);
        }
        let ramps = self.ramps as u64 * RAMP_MILLI_PER_S;
        let doors = self.doors as u64 * DOOR_MILLI_PER_S;
        let corridor = self.corridor_width_cm as u64 * CORRIDOR_MILLI_PER_CM;
        let min = ramps.min(doors).min(corridor);
        if min == 0 {
            return Err(ProcessError::ZeroThroughput);
        }
        Ok(min)
    }
}

/// Milliseconds between consecutive queue slots: `ceil(1_000_000 / throughput)`.
pub fn interval_ms(throughput_milli_per_s: u64) -> u64 {
    (1_000_000 + throughput_milli_per_s - 1) / throughput_milli_per_s
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BoardTimes {
    pub start_walk_ms: u64,
    pub reach_ramp_ms: u64,
    pub top_ramp_ms: u64,
    pub seated_ms: u64,
}

fn jittered(base: u64, h: u64) -> u64 {
    let pct = (100 - JITTER_PERCENT + range_i64(h, 0, 2 * JITTER_PERCENT + 1)) as u64;
    base * pct / 100
}

/// Offsets from the process start. The queue position is the slot index, so
/// dead or wounded slots leave gaps in the queue.
pub fn board_times(seed: u64, gid: GroupId, slot: u32, interval_ms: u64) -> BoardTimes {
    let start_walk_ms = (slot as u64).saturating_mul(interval_ms);
    let walk = jittered(
        WALK_BASE_MS,
        hash4(seed, STREAM_WALK, gid.0 as u64, slot as u64),
    );
    let seat = jittered(
        SEAT_BASE_MS,
        hash4(seed, STREAM_SEAT, gid.0 as u64, slot as u64),
    );
    let reach_ramp_ms = start_walk_ms.saturating_add(walk);
    let top_ramp_ms = reach_ramp_ms.saturating_add(RAMP_CLIMB_MS);
    let seated_ms = top_ramp_ms.saturating_add(seat);
    BoardTimes {
        start_walk_ms,
        reach_ramp_ms,
        top_ramp_ms,
        seated_ms,
    }
}

impl Process {
    pub fn board(
        start: WorldMs,
        ship: Pos,
        layout: ShipLayout,
        seed: u64,
        gid: GroupId,
        healthy_slots: impl Iterator<Item = u32>,
    ) -> Result<Process, ProcessError> {
        let interval = interval_ms(layout.throughput_milli_per_s()?);
        let duration_ms = healthy_slots
            .map(|s| board_times(seed, gid, s, interval).seated_ms)
            .max()
            .unwrap_or(0);
        Ok(Process {
            kind: ProcessKind::Board {
                ship,
                layout,
                interval_ms: interval,
            },
            start,
            duration_ms,
        })
    }
}

/// Soldiers seated at time `t`.
pub fn seated_count(
    seed: u64,
    gid: GroupId,
    healthy_slots: impl Iterator<Item = u32>,
    interval_ms: u64,
    start: WorldMs,
    t: WorldMs,
) -> u32 {
    if t.0 < start.0 {
        return 0;
    }
    let elapsed = t.since(start);
    healthy_slots
        .filter(|&s| board_times(seed, gid, s, interval_ms).seated_ms <= elapsed)
        .count() as u32
}
