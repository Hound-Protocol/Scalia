use scalia_core::boarding::ShipLayout;
use scalia_core::fixed::Fix;
use scalia_core::groups::{GroupId, GroupSpec};
use scalia_core::position::{Pos, RegionId};
use scalia_core::process::{Process, ProcessError};
use scalia_core::time::WorldMs;
use scalia_core::units::{Composition, Needs};
use scalia_core::world::{TickError, World};

fn pos(x: i64) -> Pos {
    Pos { region: RegionId(0), x, y: 0, z: 0 }
}

fn spawn(w: &mut World, c: [u32; 4]) -> GroupId {
    let spec = GroupSpec { pos: pos(0), composition: Composition(c), needs: Needs::full() };
    w.groups.spawn(spec, &mut w.roster)
}

fn pct(n: i64) -> i64 {
    Fix::from_ratio(n, 100).unwrap().0
}

#[test]
fn needs_drain_one_game_hour() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [150, 5, 0, 0]);
    for s in 1..=150u64 {
        w.tick(WorldMs(s * 1000)).unwrap();
    }
    let n = *w.groups.needs(id);
    assert!((n.food.0 - (Fix::ONE.0 - pct(4))).abs() <= 256, "food {}", n.food.0);
    assert!((n.upkeep.0 - (Fix::ONE.0 - pct(1))).abs() <= 256);
    assert!((n.sleep.0 - (Fix::ONE.0 - pct(6))).abs() <= 256);
    assert_eq!(n.morale, Fix::ONE);
}

#[test]
fn needs_never_negative() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    // 10 game days = 10 * 60 real minutes, one tick per 10 s to keep the test fast.
    for s in 1..=(10 * 3600 / 10) as u64 {
        w.tick(WorldMs(s * 10_000)).unwrap();
    }
    let n = w.groups.needs(id);
    for m in [n.food, n.sleep, n.morale, n.upkeep] {
        assert!(m.0 >= 0);
    }
    assert_eq!(n.food, Fix::ZERO);
}

#[test]
fn tick_completes_march() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.start_march(id, pos(5000), 1000, WorldMs(0)).unwrap();
    w.tick(WorldMs(4999)).unwrap();
    assert_eq!(w.groups.pos(id), pos(0));
    w.tick(WorldMs(5000)).unwrap();
    assert_eq!(w.groups.pos(id), pos(5000));
    assert_eq!(*w.groups.process(id), Process::idle());
}

#[test]
fn tick_backwards_errors() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    w.tick(WorldMs(10)).unwrap();
    let before = *w.groups.needs(id);
    assert_eq!(w.tick(WorldMs(5)), Err(TickError::TimeWentBackwards));
    assert_eq!(w.now, WorldMs(10));
    assert_eq!(*w.groups.needs(id), before);
}

#[test]
fn empty_and_zero_soldier_world() {
    let mut w = World::new(1);
    w.tick(WorldMs(1000)).unwrap();
    spawn(&mut w, [0, 0, 5, 2]);
    w.tick(WorldMs(2000)).unwrap();
}

#[test]
fn start_boarding_error_leaves_group_unchanged() {
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    let bad = ShipLayout { ramps: 0, doors: 2, corridor_width_cm: 100, seat_cols: 10 };
    assert_eq!(w.start_boarding(id, pos(0), bad, WorldMs(0)), Err(ProcessError::ZeroThroughput));
    assert_eq!(*w.groups.process(id), Process::idle());
}

#[test]
fn boarding_skips_dead_slots_in_the_record() {
    use scalia_core::units::Health;
    let mut w = World::new(1);
    let id = spawn(&mut w, [10, 0, 0, 0]);
    let (start, _) = w.groups.roster_range(id);
    for slot in 0..9 {
        w.roster.rec_mut(start, slot).health = Health::Dead;
    }
    let ok = ShipLayout { ramps: 1, doors: 1, corridor_width_cm: 100, seat_cols: 10 };
    w.start_boarding(id, pos(0), ok, WorldMs(0)).unwrap();
    // Only slot 9 boards, so the queue wait is 9 intervals (1_250 ms each at 800 milli/s).
    assert!(w.groups.process(id).duration_ms >= 9 * 1250);
}
