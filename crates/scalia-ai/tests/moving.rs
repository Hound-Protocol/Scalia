//! The squad Move order: get there (round walls, through doors), then make the place safe: men take the cover
//! within 10 m, or in the open form a 360-degree perimeter, each covering his own sector.
use scalia_ai::battle::{Battle, Soldier, Stance, Task};
use scalia_ai::leader::{Mode, Order};
use scalia_ai::save::{Command, MapSpec};
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{Prop, PropKind};

fn squad(n: usize, at: (i64, i64)) -> Vec<Soldier> {
    (0..n as i64).map(|i| Soldier::new(0, 0, (at.0 + (i % 3) * 1_200, at.1 + (i / 3) * 1_200), Stance::Stand)).collect()
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
fn in_the_open_they_form_a_perimeter_facing_out() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(6, (20_000, 20_000)), 1);
    let to = (60_000, 60_000);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to, mode: Mode::Plain } });
    run(&mut b, 60_000);
    let ms: Vec<&Soldier> = b.soldiers.iter().collect();
    let n = ms.len() as i64;
    let c = (ms.iter().map(|s| s.pos.0).sum::<i64>() / n, ms.iter().map(|s| s.pos.1).sum::<i64>() / n);
    assert!(dist(c, to) < 2_000, "the perimeter is round the spot: centre {c:?}");
    for s in &ms {
        let out = (s.pos.0 - c.0, s.pos.1 - c.1);
        assert!(dist(s.pos, to) < 8_000 && dist(s.pos, c) > 1_500, "on the ring: {:?}", s.pos);
        assert!(out.0 * s.facing.0 + out.1 * s.facing.1 > 0, "faces out: pos {:?} facing {:?}", s.pos, s.facing);
        assert_eq!(s.stance, Stance::Crouch, "low on the perimeter");
        assert!(matches!(s.task, Task::Hold { .. }));
    }
    // All round: someone faces each quarter.
    for q in 0..4 {
        assert!(ms.iter().any(|s| { let (x, y) = s.facing; match q { 0 => x > 0 && y >= 0, 1 => x <= 0 && y > 0, 2 => x < 0 && y <= 0, _ => x >= 0 && y < 0 } }), "quarter {q} covered");
    }
}

#[test]
fn near_walls_they_take_the_cover() {
    // Two low walls a few metres from the spot.
    let props = [
        Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 600 }, x_mm: 60_000, y_mm: 66_000, quarter_turns: 0 },
        Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 600 }, x_mm: 66_000, y_mm: 60_000, quarter_turns: 1 },
    ];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &props), squad(4, (20_000, 20_000)), 2);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (60_000, 60_000), mode: Mode::Plain } });
    run(&mut b, 60_000);
    let near_wall = |p: (i64, i64)| (p.1 - 66_000).abs() < 1_500 && (p.0 - 60_000).abs() < 3_500 || (p.0 - 66_000).abs() < 1_500 && (p.1 - 60_000).abs() < 3_500;
    let behind = b.soldiers.iter().filter(|s| near_wall(s.pos)).count();
    assert!(behind >= 3, "most men on the walls: {:?}", b.soldiers.iter().map(|s| s.pos).collect::<Vec<_>>());
}

#[test]
fn they_walk_into_a_building_through_its_door() {
    let k = Killhouse::new(80_000, 62_000);
    let mut b = Battle::on_map(MapSpec::killhouse((80_000, 62_000)), squad(4, (86_000, 35_000)), 3);
    // Room C, the north-west room: through the west door or the inner doors.
    let to = (80_000 + 4_000, 62_000 + 11_000);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to, mode: Mode::Plain } });
    run(&mut b, 90_000);
    let room = k.room_at(to.0, to.1);
    for s in &b.soldiers {
        assert_eq!(k.room_at(s.pos.0, s.pos.1), room, "inside room C: {:?}", s.pos);
    }
}
