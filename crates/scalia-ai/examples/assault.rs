//! Runs the killhouse assault and prints its timeline (leader steps, kills, grenades, cover calls).
use scalia_ai::battle::{Battle, Event, Soldier, Stance};
use scalia_ai::leader::Order;
use scalia_ai::sight::SightGrid;
use scalia_ai::tactics::CoverMap;
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::NavGrid;
fn main() {
    let seed: u64 = std::env::args().nth(1).map_or(21, |s| s.parse().unwrap());
    let k = Killhouse::new(80_000, 62_000);
    let props = k.props();
    let mut nav = NavGrid::new(40_000, 20_000, 200, 160);
    nav.rebuild(&props);
    let sight = SightGrid::build(40_000, 20_000, 400, 320, &props);
    let cover = CoverMap::build(&props, &nav);
    let mut soldiers: Vec<Soldier> = (0..12).map(|i| Soldier::new(0, 0, (86_000 + (i as i64 % 4) * 1_500, 35_000 + (i as i64 / 4) * 1_500), Stance::Stand)).collect();
    for (x, y) in [(12_500, 3_500), (16_000, 2_000), (5_000, 2_000), (2_500, 5_000), (3_000, 10_600), (6_500, 12_000), (16_500, 10_200), (13_000, 12_500)] {
        soldiers.push(Soldier::new(1, 1, (80_000 + x, 62_000 + y), Stance::Stand));
    }
    let mut b = Battle::with_map(nav, sight, cover, soldiers, seed);
    b.set_building(k);
    b.command(0, Order::Capture);
    b.command(1, Order::Hold);
    let mut seen = 0;
    let (mut shots, mut cover_calls) = (0, 0);
    for t in 0..(300_000 / 50) {
        b.step(50);
        for e in &b.log[seen..] {
            match e {
                Event::Order { squad, step } => println!("{:6.1}s  squad {squad}: {step}", t as f64 * 0.05),
                Event::Killed { soldier, by } => println!("{:6.1}s  killed {soldier} (team {}) by {}", t as f64 * 0.05, b.soldiers[*soldier].team, if *by == usize::MAX { "grenade".into() } else { by.to_string() }),
                Event::Explosion { .. } => println!("{:6.1}s  explosion", t as f64 * 0.05),
                Event::Shot { .. } => shots += 1,
                Event::Bark { bark: scalia_ai::battle::Bark::TakingCover, .. } => cover_calls += 1,
                _ => {}
            }
        }
        seen = b.log.len();
        if b.squad_done(0) { break; }
    }
    let alive = |team| b.soldiers.iter().filter(|s| s.team == team && s.alive()).count();
    println!("end {:.1}s: attackers alive {}/12, defenders alive {}/8, shots {shots}, cover calls {cover_calls}", b.time_ms as f64 / 1000.0, alive(0), alive(1));
}
