use scalia_ai::battle::{Battle, Soldier, Stance};
use scalia_ai::sight::SightGrid;

fn field() -> SightGrid {
    SightGrid::build(0, 0, 400, 400, &[])
}
fn man(team: u8, squad: u16, x: i64, y: i64, facing: (i64, i64)) -> Soldier {
    let mut s = Soldier::new(team, squad, (x, y), Stance::Stand);
    s.facing = facing;
    s
}
const NORTH: (i64, i64) = (0, 1_000);
const SOUTH: (i64, i64) = (0, -1_000);

#[test]
fn he_notices_what_is_in_front_not_behind() {
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(1, 1, 50_000, 20_000, SOUTH)], 1);
    b.soldiers[1].passive = true;
    b.step(250);
    assert!(!b.sees(0, 1), "30 m behind him");
    assert!(b.contact(0, 1).is_none());
    b.soldiers[0].facing = SOUTH;
    b.step(250);
    assert!(b.sees(0, 1));
    assert!(b.contact(0, 1).is_some());
}

#[test]
fn a_man_right_beside_him_is_noticed_whichever_way_he_faces() {
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(1, 1, 50_000, 48_000, NORTH)], 1);
    b.soldiers[1].passive = true;
    b.step(250);
    assert!(b.sees(0, 1), "2 m behind: heard and felt");
}

#[test]
fn the_squad_shares_what_one_man_sees() {
    // Squadmate 1 faces away; only man 0 sees the enemy, both know of him.
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(0, 0, 48_000, 50_000, SOUTH), man(1, 1, 50_000, 80_000, SOUTH)], 1);
    b.soldiers[2].passive = true;
    b.step(250);
    assert!(!b.sees(1, 2));
    assert!(b.contact(1, 2).is_some(), "shared through the squad");
}

#[test]
fn shot_from_behind_he_learns_where_from_turns_and_returns_fire() {
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(1, 1, 50_000, 25_000, NORTH)], 2);
    // Enemy (1) behind man 0, facing him. (Man 0 is made tough: this is about how he reacts, not whether he
    // survives bursts in the back.)
    b.soldiers[0].health = 1_000;
    b.soldiers[1].health = 1_000;
    for _ in 0..200 {
        b.step(50);
    }
    assert!(b.contact(0, 1).is_some(), "the shots gave the enemy away");
    assert!(b.soldiers[0].facing.1 < 0, "turned to face the fire: {:?}", b.soldiers[0].facing);
    assert!(b.log.iter().any(|e| matches!(e, scalia_ai::battle::Event::Shot { shooter: 0, .. })), "and fired back");
}

#[test]
fn a_lost_contact_fades_but_keeps_its_last_known_position() {
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(1, 1, 50_000, 70_000, SOUTH)], 1);
    b.soldiers[0].passive = true;
    b.soldiers[1].passive = true;
    b.step(250);
    let seen = b.contact(0, 1).expect("seen").confidence;
    b.soldiers[1].pos = (90_000, 90_000); // gone out of sight and away
    b.soldiers[0].facing = SOUTH;
    for _ in 0..20 {
        b.step(250);
    }
    let c = b.contact(0, 1).expect("still remembered");
    assert!(c.confidence < seen, "fading");
    assert_eq!(c.pos, (50_000, 70_000), "last known position, not where he really is");
}

#[test]
fn gunfire_is_heard_by_friends_nearby() {
    // Squad 0 fights squad 1; squad 2 (team 0) stands 40 m away facing elsewhere and hears it.
    let mut b = Battle::new(field(), vec![man(0, 0, 50_000, 50_000, NORTH), man(1, 1, 50_000, 70_000, SOUTH), man(0, 2, 10_000, 50_000, SOUTH)], 3);
    b.soldiers[1].passive = true;
    // (A moment to spot him and get a sight picture before the first round.)
    for _ in 0..60 {
        b.step(50);
    }
    assert!(b.contact(2, 1).is_some() || b.heard(2).is_some(), "the shooting is heard");
}
