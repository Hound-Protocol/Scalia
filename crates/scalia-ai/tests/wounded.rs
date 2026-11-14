//! Wounded soldiers: hit twice they go down (no moving, no firing) and bleed; enemies stop shooting at them;
//! a squadmate comes, drags them to cover if the enemy is near, and treats them, which stops the bleeding.
use scalia_ai::battle::{Act, Battle, Event, Soldier, Stance, Task, DAMAGE};
use scalia_ai::save::MapSpec;
use scalia_core::nav::{Prop, PropKind};

fn man(team: u8, squad: u16, x: i64, y: i64) -> Soldier {
    Soldier::new(team, squad, (x, y), Stance::Stand)
}

fn run(b: &mut Battle, ms: i64) {
    let end = b.time_ms + ms;
    while b.time_ms < end {
        b.step(50);
    }
}

fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f64).sqrt() as i64
}

#[test]
fn two_hits_put_a_man_down_and_he_bleeds_out_alone() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), vec![man(0, 0, 50_000, 50_000)], 1);
    b.soldiers[0].health = 100 - 2 * DAMAGE;
    b.step(50);
    assert!(b.soldiers[0].down() && b.soldiers[0].alive(), "down, not dead");
    run(&mut b, 10_000);
    assert!(b.soldiers[0].alive(), "still alive after 10 s");
    assert_eq!(b.soldiers[0].stance, Stance::Prone);
    let p = b.soldiers[0].pos;
    run(&mut b, 60_000);
    assert!(!b.soldiers[0].alive(), "bled out alone within a minute");
    assert_eq!(b.soldiers[0].pos, p, "he never moved");
}

#[test]
fn a_squadmate_treats_him_and_the_bleeding_stops() {
    let men = vec![man(0, 0, 50_000, 50_000), man(0, 0, 58_000, 50_000), man(0, 0, 60_000, 52_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), men, 2);
    b.soldiers[0].health = 100 - 2 * DAMAGE;
    let mut aided = false;
    for _ in 0..(30_000 / 50) {
        b.step(50);
        aided |= b.soldiers.iter().any(|s| matches!(s.act, Some((Act::Aid, _))));
    }
    assert!(aided, "someone gave first aid");
    assert!(b.soldiers[0].stable(), "treated");
    run(&mut b, 120_000);
    assert!(b.soldiers[0].alive() && b.soldiers[0].down(), "alive two minutes later, still down (needs evacuation)");
}

#[test]
fn under_fire_he_is_dragged_into_cover_first() {
    // A long wall south of the wounded man; the enemy is to the north.
    let props = [Prop { kind: PropKind::Wall { length_mm: 8_000, thickness_mm: 2_500 }, x_mm: 50_000, y_mm: 44_000, quarter_turns: 0 }];
    let men = vec![man(0, 0, 50_000, 50_000), man(0, 0, 53_000, 47_000), man(1, 1, 50_000, 120_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 200_000, &props), men, 3);
    b.soldiers[2].passive = true;
    b.soldiers[0].health = 100 - 2 * DAMAGE;
    b.soldiers[0].facing = (0, 1_000);
    b.soldiers[1].facing = (0, 1_000);
    run(&mut b, 1_000); // they see him
    let start = b.soldiers[0].pos;
    let mut dragged = false;
    for _ in 0..(40_000 / 50) {
        b.step(50);
        dragged |= b.soldiers[1].dragging().is_some();
    }
    assert!(dragged, "the helper dragged him");
    let p = b.soldiers[0].pos;
    assert!(dist(p, start) > 2_000, "moved from where he fell: {p:?}");
    assert!(p.1 < 43_500 && (p.0 - 50_000).abs() < 4_500, "behind the wall, away from the enemy: {p:?}");
    assert!(b.soldiers[0].stable(), "and treated there");
}

#[test]
fn nobody_shoots_at_a_man_who_is_down() {
    let men = vec![man(0, 0, 50_000, 50_000), man(1, 1, 50_000, 70_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), men, 4);
    b.soldiers[1].facing = (0, -1_000);
    b.soldiers[0].health = 100 - 2 * DAMAGE;
    run(&mut b, 5_000);
    let at_him = b.log.iter().filter(|e| matches!(e, Event::Shot { target: 0, .. })).count();
    assert_eq!(at_him, 0, "shots at a downed man");
    assert!(matches!(b.soldiers[0].task, Task::Idle));
}
