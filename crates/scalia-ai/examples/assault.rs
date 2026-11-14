//! Runs the killhouse assault and prints its timeline (leader steps, kills, grenades, cover calls).
use scalia_ai::battle::Event;
fn main() {
    let seed: u64 = std::env::args().nth(1).map_or(21, |s| s.parse().unwrap());
    let mut b = scalia_ai::scenario::killhouse_assault((80_000, 62_000), seed);
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
    let alive = |team| b.soldiers.iter().filter(|s| s.team == team && (s.alive() && !s.down())).count();
    println!("end {:.1}s: attackers alive {}/12, defenders alive {}/8, shots {shots}, cover calls {cover_calls}", b.time_ms as f64 / 1000.0, alive(0), alive(1));
}
