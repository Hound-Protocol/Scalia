use scalia_ai::battle::{Battle, Gait, Soldier, Stance, Task};
use scalia_ai::sight::SightGrid;
use scalia_ai::tactics::CoverMap;
use scalia_core::nav::{NavGrid, Prop, PropKind};

fn battle(props: &[Prop], soldiers: Vec<Soldier>, seed: u64) -> Battle {
    let mut nav = NavGrid::new(0, 0, 200, 200);
    nav.rebuild(props);
    let sight = SightGrid::build(0, 0, 400, 400, props);
    let cover = CoverMap::build(props, &nav);
    Battle::with_map(nav, sight, cover, soldiers, seed)
}
fn man(team: u8, squad: u16, x: i64, y: i64) -> Soldier {
    Soldier::new(team, squad, (x, y), Stance::Stand)
}
fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f64).sqrt() as i64
}

#[test]
fn he_walks_around_a_wall_to_his_destination() {
    let wall = Prop { kind: PropKind::Wall { length_mm: 20_000, thickness_mm: 600 }, x_mm: 50_000, y_mm: 50_000, quarter_turns: 0 };
    let mut b = battle(&[wall], vec![man(0, 0, 50_000, 40_000)], 1);
    b.order(0, Task::Move { to: (50_000, 60_000), gait: Gait::Walk });
    let mut ms = 0;
    while dist(b.soldiers[0].pos, (50_000, 60_000)) > 400 && ms < 60_000 {
        b.step(50);
        ms += 50;
        assert!(!b.nav().unwrap().cell_of(b.soldiers[0].pos.0, b.soldiers[0].pos.1).map_or(false, |(i, j)| b.nav().unwrap().blocked(i, j)), "never inside the wall");
    }
    assert!(ms < 60_000, "arrived");
    assert!(ms > 17_000, "around the 20 m wall at walking pace (about 29 m), not straight through it (13 s): {ms} ms");
}

#[test]
fn two_men_sent_to_one_spot_keep_their_distance() {
    let mut b = battle(&[], vec![man(0, 0, 40_000, 40_000), man(0, 0, 42_000, 40_000)], 1);
    for i in 0..2 {
        b.order(i, Task::Move { to: (50_000, 50_000), gait: Gait::Run });
    }
    for _ in 0..400 {
        b.step(50);
    }
    assert!(dist(b.soldiers[0].pos, b.soldiers[1].pos) >= 500);
}

#[test]
fn pinned_in_the_open_he_runs_to_cover_that_hides_him() {
    let bags = Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 700 }, x_mm: 50_000, y_mm: 52_000, quarter_turns: 0 };
    // He stands 5 m south of the bags; the enemy is 30 m north and keeps firing.
    let mut b = battle(&[bags], vec![man(0, 0, 50_000, 47_000), man(1, 1, 50_000, 80_000)], 4);
    b.soldiers[0].passive = true;
    b.soldiers[0].health = i32::MAX;
    b.soldiers[0].facing = (0, 1_000);
    b.soldiers[1].facing = (0, -1_000);
    b.soldiers[1].mags = 99;
    for _ in 0..200 {
        b.step(50);
    }
    let me = &b.soldiers[0];
    assert!(me.cover.is_some(), "took cover");
    assert!(me.pos.1 < 52_000, "on the safe side: {:?}", me.pos);
    assert!(b.exposure(1, 0) < 3, "the bags hide at least part of him");
}

#[test]
fn holding_men_face_where_they_are_told() {
    let mut b = battle(&[], vec![man(0, 0, 50_000, 50_000)], 1);
    b.order(0, Task::Hold { at: (50_000, 50_000), face: (60_000, 50_000) });
    for _ in 0..40 {
        b.step(50);
    }
    assert!(b.soldiers[0].facing.0 > 900, "facing east: {:?}", b.soldiers[0].facing);
}

#[test]
fn a_moving_fight_is_deterministic() {
    let run = || {
        let mut b = battle(&[], vec![man(0, 0, 40_000, 40_000), man(0, 0, 41_000, 40_000), man(1, 1, 60_000, 70_000)], 9);
        for i in 0..2 {
            b.order(i, Task::Move { to: (55_000, 60_000), gait: Gait::Run });
        }
        for _ in 0..300 {
            b.step(50);
        }
        (b.soldiers.iter().map(|s| (s.pos, s.health)).collect::<Vec<_>>(), b.log.len())
    };
    assert_eq!(run(), run());
}
