use scalia_ai::battle::{Battle, Event, Gait, Soldier, Stance, Task};
use scalia_ai::doors::{Door, DoorState};
use scalia_ai::save::{Command, MapSpec};
use scalia_ai::sight::Point;
use scalia_core::nav::{Prop, PropKind};

/// 60 x 60 m with a plywood wall along y = 30 m from x = 10 to 50 m, and doorways (1.4 m) at x = 20 and 40 m.
fn ground(doors: &[(i64, DoorState)]) -> MapSpec {
    let panel = |x0: i64, x1: i64| Prop { kind: PropKind::Panel { length_mm: x1 - x0, window: false }, x_mm: (x0 + x1) / 2, y_mm: 30_000, quarter_turns: 0 };
    let props = [panel(10_000, 19_300), panel(20_700, 39_300), panel(40_700, 50_000)];
    let mut m = MapSpec::new(0, 0, 60_000, 60_000, &props);
    m.doors = doors.iter().map(|&(x, s)| Door::new((x, 30_000), true, 1_400, 2_100, s)).collect();
    m
}

fn walker(x: i64) -> Soldier {
    Soldier::new(0, 0, (x, 25_000), Stance::Stand)
}

fn eye(x: i64, y: i64) -> Point {
    Point { x, y, z: 1_600 }
}

#[test]
fn a_shut_door_blocks_the_view_through_its_doorway_and_an_open_one_does_not() {
    let mut b = Battle::on_map(ground(&[(20_000, DoorState::Closed)]), vec![], 1);
    let (a, c) = (eye(20_000, 25_000), eye(20_000, 35_000));
    assert!(!b.sight.clear(a, c), "closed");
    // Over the top of the door it is open (the doorway is open above 2.1 m: a man on a roof would see).
    assert!(b.sight.clear(Point { z: 2_300, ..a }, Point { z: 2_300, ..c }));
    b.set_door(0, DoorState::Open, None);
    assert!(b.sight.clear(a, c), "open");
    b.set_door(0, DoorState::Breached, None);
    assert!(b.sight.clear(a, c), "breached");
}

#[test]
fn a_man_walking_into_a_closed_door_stops_opens_it_and_walks_on() {
    let mut b = Battle::on_map(ground(&[(20_000, DoorState::Closed)]), vec![walker(20_000)], 1);
    b.order(0, Task::Move { to: (20_000, 36_000), gait: Gait::Walk });
    let mut opened_at = None;
    for _ in 0..400 {
        b.step(50);
        if opened_at.is_none() && b.doors[0].state == DoorState::Open {
            opened_at = Some(b.time_ms);
            assert!((b.soldiers[0].pos.1 - 30_000).abs() <= 600, "he opened it standing at it: {:?}", b.soldiers[0].pos);
        }
    }
    let at = opened_at.expect("he opened the door");
    assert!(b.log.iter().any(|e| matches!(e, Event::Door { door: 0, state: DoorState::Open, by: Some(0) })), "logged as his");
    assert!(b.soldiers[0].pos.1 > 35_000, "and went through: {:?} (opened at {at} ms)", b.soldiers[0].pos);
}

#[test]
fn a_locked_door_turns_the_route_another_way() {
    let mut b = Battle::on_map(ground(&[(20_000, DoorState::Locked), (40_000, DoorState::Open)]), vec![walker(20_000)], 1);
    b.order(0, Task::Move { to: (20_000, 36_000), gait: Gait::Run });
    let mut crossed_at = None;
    for _ in 0..1_200 {
        let before = b.soldiers[0].pos;
        b.step(50);
        if before.1 < 30_000 && b.soldiers[0].pos.1 >= 30_000 {
            crossed_at = Some(b.soldiers[0].pos.0);
        }
    }
    assert!(b.soldiers[0].pos.1 > 35_000, "he got through: {:?}", b.soldiers[0].pos);
    let x = crossed_at.expect("he crossed the wall's line");
    assert!((x - 20_000).abs() > 1_000, "not through the locked doorway (crossed at x = {x})");
    assert_eq!(b.doors[0].state, DoorState::Locked, "nobody opened a locked door");
}

#[test]
fn doors_are_set_by_command_and_kept_in_saves_and_replays() {
    let mut b = Battle::on_map(ground(&[(20_000, DoorState::Open)]), vec![walker(30_000)], 1);
    b.submit(1_000, Command::Door { door: 0, state: DoorState::Locked });
    for _ in 0..40 {
        b.step(50);
    }
    assert_eq!(b.doors[0].state, DoorState::Locked);
    let saved = b.save();
    let mut r = Battle::load(&saved).expect("loads");
    assert_eq!(r.doors[0].state, DoorState::Locked, "the save keeps it");
    assert!(!r.sight.clear(eye(20_000, 25_000), eye(20_000, 35_000)), "and its sight");
    // The same steps on both give the same battle.
    for _ in 0..40 {
        b.step(50);
        r.step(50);
    }
    assert_eq!(b.save(), r.save());
}
