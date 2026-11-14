//! A soldier the player controls (the commander on foot): no AI moves or fires him; he fires where he points,
//! under the same hit rules, ammunition and reloads as everyone else.
use scalia_ai::battle::{Battle, Event, PlayerShot, Soldier, Stance, MAG};
use scalia_ai::sight::SightGrid;
use scalia_core::nav::{Prop, PropKind};

fn field(props: &[Prop]) -> SightGrid {
    SightGrid::build(0, 0, 400, 400, props)
}

fn duel(props: &[Prop]) -> (Battle, usize) {
    let mut foe = Soldier::new(1, 1, (50_000, 40_000), Stance::Stand);
    foe.passive = true;
    foe.health = i32::MAX;
    let mut b = Battle::new(field(props), vec![foe], 3);
    let me = b.join_player(0, 2, (50_000, 30_000));
    (b, me)
}

/// Holds the trigger towards `dir` for `ms`, stepping the battle; returns the shots logged by the player.
fn hold(b: &mut Battle, me: usize, dir: (i64, i64), aimed: bool, ms: i64) -> Vec<(bool, Option<usize>)> {
    let mut t = 0;
    while t < ms {
        b.player_fire(me, dir, aimed);
        b.step(50);
        t += 50;
    }
    b.log.iter().filter_map(|e| match e { Event::Shot { shooter, target, hit, .. } if *shooter == me => Some((*hit, (*target != me).then_some(*target))), _ => None }).collect()
}

#[test]
fn he_hits_what_he_points_at_and_misses_what_he_does_not() {
    let (mut b, me) = duel(&[]);
    let at_him = hold(&mut b, me, (0, 1_000), true, 1_500);
    assert!(at_him.iter().any(|s| s.0), "aimed at a man 10 m off, some rounds hit");
    assert!(at_him.iter().all(|s| s.1 == Some(0)), "every round goes at the man in the line");
    let (mut b, me) = duel(&[]);
    let away = hold(&mut b, me, (1_000, 0), true, 1_500);
    assert!(!away.is_empty() && away.iter().all(|s| !s.0 && s.1.is_none()), "pointed away: rounds fly, none hit");
}

#[test]
fn the_hip_is_less_accurate_than_the_sights() {
    let hits = |aimed| {
        let (mut b, me) = duel(&[]);
        b.soldiers[me].mags = 1_000;
        hold(&mut b, me, (0, 1_000), aimed, 40_000).iter().filter(|s| s.0).count()
    };
    assert!(hits(true) > hits(false), "aimed {} vs hip {}", hits(true), hits(false));
}

#[test]
fn a_wall_stops_his_rounds() {
    let wall = Prop { kind: PropKind::Panel { length_mm: 20_000, window: false }, x_mm: 50_000, y_mm: 35_000, quarter_turns: 0 };
    let (mut b, me) = duel(&[wall]);
    assert!(hold(&mut b, me, (0, 1_000), true, 1_500).iter().all(|s| !s.0 && s.1.is_none()));
}

#[test]
fn a_magazine_runs_dry_and_reloads_from_the_spares() {
    let (mut b, me) = duel(&[]);
    b.soldiers[me].mags = 5;
    let mut empty = false;
    for _ in 0..200 {
        if b.player_fire(me, (1_000, 0), false) == PlayerShot::Empty {
            empty = true;
        }
        b.step(50);
    }
    assert!(empty || b.soldiers[me].reloading_ms > 0 || b.soldiers[me].ammo < MAG, "the magazine empties");
    assert!(b.soldiers[me].mags < 5, "a spare went in");
    // No AI moves him or fires for him.
    let (mut b, me) = duel(&[]);
    for _ in 0..100 {
        b.step(50);
    }
    assert_eq!(b.soldiers[me].pos, (50_000, 30_000));
    assert!(b.log.iter().all(|e| !matches!(e, Event::Shot { shooter, .. } if *shooter == me)));
}
