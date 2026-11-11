//! A star system: a star and bodies on circular orbits, each a pure function of time.

use crate::fixed::Fix;
use crate::position::{Pos, RegionId};
use crate::time::WorldMs;
use crate::trig::sin_cos_turns;

pub const SYSTEM_REGION: RegionId = RegionId(100);

/// Where a ship parks relative to a body, in millimetres.
pub const PARK_OFFSET: (i64, i64, i64) = (0, 50_000_000, 0);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BodyId(pub u8);

#[derive(Clone, Debug)]
pub struct Body {
    pub name: &'static str,
    pub orbit_radius_mm: i64,
    /// Real milliseconds for one orbit; 0 means the body does not move.
    pub period_ms: u64,
    /// Starting angle in turns, 0 up to 1.
    pub phase: Fix,
}

#[derive(Clone, Debug)]
pub struct System {
    pub star: Pos,
    pub bodies: Vec<Body>,
}

impl System {
    pub fn demo() -> System {
        System {
            star: Pos { region: SYSTEM_REGION, x: 0, y: 0, z: 0 },
            bodies: vec![
                Body {
                    name: "Aster",
                    orbit_radius_mm: 40_000_000_000,
                    period_ms: 600_000,
                    phase: Fix::ZERO,
                },
                Body {
                    name: "Brine",
                    orbit_radius_mm: 90_000_000_000,
                    period_ms: 1_800_000,
                    phase: Fix::from_ratio(1, 4).unwrap(),
                },
            ],
        }
    }

    pub fn body_pos(&self, id: BodyId, t: WorldMs) -> Option<Pos> {
        let b = self.bodies.get(id.0 as usize)?;
        let frac = if b.period_ms == 0 {
            0
        } else {
            (((t.0 % b.period_ms) as u128) << 32) / b.period_ms as u128
        } as i64;
        let (sin, cos) = sin_cos_turns(Fix(b.phase.0.wrapping_add(frac)));
        let r = b.orbit_radius_mm as i128;
        let dx = ((r * cos.0 as i128) >> 32) as i64;
        let dy = ((r * sin.0 as i128) >> 32) as i64;
        Some(self.star.offset(dx, dy, 0))
    }
}
