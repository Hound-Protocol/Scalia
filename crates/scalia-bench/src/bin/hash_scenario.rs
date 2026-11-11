//! Prints `state_hash ^ derive_hash` for a fixed scenario, for the native-versus-WebAssembly check.

use scalia_core::derive::ViewBox;
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::state_hash::{derive_hash, state_hash};
use scalia_core::time::WorldMs;

fn main() {
    let seed: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(42);
    let mut w = build_scenario(seed, 200, 150);
    for s in 1..=10u64 {
        w.tick(WorldMs(s * 1000)).unwrap();
    }
    let view = ViewBox {
        region: RegionId(1),
        min_x: -1_000_000,
        max_x: 6_000_000,
        min_y: -1_000_000,
        max_y: 6_000_000,
    };
    println!("{}", state_hash(&w) ^ derive_hash(&w, WorldMs(10_000), &view, 5000));
}
