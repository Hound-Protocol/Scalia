//! A repeatable scenario for tests, benchmarks and the first scene: groups on a
//! square grid, some marching, some boarding, the rest standing.

use crate::boarding::ShipLayout;
use crate::groups::GroupSpec;
use crate::position::{isqrt, Pos, RegionId};
use crate::time::WorldMs;
use crate::units::{Composition, Needs};
use crate::world::World;

pub const GRID_SPACING_MM: i64 = 200_000;
pub const MARCH_DISTANCE_MM: i64 = 500_000;
pub const MARCH_SPEED_MM_S: u32 = 1500;
pub const SHIP_OFFSET_X_MM: i64 = 30_000;

pub fn build_scenario(seed: u64, groups: u32, soldiers_per_group: u32) -> World {
    let mut w = World::new(seed);
    let side = {
        let r = isqrt(groups.max(1) as u128) as u32;
        if r * r < groups { r + 1 } else { r }.max(1)
    };
    let layout = ShipLayout {
        ramps: 2,
        doors: 3,
        corridor_width_cm: 200,
        seat_cols: 20,
    };
    for g in 0..groups {
        let pos = Pos {
            region: RegionId(1),
            x: (g % side) as i64 * GRID_SPACING_MM,
            y: (g / side) as i64 * GRID_SPACING_MM,
            z: 0,
        };
        let spec = GroupSpec {
            pos,
            composition: Composition([soldiers_per_group, 0, 0, 0]),
            needs: Needs::full(),
        };
        let id = w.groups.spawn(spec, &mut w.roster);
        match g % 10 {
            0 => {
                let to = pos.offset(MARCH_DISTANCE_MM, 0, 0);
                w.start_march(id, to, MARCH_SPEED_MM_S, WorldMs(0))
                    .expect("valid march");
            }
            1 => {
                let ship = pos.offset(SHIP_OFFSET_X_MM, 0, 0);
                w.start_boarding(id, ship, layout, WorldMs(0))
                    .expect("valid layout");
            }
            _ => {}
        }
    }
    w
}
