//! The simulation contract: commands are the only input; replaying them reproduces the battle; a save restored
//! and continued is indistinguishable from never stopping.
use scalia_ai::battle::{Battle, Gait, Soldier, Stance, Task};
use scalia_ai::save::{Command, MapSpec};
use scalia_ai::leader::Order;

const HOUSE: (i64, i64) = (80_000, 62_000);
const DEFENDERS: [(i64, i64); 8] = [(12_500, 3_500), (16_000, 2_000), (5_000, 2_000), (2_500, 5_000), (3_000, 10_600), (6_500, 12_000), (16_500, 10_200), (13_000, 12_500)];

fn killhouse(seed: u64) -> Battle {
    let map = MapSpec::killhouse(HOUSE);
    let mut soldiers: Vec<Soldier> = (0..12).map(|i| Soldier::new(0, 0, (HOUSE.0 + 6_000 + (i % 4) * 1_500, HOUSE.1 - 27_000 + (i / 4) * 1_500), Stance::Stand)).collect();
    for (x, y) in DEFENDERS {
        soldiers.push(Soldier::new(1, 1, (HOUSE.0 + x, HOUSE.1 + y), Stance::Stand));
    }
    Battle::on_map(map, soldiers, seed)
}

/// The orders of a short fight: capture and hold at the start, one defender sent to another room later.
fn script(b: &mut Battle) {
    b.submit(0, Command::Squad { squad: 0, order: Order::Capture });
    b.submit(0, Command::Squad { squad: 1, order: Order::Hold });
    b.submit(20_000, Command::Task { soldier: 19, task: Task::Move { to: (HOUSE.0 + 15_000, HOUSE.1 + 11_000), gait: Gait::Run } });
}

fn run(b: &mut Battle, until_ms: i64) -> Vec<u64> {
    let mut hashes = Vec::new();
    while b.time_ms < until_ms {
        b.step(50);
        if b.time_ms % 10_000 == 0 {
            hashes.push(b.hash());
        }
    }
    hashes
}

#[test]
fn replaying_the_command_history_reproduces_the_battle() {
    let mut a = killhouse(7);
    script(&mut a);
    let first = run(&mut a, 120_000);
    let history = a.history().to_vec();
    assert_eq!(history.len(), 3);
    let mut b = killhouse(7);
    for (at, c) in history {
        b.submit(at, c);
    }
    assert_eq!(run(&mut b, 120_000), first);
}

#[test]
fn a_different_order_gives_a_different_battle() {
    let mut a = killhouse(7);
    script(&mut a);
    let mut b = killhouse(7);
    script(&mut b);
    b.submit(30_000, Command::Task { soldier: 0, task: Task::Hold { at: (HOUSE.0 + 7_000, HOUSE.1 - 20_000), face: HOUSE } });
    assert_ne!(run(&mut a, 60_000).last(), run(&mut b, 60_000).last());
}

#[test]
fn a_late_command_takes_effect_at_the_next_step_and_is_recorded_there() {
    let mut a = killhouse(3);
    script(&mut a);
    run(&mut a, 25_000);
    a.submit(1_000, Command::Task { soldier: 2, task: Task::Hold { at: (HOUSE.0, HOUSE.1 - 20_000), face: HOUSE } });
    a.step(50);
    let last = a.history().last().unwrap();
    assert_eq!(last.0, 25_050, "recorded at the step it took effect, not the time it claimed");
}

#[test]
fn save_restore_and_continue_equals_never_stopping() {
    let mut a = killhouse(11);
    script(&mut a);
    run(&mut a, 40_000);
    let saved = a.save();
    run(&mut a, 100_000);
    let mut b = Battle::load(&saved).expect("loads");
    assert_eq!(b.time_ms, 40_000);
    run(&mut b, 100_000);
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.soldiers, b.soldiers);
}

#[test]
fn a_damaged_save_is_refused() {
    let mut a = killhouse(1);
    script(&mut a);
    run(&mut a, 5_000);
    let mut saved = a.save();
    saved.truncate(saved.len() / 2);
    assert!(Battle::load(&saved).is_err());
}

#[test]
fn soldiers_joining_later_are_part_of_the_replay() {
    use scalia_ai::leader::Mode;
    let mut a = killhouse(5);
    script(&mut a);
    for k in 0..4 {
        a.submit(10_000, Command::Spawn { team: 0, squad: 7, pos: (HOUSE.0 - 10_000 + k * 1_000, HOUSE.1 - 10_000), stance: Stance::Stand });
    }
    a.submit(10_000, Command::Squad { squad: 7, order: Order::Move { to: (HOUSE.0 - 15_000, HOUSE.1), mode: Mode::Plain } });
    let first = run(&mut a, 60_000);
    assert_eq!(a.soldiers.len(), 24);
    let mut b = killhouse(5);
    for (at, c) in a.history().to_vec() {
        b.submit(at, c);
    }
    assert_eq!(run(&mut b, 60_000), first);
    let saved = a.save();
    assert_eq!(Battle::load(&saved).unwrap().hash(), a.hash());
}
