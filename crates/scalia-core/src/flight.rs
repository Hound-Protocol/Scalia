//! Eased flights between bodies that lead their moving target.
//! The whole flight is a pure function of the plan and the query time.

use crate::fixed::Fix;
use crate::position::{dist_mm, Pos};
use crate::process::ProcessError;
use crate::system::{BodyId, System, SYSTEM_REGION};
use crate::time::WorldMs;

/// Peak speed used by the demo: 2,000 km/s.
pub const DEMO_PEAK_SPEED_MM_S: u32 = 2_000_000_000;

/// Smootherstep has a largest slope of 1.875, so average speed is peak / 1.875.
const PEAK_NUM: u128 = 1875;
const ITERATIONS: usize = 12;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FlightPlan {
    pub from: Pos,
    pub to: Pos,
    pub start: WorldMs,
    pub duration_ms: u64,
}

/// Smootherstep `6t^5 - 15t^4 + 10t^3` for `t = num/den`, clamped to [0, 1]; `den` 0 gives one.
pub fn ease(num: u64, den: u64) -> Fix {
    if den == 0 || num >= den {
        return Fix::ONE;
    }
    let t = Fix((((num as u128) << 32) / den as u128) as i64);
    let inner = t.mul(Fix::from_int(6)).0 - Fix::from_int(15).0;
    let inner = Fix(t.mul(Fix(inner)).0 + Fix::from_int(10).0);
    t.mul(t).mul(t).mul(inner)
}

pub fn flight_pos_raw(from: Pos, to: Pos, start: WorldMs, duration_ms: u64, t: WorldMs) -> Pos {
    let elapsed = t.since(start);
    if elapsed >= duration_ms {
        return to;
    }
    let e = ease(elapsed, duration_ms);
    from.lerp(to, e.0 as u64, 1 << 32).unwrap_or(to)
}

pub fn flight_pos(plan: &FlightPlan, t: WorldMs) -> Pos {
    flight_pos_raw(plan.from, plan.to, plan.start, plan.duration_ms, t)
}

fn duration_for(distance_mm: u64, peak_speed_mm_s: u32) -> u64 {
    let n = distance_mm as u128 * PEAK_NUM;
    let d = peak_speed_mm_s as u128;
    ((n + d - 1) / d).min(u64::MAX as u128) as u64
}

/// Plans a flight from `from` to `dest` plus `offset`, aiming where the body will be on arrival.
pub fn plan_flight(
    sys: &System,
    from: Pos,
    dest: BodyId,
    offset: (i64, i64, i64),
    peak_speed_mm_s: u32,
    start: WorldMs,
) -> Result<FlightPlan, ProcessError> {
    if peak_speed_mm_s == 0 {
        return Err(ProcessError::ZeroSpeed);
    }
    if from.region != SYSTEM_REGION {
        return Err(ProcessError::RegionMismatch);
    }
    let target_at = |ms: u64| -> Result<Pos, ProcessError> {
        let p = sys
            .body_pos(dest, start.saturating_add(ms))
            .ok_or(ProcessError::UnknownBody)?;
        Ok(p.offset(offset.0, offset.1, offset.2))
    };
    let mut duration_ms = 0u64;
    for _ in 0..ITERATIONS {
        duration_ms = duration_for(dist_mm(from, target_at(duration_ms)?), peak_speed_mm_s);
    }
    let to = target_at(duration_ms)?;
    Ok(FlightPlan {
        from,
        to,
        start,
        duration_ms,
    })
}
