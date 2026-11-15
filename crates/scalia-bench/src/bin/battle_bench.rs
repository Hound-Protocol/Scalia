//! Battle AI workloads: two forces of `n/2` each on 400 × 300 m of ground with scattered walls and rocks.
//! Prints the average and worst cost of a 50 ms step for: standing (idle), marching (moving, no contact)
//! and in contact (closing and fighting). Usage: battle_bench [n ...]
use scalia_ai::battle::{Battle, Gait, Soldier, Stance, Task};
use scalia_ai::save::MapSpec;
use scalia_core::hash::mix64;
use scalia_core::nav::{Prop, PropKind};
use std::time::Instant;

fn field(seed: u64) -> MapSpec {
    let mut props = Vec::new();
    for k in 0..240u64 {
        let r = mix64(seed ^ k);
        let (x, y) = (20_000 + (r % 360_000) as i64, 20_000 + ((r >> 20) % 260_000) as i64);
        let kind = if k % 3 == 0 { PropKind::Rock { radius_mm: 900 } } else { PropKind::Wall { length_mm: 4_000 + (r >> 40) as i64 % 6_000, thickness_mm: 600 } };
        props.push(Prop { kind, x_mm: x, y_mm: y, quarter_turns: (r >> 50) as u8 % 2 });
    }
    MapSpec::new(0, 0, 400_000, 300_000, &props)
}

fn battle(n: usize, gap_mm: i64) -> Battle {
    let half = n / 2;
    let mut soldiers = Vec::with_capacity(n);
    for team in 0..2u8 {
        for i in 0..half {
            // Squads of 12 in lines 4 m apart; the two forces face each other `gap_mm` apart.
            let (col, row) = ((i % 60) as i64, (i / 60) as i64);
            let y = if team == 0 { 150_000 - gap_mm / 2 - row * 4_000 } else { 150_000 + gap_mm / 2 + row * 4_000 };
            let mut s = Soldier::new(team, (team as usize * 1_000 + i / 12) as u16, (40_000 + col * 5_000, y), Stance::Stand);
            s.facing = if team == 0 { (0, 1_000) } else { (0, -1_000) };
            soldiers.push(s);
        }
    }
    Battle::on_map(field(7), soldiers, 1)
}

/// Step times for `steps` steps of 50 ms: (median, 99th percentile, worst), in ms.
fn time(b: &mut Battle, steps: usize) -> (f64, f64, f64) {
    let mut ms: Vec<f64> = (0..steps)
        .map(|_| {
            let t = Instant::now();
            b.step(50);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    ms.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
    (at(0.5), at(0.99), ms[ms.len() - 1])
}

/// The step budget: a 50 ms battle step must finish well inside its own 50 ms (the rest of the frame needs time).
const CHECK_P99_MS: f64 = 35.0;

fn main() {
    // `battle_bench [--check] [sizes...]`: with --check, fail if the contact step's 99th percentile is over budget.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let sizes: Vec<usize> = args.iter().filter_map(|s| s.parse().ok()).collect();
    let sizes = if sizes.is_empty() { vec![200, 500, 1_000, 2_000] } else { sizes };
    println!("soldiers | idle p50/p99/worst ms | marching p50/p99/worst ms | contact p50/p99/worst ms   (per 50 ms step)");
    let mut over = Vec::new();
    for n in sizes {
        // Idle: far apart, nothing to do.
        let mut b = battle(n, 280_000);
        let idle = time(&mut b, 40);
        // Marching: everyone walks 60 m forward, out of contact.
        let mut b = battle(n, 280_000);
        for i in 0..b.soldiers.len() {
            let p = b.soldiers[i].pos;
            let dy = if b.soldiers[i].team == 0 { -60_000 } else { 60_000 };
            b.order(i, Task::Move { to: (p.0, (p.1 + dy).clamp(5_000, 295_000)), gait: Gait::Walk });
        }
        let march = time(&mut b, 40);
        // Contact: 120 m apart, both sides advance into each other.
        let mut b = battle(n, 120_000);
        for i in 0..b.soldiers.len() {
            let p = b.soldiers[i].pos;
            b.order(i, Task::Move { to: (p.0, 150_000), gait: Gait::Walk });
        }
        let contact = time(&mut b, 100);
        let f = |t: (f64, f64, f64)| format!("{:6.2} /{:6.2} /{:6.2}", t.0, t.1, t.2);
        println!("{n:8} | {} | {} | {}", f(idle), f(march), f(contact));
        if contact.1 > CHECK_P99_MS {
            over.push(format!("{n} soldiers in contact: p99 {:.1} ms over {CHECK_P99_MS} ms", contact.1));
        }
    }
    if check && !over.is_empty() {
        eprintln!("battle step over budget: {}", over.join("; "));
        std::process::exit(1);
    }
}
