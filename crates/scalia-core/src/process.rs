//! Processes are timestamped records: state at any time is a pure function of
//! the record and the query time, never of how often it was ticked.

use crate::boarding::ShipLayout;
use crate::flight::{flight_pos_raw, FlightPlan};
use crate::position::{dist_mm, Pos};
use crate::time::WorldMs;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ProcessError {
    ZeroSpeed,
    ZeroThroughput,
    ZeroSeatCols,
    /// A march or boarding that would cross regions; there is no teleporting.
    RegionMismatch,
    /// The group already has a process that has not finished at the requested start time.
    Busy,
    /// The destination body does not exist in the system.
    UnknownBody,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ProcessKind {
    Idle,
    March { from: Pos, to: Pos, speed_mm_s: u32 },
    Board { ship: Pos, layout: ShipLayout, interval_ms: u64 },
    Flight { from: Pos, to: Pos },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Process {
    pub kind: ProcessKind,
    pub start: WorldMs,
    pub duration_ms: u64,
}

impl Process {
    /// True while a non-idle process has not reached its scheduled end at time `t`.
    pub fn is_running_at(&self, t: WorldMs) -> bool {
        self.kind != ProcessKind::Idle && t < self.start.saturating_add(self.duration_ms)
    }

    pub fn idle() -> Process {
        Process { kind: ProcessKind::Idle, start: WorldMs(0), duration_ms: 0 }
    }

    pub fn flight(plan: &FlightPlan) -> Process {
        Process {
            kind: ProcessKind::Flight { from: plan.from, to: plan.to },
            start: plan.start,
            duration_ms: plan.duration_ms,
        }
    }

    /// `duration_ms = ceil(distance_mm * 1000 / speed_mm_s)`.
    pub fn march(
        from: Pos,
        to: Pos,
        speed_mm_s: u32,
        start: WorldMs,
    ) -> Result<Process, ProcessError> {
        if speed_mm_s == 0 {
            return Err(ProcessError::ZeroSpeed);
        }
        if from.region != to.region {
            return Err(ProcessError::RegionMismatch);
        }
        let d = dist_mm(from, to) as u128;
        let s = speed_mm_s as u128;
        let duration_ms = ((d * 1000 + s - 1) / s).min(u64::MAX as u128) as u64;
        Ok(Process { kind: ProcessKind::March { from, to, speed_mm_s }, start, duration_ms })
    }
}

/// Position of a marching group at time `t`; `from` before the start, `to` after the end.
/// `None` for processes that do not move the group (the caller uses the group's own position).
pub fn march_pos(p: &Process, t: WorldMs) -> Option<Pos> {
    match p.kind {
        ProcessKind::March { from, to, .. } => {
            let elapsed = t.since(p.start);
            if elapsed >= p.duration_ms {
                return Some(to);
            }
            Some(from.lerp(to, elapsed, p.duration_ms).unwrap_or(to))
        }
        ProcessKind::Idle | ProcessKind::Board { .. } | ProcessKind::Flight { .. } => None,
    }
}

/// Position of a marching or flying group at time `t`; `None` for processes that do not move it.
pub fn process_pos(p: &Process, t: WorldMs) -> Option<Pos> {
    match p.kind {
        ProcessKind::Flight { from, to } => Some(flight_pos_raw(from, to, p.start, p.duration_ms, t)),
        _ => march_pos(p, t),
    }
}
