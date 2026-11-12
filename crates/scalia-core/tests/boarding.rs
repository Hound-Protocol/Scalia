use scalia_core::boarding::*;
use scalia_core::groups::GroupId;
use scalia_core::hash::mix64;
use scalia_core::position::{Pos, RegionId};
use scalia_core::process::{Process, ProcessError, ProcessKind};
use scalia_core::time::WorldMs;

const SEED: u64 = 42;
const GID: GroupId = GroupId(7);

fn layout() -> ShipLayout {
    ShipLayout {
        ramps: 2,
        doors: 3,
        corridor_width_cm: 200,
        seat_cols: 20,
    }
}

fn ship() -> Pos {
    Pos {
        region: RegionId(0),
        x: 0,
        y: 0,
        z: 0,
    }
}

#[test]
fn throughput_is_minimum() {
    assert_eq!(layout().throughput_milli_per_s(), Ok(1600));
    assert_eq!(interval_ms(1600), 625);
}

#[test]
fn zero_layout_errors() {
    let base = layout();
    for l in [
        ShipLayout { ramps: 0, ..base },
        ShipLayout { doors: 0, ..base },
        ShipLayout {
            corridor_width_cm: 0,
            ..base
        },
    ] {
        assert_eq!(
            l.throughput_milli_per_s(),
            Err(ProcessError::ZeroThroughput)
        );
    }
    let l = ShipLayout {
        seat_cols: 0,
        ..base
    };
    assert_eq!(l.throughput_milli_per_s(), Err(ProcessError::ZeroSeatCols));
}

#[test]
fn board_times_ordered_and_bounded() {
    let t = board_times(SEED, GID, 0, 625);
    assert!(t.start_walk_ms <= t.reach_ramp_ms);
    assert!(t.reach_ramp_ms <= t.top_ramp_ms);
    assert!(t.top_ramp_ms <= t.seated_ms);
    let walk = t.reach_ramp_ms - t.start_walk_ms;
    assert!((9_600..=14_400).contains(&walk), "walk {walk}");
    assert_eq!(t.top_ramp_ms - t.reach_ramp_ms, 6_000);
    let seat = t.seated_ms - t.top_ramp_ms;
    assert!((6_400..=9_600).contains(&seat), "seat {seat}");
    assert_eq!(t, board_times(SEED, GID, 0, 625));
}

#[test]
fn queue_position_is_slot_index() {
    let t = board_times(SEED, GID, 10, 625);
    assert_eq!(t.start_walk_ms, 6250);
}

#[test]
fn duration_is_max_seated() {
    let p = Process::board(WorldMs(0), ship(), layout(), SEED, GID, 0..150u32).unwrap();
    let max = (0..150u32)
        .map(|s| board_times(SEED, GID, s, 625).seated_ms)
        .max()
        .unwrap();
    assert_eq!(p.duration_ms, max);
    match p.kind {
        ProcessKind::Board { interval_ms, .. } => assert_eq!(interval_ms, 625),
        _ => panic!("expected a boarding process"),
    }
}

#[test]
fn five_thousand_boarding_query_invariant() {
    let start = WorldMs(10_000);
    let p = Process::board(start, ship(), layout(), SEED, GID, 0..5000u32).unwrap();
    let count = |t: u64| seated_count(SEED, GID, 0..5000u32, 625, start, WorldMs(t));
    assert_eq!(count(start.0 - 1), 0);
    let times: Vec<u64> = (0..200u64)
        .map(|i| 10_000 + i * (p.duration_ms / 199))
        .collect();
    let in_order: Vec<u32> = times.iter().map(|&t| count(t)).collect();
    // Same times visited in a shuffled order must give the same answers.
    let mut idx: Vec<usize> = (0..times.len()).collect();
    idx.sort_by_key(|&i| mix64(i as u64));
    for &i in &idx {
        assert_eq!(count(times[i]), in_order[i]);
    }
    assert!(in_order.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(count(start.0 + p.duration_ms), 5000);
}

#[test]
fn all_dead_group() {
    let p = Process::board(WorldMs(0), ship(), layout(), SEED, GID, std::iter::empty()).unwrap();
    assert_eq!(p.duration_ms, 0);
    assert_eq!(
        seated_count(
            SEED,
            GID,
            std::iter::empty(),
            625,
            WorldMs(0),
            WorldMs(u64::MAX)
        ),
        0
    );
}

#[test]
fn gaps_delay_the_queue() {
    // Slots 0 and 100 healthy: the second still waits 100 intervals.
    let slots = [0u32, 100];
    let p = Process::board(
        WorldMs(0),
        ship(),
        layout(),
        SEED,
        GID,
        slots.iter().copied(),
    )
    .unwrap();
    assert!(p.duration_ms >= 100 * 625);
}

#[test]
fn query_extremes() {
    let start = WorldMs(10_000);
    assert_eq!(
        seated_count(SEED, GID, 0..150u32, 625, start, WorldMs(0)),
        0
    );
    assert_eq!(
        seated_count(SEED, GID, 0..150u32, 625, start, WorldMs(u64::MAX)),
        150
    );
}
