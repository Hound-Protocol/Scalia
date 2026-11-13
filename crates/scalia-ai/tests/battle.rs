use scalia_ai::battle::{Battle, Event, Soldier, Stance};
use scalia_ai::sight::SightGrid;
use scalia_core::nav::{Prop, PropKind};

fn field(props: &[Prop]) -> SightGrid {
    SightGrid::build(0, 0, 400, 400, props) // 100 x 100 m
}
fn man(team: u8, x: i64, y: i64, stance: Stance) -> Soldier {
    Soldier::new(team, 0, (x, y), stance)
}
/// Runs until someone dies or `ms` pass; returns the battle.
fn run(mut b: Battle, ms: i64) -> Battle {
    let mut t = 0;
    while t < ms && b.soldiers.iter().all(|s| s.alive()) {
        b.step(50);
        t += 50;
    }
    b
}

#[test]
fn an_open_ground_duel_ends_and_is_deterministic() {
    let mk = || Battle::new(field(&[]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7);
    let (a, b) = (run(mk(), 30_000), run(mk(), 30_000));
    assert!(a.soldiers.iter().any(|s| !s.alive()), "someone falls within 30 s at 30 m");
    assert_eq!(a.log, b.log, "same seed, same fight");
}

#[test]
fn nobody_shoots_through_a_wall() {
    let wall = Prop { kind: PropKind::Panel { length_mm: 20_000, window: false }, x_mm: 50_000, y_mm: 45_000, quarter_turns: 0 };
    let b = run(Battle::new(field(&[wall]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7), 10_000);
    assert!(b.log.iter().all(|e| !matches!(e, Event::Shot { .. })), "no line of sight, no shots");
}

/// Hits landed on an unkillable target over 60 s of fire.
fn hits_on(target: Soldier, props: &[Prop]) -> usize {
    let mut b = Battle::new(field(props), vec![man(0, 50_000, 20_000, Stance::Stand), target], 11);
    b.soldiers[1].health = i32::MAX;
    b.soldiers[1].passive = true;
    b.soldiers[0].mags = 99;
    for _ in 0..1200 {
        b.step(50);
    }
    b.log.iter().filter(|e| matches!(e, Event::Shot { hit: true, .. })).count()
}

#[test]
fn cover_and_stance_make_a_man_harder_to_hit() {
    let bags = Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 700 }, x_mm: 50_000, y_mm: 59_000, quarter_turns: 0 };
    let open = hits_on(man(1, 50_000, 60_000, Stance::Stand), &[]);
    let behind_bags = hits_on(man(1, 50_000, 60_000, Stance::Stand), &[bags]);
    let crouched_behind = hits_on(man(1, 50_000, 60_000, Stance::Crouch), &[bags]);
    assert!(open > behind_bags, "legs behind the bags: {open} vs {behind_bags}");
    assert!(behind_bags > crouched_behind, "crouched behind the bags: {behind_bags} vs {crouched_behind}");
}

#[test]
fn being_shot_at_suppresses_and_suppression_spoils_aim() {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 20_000, Stance::Stand), man(1, 50_000, 80_000, Stance::Stand)], 3);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    for _ in 0..40 {
        b.step(50);
    }
    assert!(b.soldiers[1].suppression > 0, "rounds coming in suppress");
    let calm = b.hit_permille(0, 1);
    b.soldiers[0].suppression = 1000;
    assert!(b.hit_permille(0, 1) < calm, "a suppressed shooter aims worse");
}

#[test]
fn a_rifleman_reloads_after_a_magazine() {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 20_000, Stance::Stand), man(1, 50_000, 80_000, Stance::Stand)], 5);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    for _ in 0..400 {
        b.step(50);
    }
    assert!(b.log.iter().any(|e| matches!(e, Event::Reload { soldier: 0 })));
}
