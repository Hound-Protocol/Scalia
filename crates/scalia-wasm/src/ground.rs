//! The testing ground's layout: the walled base, the live-fire range and its training house (millimetres).
use scalia_core::nav::{Prop, PropKind};

pub(crate) const BASE_LO: f32 = -20.0;
pub(crate) const BASE_HI: f32 = 220.0;

/// The testing ground's military base (positions in millimetres). A walled compound with a gate, barracks,
/// guard towers, sandbag lines and containers, and outside it an obstacle lane: a rock field and a narrow gap.
pub(crate) fn base_props() -> Vec<Prop> {
    let mut p = Vec::new();
    let wall = |x: i64, y: i64, len: i64, thick: i64, turns: u8| Prop { kind: PropKind::Wall { length_mm: len, thickness_mm: thick }, x_mm: x, y_mm: y, quarter_turns: turns };
    // Perimeter: a 140 m square from (20, 0) to (160, 140) m, 0.6 m concrete. Closed off for now (the 12 m
    // gate in the south wall is walled up).
    p.push(wall(20_000 + 32_000, 0, 64_000, 600, 0));
    p.push(wall(160_000 - 32_000, 0, 64_000, 600, 0));
    p.push(wall(90_000, 0, 12_000, 600, 0));
    p.push(wall(90_000, 140_000, 140_000, 600, 0));
    p.push(wall(20_000, 70_000, 140_000, 600, 1));
    p.push(wall(160_000, 70_000, 140_000, 600, 1));
    // Barracks: three 24 x 8 m blocks along the north.
    for k in 0..3 {
        p.push(wall(45_000 + k * 40_000, 118_000, 24_000, 8_000, 0));
    }
    // Guard towers in the corners and by the gate (3 m footprints).
    for (x, y) in [(24_000, 4_000), (156_000, 4_000), (24_000, 136_000), (156_000, 136_000), (79_000, 5_000), (101_000, 5_000)] {
        p.push(Prop { kind: PropKind::Crate { size_mm: 3_000 }, x_mm: x, y_mm: y, quarter_turns: 0 });
    }
    // Sandbag lines behind the gate, staggered so the way in zig-zags.
    p.push(wall(78_000, 14_000, 10_000, 700, 0));
    p.push(wall(102_000, 22_000, 10_000, 700, 0));
    p.push(wall(68_000, 30_000, 8_000, 700, 1));
    // Container stacks (2.5 x 12 m) in the depot corner.
    for k in 0..4 {
        p.push(wall(140_000, 30_000 + k * 6_000, 12_000, 2_500, 0));
    }
    // Crates scattered by the depot.
    for k in 0..8i64 {
        p.push(Prop { kind: PropKind::Crate { size_mm: 1_200 }, x_mm: 120_000 + (k % 4) * 3_000, y_mm: 60_000 + (k / 4) * 4_000, quarter_turns: 0 });
    }
    // Outside, west: a rock field and a wall with a 2 m gap (soldiers squeeze through one at a time).
    for k in 0..24i64 {
        let h = scalia_core::hash::mix64(k as u64 ^ 0x70c4);
        p.push(Prop { kind: PropKind::Rock { radius_mm: 700 + (h % 1_400) as i64 }, x_mm: -10_000 + (h >> 16) as i64 % 24_000, y_mm: 20_000 + (h >> 32) as i64 % 90_000, quarter_turns: 0 });
    }
    p.push(wall(-2_500, 150_000, 31_000, 600, 0));
    p.push(wall(32_000 - 1_000, 150_000, 34_000, 600, 0));
    // Headquarters in the middle of the base.
    p.push(wall(90_000, 70_000, 16_000, 8_000, 0));
    // The live-fire range in the south-west corner: a concrete-walled yard on a concrete floor, the roofless
    // training house inside it (where the enemy squad is stationed), and one door in from the base, in the
    // east wall. Through that door it is live.
    let (x0, y0, x1, y1) = RANGE_MM;
    let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
    p.push(wall(cx, y0, x1 - x0, 600, 0));
    p.push(wall(cx, y1, x1 - x0, 600, 0));
    p.push(wall(x0, cy, y1 - y0, 600, 1));
    let (d0, d1) = (RANGE_DOOR_Y_MM - RANGE_DOOR_MM / 2, RANGE_DOOR_Y_MM + RANGE_DOOR_MM / 2);
    p.push(wall(x1, (y0 + d0) / 2, d0 - y0, 600, 1));
    p.push(wall(x1, (d1 + y1) / 2, y1 - d1, 600, 1));
    p.extend(killhouse().props());
    p
}

/// The training house on the testing ground (see scalia_core::killhouse).
pub(crate) fn killhouse() -> scalia_core::killhouse::Killhouse {
    scalia_core::killhouse::Killhouse::new(KILLHOUSE_MM.0, KILLHOUSE_MM.1)
}
/// Its south-west corner (absolute millimetres): inside the live-fire range.
pub(crate) const KILLHOUSE_MM: (i64, i64) = (34_000, 20_000);
/// The live-fire range: its walls (x0, y0, x1, y1; mm), the door in its east wall (centre y, width).
pub(crate) const RANGE_MM: (i64, i64, i64, i64) = (26_000, 8_000, 64_000, 46_000);
pub(crate) const RANGE_DOOR_Y_MM: i64 = 27_000;
pub(crate) const RANGE_DOOR_MM: i64 = 1_600;
/// Inside the range's walls: live (soldiers see and fight each other only in here).
pub(crate) const RANGE_LIVE_MM: (i64, i64, i64, i64) = (RANGE_MM.0 + 300, RANGE_MM.1 + 300, RANGE_MM.2 - 300, RANGE_MM.3 - 300);


#[cfg(test)]
mod lab_tests {
    use super::*;

    /// The lab's "clear" scenario, natively: Blue (12) briefs outside the range door and takes the house from
    /// Red (8). Returns (blue standing, red standing, seconds, done) and prints the timeline.
    fn run_clear(seed: u64, print: bool) -> (usize, usize, f64, bool) {
        use crate::assault::{Assault, DEMO_SQUAD};
        use scalia_ai::battle::Event;
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        let props = base_props();
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &props);
        map.killhouse = Some(KILLHOUSE_MM);
        map.live = Some(RANGE_LIVE_MM);
        let mut a = Assault::field_with(map, KILLHOUSE_MM, seed, 0.0, true);
        let door = (RANGE_MM.2, RANGE_DOOR_Y_MM);
        let block: Vec<(i64, i64)> = (0..12).map(|i| (door.0 + 8_000 + (i % 4) * 1_500, door.1 - 3_000 + (i / 4) * 1_500)).collect();
        a.spawn_squad(DEMO_SQUAD, 0, &block);
        a.order(DEMO_SQUAD, scalia_ai::leader::Order::Capture);
        let b = &mut a.battle;
        let mut seen = 0;
        let mut was_down = vec![false; 64];
        let mut was_down2 = vec![false; 64];
        for _ in 0..(240_000 / 50) {
            b.step(50);
            {
                for (i, s) in b.soldiers.iter().enumerate() {
                    let d = s.down() || !s.alive();
                    if d && !was_down[i] && print {
                        println!("{:6.1}s  DOWN #{i} team {} at ({:.1},{:.1}) phase '{}' {}", b.time_ms as f64 / 1000.0, s.team, s.pos.0 as f64 / 1000.0, s.pos.1 as f64 / 1000.0, b.squad_phase(s.squad as usize), crate::assault::lab_state(b, i));
                    }
                    was_down[i] = d;
                }
            }
            if print {
                for e in &b.log[seen..] {
                    let t = b.time_ms as f64 / 1000.0;
                    match e {
                        Event::Order { squad, step } if *step != "casualty" => println!("{t:6.1}s  squad {squad}: {step}"),
                        Event::Killed { soldier, by } => {
                            let s = &b.soldiers[*soldier];
                            println!("{t:6.1}s  hit {soldier} (team {}) by {by} at ({:.1},{:.1}) {}", s.team, s.pos.0 as f64 / 1000.0, s.pos.1 as f64 / 1000.0, if s.alive() { "down" } else { "dead" })
                        }
                        Event::Explosion { .. } => println!("{t:6.1}s  explosion"),
                        Event::Shot { shooter, target, hit: true, .. } if std::env::var("HITS").is_ok() && print => {
                            let (a, z) = (&b.soldiers[*shooter], &b.soldiers[*target]);
                            println!("{t:6.1}s    hit #{shooter}({:.1},{:.1}) -> #{target}({:.1},{:.1}) [{}] -> [{}]", a.pos.0 as f64 / 1e3, a.pos.1 as f64 / 1e3, z.pos.0 as f64 / 1e3, z.pos.1 as f64 / 1e3, crate::assault::lab_state(b, *shooter), crate::assault::lab_state(b, *target))
                        }
                        Event::Bark { soldier, bark: scalia_ai::battle::Bark::Smoke } => println!("{t:6.1}s  smoke by #{soldier} -> {:?}", b.smoke.last().map(|c| c.pos)),
                        _ => {}
                    }
                }
            }
            for e in &b.log[seen..] {
                if let Event::Shot { shooter, target, hit: true, .. } = e {
                    let (sh, t) = (&b.soldiers[*shooter], &b.soldiers[*target]);
                    if t.team == 0 && (t.down() || !t.alive()) && !was_down2[*target] {
                        was_down2[*target] = true;
                        let k = b.building().expect("house");
                        let (rs, rt) = (k.room_at(sh.pos.0, sh.pos.1), k.room_at(t.pos.0, t.pos.1));
                        let how = match (rt, rs) {
                            (None, None) => "out<-out",
                            (None, Some(_)) => "out<-in",
                            (Some(_), None) => "in<-out",
                            (Some(a), Some(c)) if a == c => "same room",
                            _ => "other room",
                        };
                        PHASES.with(|p| *p.borrow_mut().entry(format!("{} {how}", b.squad_phase(t.squad as usize))).or_insert(0) += 1);
                        if std::env::var("WHY").is_ok() && how == "same room" {
                            println!("  {:.1}s same-room: #{shooter} ({:.1},{:.1}) [{}] -> #{target} ({:.1},{:.1}) room {:?} cleared-order {}", b.time_ms as f64 / 1e3, sh.pos.0 as f64 / 1e3, sh.pos.1 as f64 / 1e3, crate::assault::lab_state(b, *shooter), t.pos.0 as f64 / 1e3, t.pos.1 as f64 / 1e3, rt, b.squad_phase(t.squad as usize));
                        }
                    }
                }
            }
            seen = b.log.len();
            if b.squad_done(DEMO_SQUAD as usize) {
                break;
            }
        }
        let up = |team| b.soldiers.iter().filter(|s| s.team == team && s.alive() && !s.down()).count();
        (up(0), up(1), b.time_ms as f64 / 1000.0, b.squad_done(DEMO_SQUAD as usize))
    }

    thread_local! {
        static PHASES: std::cell::RefCell<std::collections::BTreeMap<String, u32>> = Default::default();
    }

    /// Report, not a gate: `cargo test -p scalia-wasm --release lab_clear_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lab_clear_report() {
        let seeds: u64 = std::env::var("SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
        let mut blue = 0;
        for seed in 0..seeds {
            let r = run_clear(seed + 1, std::env::var("PRINT").ok().and_then(|p| p.parse::<u64>().ok()) == Some(seed + 1));
            println!("seed {:2}: blue {:2}/12  red {}/8  {:6.1}s  done {}", seed + 1, r.0, r.1, r.2, r.3);
            blue += r.0;
        }
        println!("mean blue standing {:.1}/12", blue as f64 / seeds as f64);
        PHASES.with(|p| println!("blue down by phase (per run): {}", p.borrow().iter().map(|(k, v)| format!("{k} {:.1}", *v as f64 / seeds as f64)).collect::<Vec<_>>().join(", ")));
    }
}
