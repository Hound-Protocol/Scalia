//! Wounded soldiers: hit twice they go down (no moving, no firing) and bleed; enemies stop shooting at them;
//! a squadmate comes, drags them to cover if the enemy is near, and treats them, which stops the bleeding.
use scalia_ai::battle::{Act, Battle, Event, Gait, InterruptKind, Soldier, Stance, Task, DAMAGE};
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

#[test]
fn nobody_runs_out_to_a_casualty_while_an_enemy_who_is_not_pinned_covers_him() {
    // The enemy 40 m off has a clear line to the wounded man and is not suppressed (he cannot be hit: no
    // fire superiority). The squadmate stays out of it.
    let men = vec![man(0, 0, 50_000, 50_000), man(0, 0, 56_000, 46_000), man(1, 1, 50_000, 90_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 120_000, &[]), men, 4);
    b.soldiers[2].health = i32::MAX;
    b.soldiers[1].health = i32::MAX;
    b.soldiers[0].health = 100 - 2 * DAMAGE;
    let mut went = false;
    for _ in 0..(8_000 / 50) {
        b.step(50);
        // Keep the enemy unpinned (he shrugs off the return fire).
        b.soldiers[2].suppression = 0;
        went |= dist(b.soldiers[1].pos, b.soldiers[0].pos) < 2_000;
    }
    assert!(!went, "he did not run out into the open to him");
    // The enemy is pinned down now (or gone): then he goes.
    b.soldiers[2].health = 0;
    let mut reached = false;
    for _ in 0..(15_000 / 50) {
        b.step(50);
        reached |= dist(b.soldiers[1].pos, b.soldiers[0].pos) < 2_000;
    }
    assert!(reached, "once it is safe, he goes to him");
}

#[test]
fn a_helper_resumes_his_prior_task_after_aid_and_after_threat_abort() {
    let dest = (75_000, 50_000);
    let men = vec![man(0, 0, 52_000, 50_000), man(0, 0, 55_000, 50_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), men, 11);
    b.order(1, Task::Move { to: dest, gait: Gait::Walk });
    b.soldiers[0].health = 100 - 2 * DAMAGE;

    let mut interrupted = false;
    for _ in 0..(25_000 / 50) {
        b.step(50);
        if b.soldiers[1].interrupt == Some(InterruptKind::Aid) {
            interrupted = true;
            assert!(b.soldiers[1].saved_intent.is_some());
        }
        if interrupted && b.soldiers[0].stable() && b.soldiers[1].aiding().is_none() {
            break;
        }
    }
    assert!(interrupted, "helper entered Aid interrupt");
    assert!(b.soldiers[0].stable(), "wounded soldier was treated");
    assert_eq!(b.soldiers[1].interrupt, None);
    assert_eq!(b.soldiers[1].saved_intent, None);
    assert_eq!(b.soldiers[1].task, Task::Move { to: dest, gait: Gait::Walk });

    // Threat abort while en route also restores the helper's preserved Hold assignment.
    let hold = Task::Hold { at: (46_000, 48_000), face: (50_000, 80_000) };
    let mut men2 = vec![
        man(0, 0, 50_000, 50_000),
        man(0, 0, 62_000, 50_000),
        man(1, 1, 50_000, 95_000),
    ];
    men2[2].passive = true;
    men2[2].health = 0; // initially dead so care dispatches helper 1
    let mut b2 = Battle::on_map(MapSpec::new(0, 0, 100_000, 120_000, &[]), men2, 12);
    b2.order(1, hold);
    b2.soldiers[0].health = 100 - 2 * DAMAGE;

    let mut started = false;
    for _ in 0..(4_000 / 50) {
        b2.step(50);
        if b2.soldiers[1].aiding() == Some(0) {
            started = true;
            break;
        }
    }
    assert!(started, "helper started heading to wounded teammate");
    // Now an unsuppressed enemy enters line of sight on the casualty before arrival.
    b2.soldiers[2].health = 100;
    b2.soldiers[2].suppression = 0;
    b2.soldiers[2].facing = (0, -1_000);
    b2.contacts[0].push(scalia_ai::battle::Contact {
        enemy: 2,
        pos: b2.soldiers[2].pos,
        seen_ms: b2.time_ms,
        confidence: 1_000,
        source: scalia_ai::battle::IntelSource::Visual,
    });
    run(&mut b2, 1_500);
    assert_eq!(b2.soldiers[1].aiding(), None, "aid aborted under casualty threat");
    assert_eq!(b2.soldiers[1].interrupt, None);
    assert_eq!(b2.soldiers[1].task, hold, "original Hold assignment resumed after threat abort");
}

#[test]
fn a_new_squad_order_while_aiding_prevents_resuming_the_stale_pre_order_task() {
    use scalia_ai::leader::{Mode, Order};
    use scalia_ai::save::Command;

    let men = vec![man(0, 0, 50_000, 40_000), man(0, 0, 54_000, 40_000)];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), men, 13);
    let old_dest = (50_000, 85_000);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: old_dest, mode: Mode::Plain } });
    run(&mut b, 500);
    assert!(matches!(b.soldiers[1].task, Task::Move { .. }));

    b.soldiers[0].health = 100 - 2 * DAMAGE;
    let mut aiding = false;
    for _ in 0..(5_000 / 50) {
        b.step(50);
        if b.soldiers[1].aiding() == Some(0) {
            aiding = true;
            break;
        }
    }
    assert!(aiding, "helper began casualty aid during Move");
    assert!(b.soldiers[1].saved_intent.is_some());

    // Commander replaces Move with Hold while helper is still busy aiding.
    b.submit(b.time_ms, Command::Squad { squad: 0, order: Order::Hold });
    b.step(50);
    assert_eq!(b.soldiers[1].interrupt, Some(InterruptKind::Aid));
    assert_eq!(b.soldiers[1].saved_intent, None, "new squad order invalidated stale saved intent");

    for _ in 0..(20_000 / 50) {
        b.step(50);
        if b.soldiers[0].stable() && b.soldiers[1].aiding().is_none() {
            break;
        }
    }
    assert!(b.soldiers[0].stable());
    assert_eq!(b.soldiers[1].interrupt, None);
    assert!(!matches!(b.soldiers[1].task, Task::Move { to, .. } if to == old_dest), "must not resume stale pre-order Move");
}
