use scalia_ai::battle::{Battle, Event, Soldier, Stance};
use scalia_ai::leader::{choose, Method, Order, Situation};
use scalia_ai::sight::SightGrid;
use scalia_ai::tactics::CoverMap;
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::NavGrid;

#[test]
fn the_leader_picks_the_method_the_situation_calls_for() {
    let base = Situation { intel: false, enemy_in_entry: false, have_drone: true, have_grenade: true, strength: 8, enemies_known: 0 };
    // Nothing known and a drone in the kit: look first.
    assert!(choose(&base).contains(&Method::Drone), "{:?}", choose(&base));
    // Intel says someone is waiting in the entry room: grenade before going in.
    let s = Situation { intel: true, enemy_in_entry: true, enemies_known: 2, ..base };
    let p = choose(&s);
    assert!(p.contains(&Method::Grenade) && !p.contains(&Method::Drone), "{p:?}");
    // Entry room empty and we outnumber them heavily: storm it.
    let s = Situation { intel: true, enemy_in_entry: false, enemies_known: 2, ..base };
    assert!(choose(&s).contains(&Method::Storm), "{:?}", choose(&s));
    // Blind, no drone, not strong: slow and careful.
    let s = Situation { have_drone: false, strength: 4, ..base };
    assert!(choose(&s).contains(&Method::SlowEntry), "{:?}", choose(&s));
    // Whatever the way in, the door is pied first.
    for p in [choose(&base), choose(&s)] {
        let (pie, entry) = (p.iter().position(|&m| m == Method::Pie), p.iter().position(|&m| matches!(m, Method::Storm | Method::SlowEntry)));
        assert!(pie.is_some() && pie < entry, "{p:?}");
    }
}

fn killhouse_battle(attackers: usize, defenders: &[(i64, i64)], seed: u64) -> Battle {
    let k = Killhouse::new(80_000, 62_000);
    let props = k.props();
    let mut nav = NavGrid::new(40_000, 20_000, 200, 160);
    nav.rebuild(&props);
    let sight = SightGrid::build(40_000, 20_000, 400, 320, &props);
    let cover = CoverMap::build(&props, &nav);
    let mut soldiers: Vec<Soldier> = (0..attackers).map(|i| Soldier::new(0, 0, (86_000 + (i as i64 % 4) * 1_500, 35_000 + (i as i64 / 4) * 1_500), Stance::Stand)).collect();
    for &(x, y) in defenders {
        soldiers.push(Soldier::new(1, 1, (80_000 + x, 62_000 + y), Stance::Stand));
    }
    let mut b = Battle::with_map(nav, sight, cover, soldiers, seed);
    b.set_building(k);
    b.command(0, Order::Capture);
    b.command(1, Order::Hold);
    b
}

#[test]
fn a_squad_takes_the_killhouse_from_two_defenders() {
    let mut b = killhouse_battle(8, &[(16_000, 2_000), (5_000, 12_000)], 21);
    let mut ms = 0;
    while ms < 240_000 && !b.squad_done(0) {
        b.step(50);
        ms += 50;
    }
    // Down men are out of the fight (they bleed out where they lie).
    let defenders_alive = b.soldiers.iter().filter(|s| s.team == 1 && s.alive() && !s.down()).count();
    assert_eq!(defenders_alive, 0, "building cleared in {ms} ms");
    assert!(b.squad_done(0), "holding the building");
    let inside = b.soldiers.iter().filter(|s| s.team == 0 && s.alive() && b.building().unwrap().room_at(s.pos.0, s.pos.1).is_some()).count();
    assert!(inside >= 2, "men inside holding it: {inside}");
    assert!(b.log.iter().any(|e| matches!(e, Event::Order { .. })), "the leader's decisions are logged");
}

#[test]
fn the_assault_is_deterministic() {
    let run = |seed| {
        let mut b = killhouse_battle(8, &[(16_000, 2_000), (5_000, 12_000)], seed);
        for _ in 0..1_200 {
            b.step(50);
        }
        b.soldiers.iter().map(|s| (s.pos, s.health)).collect::<Vec<_>>()
    };
    assert_eq!(run(5), run(5));
}

#[test]
fn the_approach_bounds_one_team_moving_while_the_other_watches() {
    let mut b = killhouse_battle(8, &[(16_000, 2_000)], 3);
    let mut both_moving = 0;
    let mut samples = 0;
    for _ in 0..300 {
        b.step(50);
        if b.squad_phase(0) == "approach" {
            samples += 1;
            let moving = |team: usize| b.soldiers.iter().enumerate().filter(|(i, s)| s.team == 0 && i % 2 == team && s.moving).count();
            if moving(0) > 0 && moving(1) > 0 {
                both_moving += 1;
            }
        }
    }
    assert!(samples > 20, "an approach happened");
    assert!(both_moving * 4 < samples, "mostly one team moves at a time: {both_moving}/{samples}");
}

#[test]
fn an_outnumbered_defender_falls_back_to_the_next_room() {
    let k = Killhouse::new(80_000, 62_000);
    let props = k.props();
    let mut nav = NavGrid::new(40_000, 20_000, 200, 160);
    nav.rebuild(&props);
    let sight = SightGrid::build(40_000, 20_000, 400, 320, &props);
    let cover = CoverMap::build(&props, &nav);
    let at = |x: i64, y: i64| (80_000 + x, 62_000 + y);
    // Two attackers already inside room A (they do not shoot), one defender there, his mate in room C.
    let mut soldiers = vec![Soldier::new(0, 0, at(6_500, 2_000), Stance::Stand), Soldier::new(0, 0, at(6_500, 5_000), Stance::Stand)];
    for s in &mut soldiers {
        s.passive = true;
    }
    soldiers.push(Soldier::new(1, 1, at(2_000, 3_500), Stance::Stand));
    soldiers.push(Soldier::new(1, 1, at(3_000, 11_000), Stance::Stand));
    soldiers[2].facing = (1_000, 0);
    let mut b = Battle::with_map(nav, sight, cover, soldiers, 3);
    b.set_building(k.clone());
    b.command(1, Order::Hold);
    for _ in 0..(8_000 / 50) {
        b.step(50);
    }
    let me = &b.soldiers[2];
    assert!(me.alive());
    let room_c = k.room_at(at(3_000, 11_000).0, at(3_000, 11_000).1);
    assert_eq!(k.room_at(me.pos.0, me.pos.1), room_c, "defender at {:?}, task {:?}", me.pos, me.task);
    assert!(b.log.iter().any(|e| matches!(e, Event::Order { squad: 1, step: "fall back" })));
    // His mate who was not threatened holds where he was.
    assert_eq!(k.room_at(b.soldiers[3].pos.0, b.soldiers[3].pos.1), room_c);
}


#[test]
fn the_stack_hugs_the_wall_beside_the_door() {
    use scalia_ai::battle::Event;
    let mut b = killhouse_battle(8, &[(16_000, 2_000), (5_000, 12_000)], 21);
    // Run until the leader moves on from the stack (it was ready, or timed out), and look at it then.
    let mut stacked_at = None;
    let mut ms = 0;
    while ms < 120_000 {
        let before = b.log.len();
        b.step(50);
        ms += 50;
        let fresh: Vec<&'static str> = b.log[before..].iter().filter_map(|e| if let Event::Order { squad: 0, step } = e { Some(*step) } else { None }).collect();
        if stacked_at.is_some() && !fresh.is_empty() {
            break;
        }
        if fresh.contains(&"stack") {
            stacked_at = Some(ms);
        }
    }
    assert!(stacked_at.is_some(), "the squad stacked up");
    let k = b.building().unwrap();
    let all: Vec<(i64, i64)> = b.soldiers.iter().filter(|s| s.team == 0 && s.alive() && !s.down()).map(|s| s.pos).collect();
    let door = k.doors.iter().filter(|d| d.rooms.0.is_none() || d.rooms.1.is_none()).min_by_key(|d| all.iter().map(|m| (m.0 - d.x_mm).pow(2) + (m.1 - d.y_mm).pow(2)).min().unwrap()).unwrap();
    // The four at the door.
    let mut men = all.clone();
    men.sort_by_key(|m| (m.0 - door.x_mm).pow(2) + (m.1 - door.y_mm).pow(2));
    men.truncate(4);
    for m in &men {
        let d = (((m.0 - door.x_mm).pow(2) + (m.1 - door.y_mm).pow(2)) as f64).sqrt() as i64;
        assert!(d < 5_500, "stack man at {m:?} is {d} mm from the door");
        let off = if door.along_x { (m.1 - door.y_mm).abs() } else { (m.0 - door.x_mm).abs() };
        assert!(off < 1_400, "stack man at {m:?} is {off} mm off the wall, not hugging it");
    }
    for a in 0..men.len() {
        for c in a + 1..men.len() {
            let d = (((men[a].0 - men[c].0).pow(2) + (men[a].1 - men[c].1).pow(2)) as f64).sqrt() as i64;
            assert!(d >= 800, "two stack men {d} mm apart: {:?} {:?}", men, b.soldiers.iter().filter(|s| s.team == 0).map(|s| (s.pos, s.task, s.down(), s.act)).collect::<Vec<_>>());
        }
    }
}


#[test]
fn a_gap_on_the_way_in_is_staged_pied_and_crossed_one_at_a_time() {
    use scalia_ai::battle::Event;
    use scalia_core::nav::{Prop, PropKind};
    let k = Killhouse::new(80_000, 62_000);
    let mut props = k.props();
    // A wall across the way from the squad (south) to the house, with a 3 m gap in it at x = 90 m.
    props.push(Prop { kind: PropKind::Wall { length_mm: 30_000, thickness_mm: 600 }, x_mm: 73_500, y_mm: 45_000, quarter_turns: 0 });
    props.push(Prop { kind: PropKind::Wall { length_mm: 30_000, thickness_mm: 600 }, x_mm: 106_500, y_mm: 45_000, quarter_turns: 0 });
    let mut nav = NavGrid::new(40_000, 20_000, 200, 160);
    nav.rebuild(&props);
    let sight = SightGrid::build(40_000, 20_000, 400, 320, &props);
    let cover = CoverMap::build(&props, &nav);
    let soldiers: Vec<Soldier> = (0..6).map(|i| Soldier::new(0, 0, (86_000 + (i as i64 % 3) * 1_500, 33_000 + (i as i64 / 3) * 1_500), Stance::Stand)).collect();
    let mut b = Battle::with_map(nav, sight, cover, soldiers, 5);
    b.set_building(k);
    b.command(0, Order::Capture);
    let mut ms = 0;
    let mut first_through: Vec<i64> = vec![-1; 6];
    while ms < 120_000 {
        b.step(50);
        ms += 50;
        for i in 0..6 {
            if first_through[i] < 0 && b.soldiers[i].pos.1 > 46_000 {
                first_through[i] = ms;
            }
        }
        if first_through.iter().all(|&t| t >= 0) {
            break;
        }
    }
    let steps: Vec<&str> = b.log.iter().filter_map(|e| if let Event::Order { squad: 0, step } = e { Some(*step) } else { None }).collect();
    let c = steps.iter().position(|&s| s == "chokepoint").expect(&format!("the gap was seen: {steps:?}"));
    assert!(steps[c..].contains(&"pie"), "and pied: {steps:?}");
    // Through one at a time: nobody at the same moment as another.
    let mut t: Vec<i64> = first_through.clone();
    t.sort();
    assert!(t[0] >= 0, "they got through: {first_through:?}");
    assert!(t.windows(2).all(|w| w[1] - w[0] >= 300), "one at a time: {t:?}");
}
