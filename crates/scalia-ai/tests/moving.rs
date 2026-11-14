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

/// Runs a squad of 6 across open ground with `mode`; returns (ms to arrive, soldiers sampled half way).
fn crossing(mode: Mode) -> (i64, Vec<Soldier>, Battle) {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(6, (20_000, 20_000)), 4);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (20_000, 80_000), mode } });
    let mut half = None;
    while b.time_ms < 200_000 && !b.squad_done(0) {
        b.step(50);
        let c = b.soldiers.iter().map(|s| s.pos.1).sum::<i64>() / 6;
        if half.is_none() && c > 45_000 {
            half = Some(b.soldiers.clone());
        }
    }
    (b.time_ms, half.expect("got half way"), b)
}

#[test]
fn with_caution_they_go_slower_and_watch_all_round() {
    let (plain, _, _) = crossing(Mode::Plain);
    let (careful, mid, b) = crossing(Mode::Caution);
    assert!(b.squad_done(0), "they get there");
    assert!(careful > plain * 13 / 10, "slower: {careful} ms vs {plain} ms");
    // Half way, walking north: weapons up, and someone watches each side and the rear.
    assert!(mid.iter().all(|s| s.aim), "weapons up");
    assert!(mid.iter().any(|s| s.facing.0 > 600), "someone watches the east");
    assert!(mid.iter().any(|s| s.facing.0 < -600), "someone watches the west");
    assert!(mid.iter().any(|s| s.facing.1 < -500), "someone watches the rear");
}

#[test]
fn with_hostiles_confirmed_they_bound_in_two_teams() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(6, (20_000, 20_000)), 5);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (20_000, 80_000), mode: Mode::Confirmed } });
    let (mut split, mut samples) = (0, 0);
    while b.time_ms < 240_000 && !b.squad_done(0) {
        b.step(50);
        if b.time_ms % 1_000 == 0 && b.time_ms > 2_000 {
            let moving = b.soldiers.iter().filter(|s| s.moving).count();
            samples += 1;
            // One team moves while the other is down covering: neither all moving nor all still.
            if (2..=4).contains(&moving) {
                split += 1;
                let still: Vec<&Soldier> = b.soldiers.iter().filter(|s| !s.moving).collect();
                assert!(still.iter().all(|s| s.stance == Stance::Crouch && s.aim), "the covering team is down, weapons up: t={} {:?}", b.time_ms, b.soldiers.iter().map(|s| (s.moving, s.stance, s.aim, s.task)).collect::<Vec<_>>());
            }
        }
    }
    assert!(b.squad_done(0), "they get there and secure it");
    assert!(split * 2 > samples, "bounding most of the way: {split} of {samples} samples");
}

fn sandbags(b: &Battle) -> Vec<scalia_core::nav::Prop> {
    b.map_spec().unwrap().props().into_iter().filter(|p| matches!(p.kind, PropKind::Wall { thickness_mm: 700, length_mm: 2_000 })).collect()
}

#[test]
fn garrisoning_open_ground_they_build_sandbags_from_their_kit() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(6, (20_000, 20_000)), 6);
    let to = (50_000, 50_000);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to, mode: Mode::Garrison } });
    run(&mut b, 80_000);
    let bags = sandbags(&b);
    assert_eq!(bags.len(), 4, "the squad's kit is four loads");
    // Each bag is in front of a man, on the outer side; he is low behind it.
    for p in &bags {
        let man = b.soldiers.iter().min_by_key(|s| dist(s.pos, (p.x_mm, p.y_mm))).unwrap();
        assert!(dist(man.pos, (p.x_mm, p.y_mm)) < 1_500, "a man behind each bag: {:?} {:?}", man.pos, (p.x_mm, p.y_mm));
        assert!(dist((p.x_mm, p.y_mm), to) > dist(man.pos, to), "the bag is on the outer side");
        assert_eq!(man.stance, Stance::Crouch);
    }
    // The kit is used up: garrisoning somewhere else builds nothing more.
    b.submit(b.time_ms, Command::Squad { squad: 0, order: Order::Move { to: (70_000, 50_000), mode: Mode::Garrison } });
    run(&mut b, 60_000);
    assert_eq!(sandbags(&b).len(), 4);
    // And the sandbags survive a save.
    let loaded = Battle::load(&b.save()).unwrap();
    assert_eq!(loaded.hash(), b.hash());
}

#[test]
fn garrisoning_a_building_they_man_its_windows_facing_out() {
    let k = Killhouse::new(80_000, 62_000);
    let mut b = Battle::on_map(MapSpec::killhouse((80_000, 62_000)), squad(4, (86_000, 35_000)), 7);
    let to = (80_000 + 4_000, 62_000 + 11_000); // room C
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to, mode: Mode::Garrison } });
    run(&mut b, 120_000);
    let centre = (80_000 + 10_000, 62_000 + 7_000);
    for s in &b.soldiers {
        let w = k.windows.iter().map(|w| dist(s.pos, (w.x_mm, w.y_mm))).min().unwrap();
        let door = k.doors.iter().map(|d| dist(s.pos, (d.x_mm, d.y_mm))).min().unwrap();
        assert!(w < 1_300 || door < 1_600, "at a window or door: {:?}", s.pos);
        assert!(k.room_at(s.pos.0, s.pos.1).is_some(), "inside: {:?}", s.pos);
        let out = (s.pos.0 - centre.0, s.pos.1 - centre.1);
        assert!(out.0 * s.facing.0 + out.1 * s.facing.1 > 0, "facing out: {:?} {:?}", s.pos, s.facing);
    }
    assert!(sandbags(&b).is_empty(), "no sandbags in doorways");
}

#[test]
fn a_soldier_joining_inside_a_wall_is_put_beside_it() {
    let props = [Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 2_000 }, x_mm: 50_000, y_mm: 50_000, quarter_turns: 0 }];
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &props), vec![], 1);
    b.submit(0, Command::Spawn { team: 0, squad: 0, pos: (50_000, 50_000), stance: Stance::Stand });
    b.step(50);
    let p = b.soldiers[0].pos;
    assert!(b.nav().unwrap().walkable_at(p.0, p.1, 250), "walkable: {p:?}");
    assert!(dist(p, (50_000, 50_000)) < 2_500, "close by: {p:?}");
}

#[test]
fn the_dead_stop_moving_and_drop_what_they_were_doing() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(1, (20_000, 20_000)), 1);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (60_000, 60_000), mode: Mode::Plain } });
    run(&mut b, 2_000);
    assert!(b.soldiers[0].moving);
    b.soldiers[0].health = 0;
    b.step(50);
    let s = &b.soldiers[0];
    assert!(!s.moving && s.vel == (0, 0) && s.task == Task::Idle && s.act.is_none(), "{:?} {:?} {:?}", s.moving, s.vel, s.task);
}

#[test]
fn a_builder_killed_at_work_lays_no_sandbag() {
    let mut b = Battle::on_map(MapSpec::new(0, 0, 100_000, 100_000, &[]), squad(4, (20_000, 20_000)), 6);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (50_000, 50_000), mode: Mode::Garrison } });
    // Until the first man starts laying bags, then he dies.
    while b.time_ms < 80_000 && !b.soldiers.iter().any(|s| s.act.is_some()) {
        b.step(50);
    }
    let k = b.soldiers.iter().position(|s| s.act.is_some()).expect("someone builds");
    b.soldiers[k].health = 0;
    run(&mut b, 20_000);
    assert_eq!(sandbags(&b).len(), 3, "the dead man's bags are never laid");
}
