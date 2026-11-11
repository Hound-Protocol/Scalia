//! The world: groups, roster and the 1 Hz tick.

use crate::boarding::ShipLayout;
use crate::fixed::Fix;
use crate::groups::{GroupId, Groups};
use crate::position::Pos;
use crate::process::{Process, ProcessError, ProcessKind};
use crate::roster::Roster;
use crate::time::WorldMs;
use crate::units::{Health, FOOD_DRAIN_PCT, SLEEP_DRAIN_PCT, UPKEEP_DRAIN_PCT};

const GAME_HOUR_MS: i128 = 3_600_000;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TickError {
    TimeWentBackwards,
}

pub struct World {
    pub seed: u64,
    pub now: WorldMs,
    pub groups: Groups,
    pub roster: Roster,
}

fn drain(meter: Fix, rate_pct: i64, game_dt_ms: u64) -> Fix {
    // rate_pct percent of a full meter per game hour, floored, clamped at zero.
    let per_hour = Fix::from_ratio(rate_pct, 100).map(|f| f.0).unwrap_or(0) as i128;
    let amount = per_hour * game_dt_ms as i128 / GAME_HOUR_MS;
    Fix((meter.0 as i128 - amount).max(0) as i64)
}

impl World {
    pub fn new(seed: u64) -> World {
        World { seed, now: WorldMs(0), groups: Groups::new(), roster: Roster::new() }
    }

    pub fn tick(&mut self, now: WorldMs) -> Result<(), TickError> {
        if now < self.now {
            return Err(TickError::TimeWentBackwards);
        }
        let game_dt = WorldMs(now.since(self.now)).game_ms();
        for n in self.groups.needs_all_mut() {
            n.food = drain(n.food, FOOD_DRAIN_PCT, game_dt);
            n.sleep = drain(n.sleep, SLEEP_DRAIN_PCT, game_dt);
            n.upkeep = drain(n.upkeep, UPKEEP_DRAIN_PCT, game_dt);
        }
        for i in 0..self.groups.len() {
            let id = GroupId(i as u32);
            let p = *self.groups.process(id);
            if let ProcessKind::March { to, .. } = p.kind {
                if p.start.saturating_add(p.duration_ms) <= now {
                    self.groups.set_pos(id, to);
                    self.groups.set_process(id, Process::idle());
                }
            }
        }
        self.now = now;
        Ok(())
    }

    pub fn start_march(
        &mut self,
        id: GroupId,
        to: Pos,
        speed_mm_s: u32,
        start: WorldMs,
    ) -> Result<(), ProcessError> {
        let p = Process::march(self.groups.pos(id), to, speed_mm_s, start)?;
        self.groups.set_process(id, p);
        Ok(())
    }

    pub fn start_boarding(
        &mut self,
        id: GroupId,
        ship: Pos,
        layout: ShipLayout,
        start: WorldMs,
    ) -> Result<(), ProcessError> {
        let (rstart, len) = self.groups.roster_range(id);
        let roster = &self.roster;
        let healthy = (0..len).filter(|&s| roster.rec(rstart, s).health == Health::Healthy);
        let p = Process::board(start, ship, layout, self.seed, id, healthy)?;
        self.groups.set_process(id, p);
        Ok(())
    }
}
