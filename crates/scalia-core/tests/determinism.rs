use scalia_core::derive::ViewBox;
use scalia_core::position::RegionId;
use scalia_core::scenario::build_scenario;
use scalia_core::state_hash::{derive_hash, state_hash};
use scalia_core::time::WorldMs;
use scalia_core::world::World;

const GOLDEN_STATE_HASH: u64 = 0xcc763f567496bd24;

fn run(seed: u64) -> World {
    let mut w = build_scenario(seed, 200, 150);
    for s in 1..=10u64 {
        w.tick(WorldMs(s * 1000)).unwrap();
    }
    w
}

fn view() -> ViewBox {
    ViewBox {
        region: RegionId(1),
        min_x: -1_000_000,
        max_x: 6_000_000,
        min_y: -1_000_000,
        max_y: 6_000_000,
    }
}

#[test]
fn same_seed_same_hash() {
    let (a, b) = (run(42), run(42));
    assert_eq!(state_hash(&a), state_hash(&b));
    let t = WorldMs(10_000);
    assert_eq!(
        derive_hash(&a, t, &view(), 50_000),
        derive_hash(&b, t, &view(), 50_000)
    );
}

#[test]
fn different_seed_different_hash() {
    let (a, b) = (run(42), run(43));
    assert_ne!(state_hash(&a), state_hash(&b));
    let t = WorldMs(10_000);
    assert_ne!(
        derive_hash(&a, t, &view(), 50_000),
        derive_hash(&b, t, &view(), 50_000)
    );
}

#[test]
fn tick_schedule_independent_of_runs() {
    assert_eq!(state_hash(&run(42)), state_hash(&run(42)));
}

#[test]
fn scenario_shape() {
    let w = build_scenario(42, 200, 150);
    assert_eq!(w.groups.len(), 200);
    assert_eq!(w.roster.len(), 200 * 150);
}

#[test]
fn golden_scenario_hash() {
    let h = state_hash(&run(42));
    assert_eq!(h, GOLDEN_STATE_HASH, "actual state hash: {h:#018x}");
}
