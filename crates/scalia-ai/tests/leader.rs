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
    let defenders_alive = b.soldiers.iter().filter(|s| s.team == 1 && s.alive()).count();
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
