//! Fixed battles: the same setup for the game, the examples, the tests and the native-versus-WebAssembly check.
use crate::battle::{Battle, Soldier, Stance};
use crate::leader::Order;
use crate::save::{Command, MapSpec};

/// Where the enemy stands inside the training house (mm from its south-west corner).
pub const KILLHOUSE_DEFENDERS: [(i64, i64); 8] = [(12_500, 3_500), (16_000, 2_000), (5_000, 2_000), (2_500, 5_000), (3_000, 10_600), (6_500, 12_000), (16_500, 10_200), (13_000, 12_500)];
pub const KILLHOUSE_ATTACKERS: usize = 12;

/// The training assault: squad 0 (12 men, 27 m south of the house) is ordered to take it; squad 1 (8 men
/// inside) to hold it. `house` is the house's south-west corner (mm). The orders go in as commands at 0 ms.
pub fn killhouse_assault(house: (i64, i64), seed: u64) -> Battle {
    let mut soldiers: Vec<Soldier> = (0..KILLHOUSE_ATTACKERS as i64).map(|i| Soldier::new(0, 0, (house.0 + 6_000 + (i % 4) * 1_500, house.1 - 27_000 + (i / 4) * 1_500), Stance::Stand)).collect();
    for (x, y) in KILLHOUSE_DEFENDERS {
        soldiers.push(Soldier::new(1, 1, (house.0 + x, house.1 + y), Stance::Stand));
    }
    let mut b = Battle::on_map(MapSpec::killhouse(house), soldiers, seed);
    b.submit(0, Command::Squad { squad: 0, order: Order::Capture });
    b.submit(0, Command::Squad { squad: 1, order: Order::Hold });
    b
}

/// The hash the native-versus-WebAssembly check compares: the assault at (80, 62) km after `ms` of fighting.
pub fn killhouse_hash(seed: u64, ms: i64) -> u64 {
    let mut b = killhouse_assault((80_000, 62_000), seed);
    while b.time_ms < ms {
        b.step(50);
    }
    b.hash()
}
