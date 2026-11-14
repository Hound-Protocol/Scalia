//! The testing ground's layout: the walled base, the live-fire range and its training house (millimetres).
use scalia_core::nav::{Furnish, Prop, PropKind};

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
    p.extend(barracks_props());
    p
}

/// The squads' barracks, south of the container stacks: walls (x0, y0, x1, y1; mm), roofless so the
/// commander sees in. Three rooms west to east: bay 0 (a squad's twelve bunks), the common room (two tables
/// with benches, weapons racks), bay 1 (the second squad). Doors: the main one in the north wall into the
/// common room, one in the west wall into bay 0, and one between each bay and the common room.
pub(crate) const BARRACKS_MM: (i64, i64, i64, i64) = (118_000, 4_000, 154_000, 24_000);
/// The partitions between the bays and the common room (x, mm).
const BARRACKS_SPLIT_MM: [i64; 2] = [130_000, 142_000];
const BARRACKS_WALL_MM: i64 = 300;
const BARRACKS_DOOR_MM: i64 = 2_000;

/// What a man does at a spot in the barracks (the animations to test there).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Use {
    /// Standing by his bunk (where he waits at ease).
    StandBy,
    /// Lying in a bunk (the lower or upper one).
    Lie { upper: bool },
    /// Sitting: on a footlocker or a bench.
    Sit,
    /// At his locker.
    Locker,
    /// At the weapons rack.
    Rack,
}

/// A place in the barracks: where, which way he faces there, what he does, and whose it is (the bay's squad
/// and his place in it; the common room's are anybody's).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Spot {
    pub at: (i64, i64),
    pub face: (i64, i64),
    pub what: Use,
    pub bay: Option<usize>,
    pub man: Option<usize>,
}

/// The rooms' extents (x0, x1) west to east: bay 0, common room, bay 1.
fn barracks_rooms() -> [(i64, i64); 3] {
    let (x0, _, x1, _) = BARRACKS_MM;
    [(x0, BARRACKS_SPLIT_MM[0]), (BARRACKS_SPLIT_MM[0], BARRACKS_SPLIT_MM[1]), (BARRACKS_SPLIT_MM[1], x1)]
}

/// A wall from a to b along one axis, with door gaps (centres along the wall) left open.
fn wall_run(p: &mut Vec<Prop>, along_x: bool, at: i64, from: i64, to: i64, doors: &[i64]) {
    let mut cuts: Vec<(i64, i64)> = doors.iter().map(|&d| (d - BARRACKS_DOOR_MM / 2, d + BARRACKS_DOOR_MM / 2)).collect();
    cuts.sort();
    let mut a = from;
    for (c0, c1) in cuts.into_iter().chain([(to, to)]) {
        if c0 > a {
            let (mid, len) = ((a + c0) / 2, c0 - a);
            let (x, y) = if along_x { (mid, at) } else { (at, mid) };
            p.push(Prop { kind: PropKind::Wall { length_mm: len, thickness_mm: BARRACKS_WALL_MM }, x_mm: x, y_mm: y, quarter_turns: if along_x { 0 } else { 1 } });
        }
        a = c1;
    }
}

/// Bunk centres in a bay: three along each long wall, head to the wall (x centres, and y of the south and north
/// rows).
fn bunk_rows(bay: (i64, i64)) -> ([i64; 3], i64, i64) {
    let (_, y0, _, y1) = BARRACKS_MM;
    let w = BARRACKS_WALL_MM / 2;
    ([bay.0 + 1_500, bay.0 + 5_500, bay.0 + 9_500], y0 + w + 1_050, y1 - w - 1_050)
}

/// The two lockers that go with a bunk at x: against the wall, in the space east of it (centres along x).
fn lockers_of(x: i64) -> [i64; 2] {
    [x + 900, x + 1_550]
}
/// A locker against a wall: its centre's distance from the wall's middle line, and its size (along, deep).
const LOCKER_OFF_MM: i64 = BARRACKS_WALL_MM / 2 + 250;
const LOCKER_MM: (i64, i64) = (600, 500);

/// The barracks' walls and furniture.
pub(crate) fn barracks_props() -> Vec<Prop> {
    let (x0, y0, x1, y1) = BARRACKS_MM;
    let mut p = Vec::new();
    let [bay0, common, bay1] = barracks_rooms();
    let mid_y = (y0 + y1) / 2;
    wall_run(&mut p, true, y0, x0 - BARRACKS_WALL_MM / 2, x1 + BARRACKS_WALL_MM / 2, &[]);
    wall_run(&mut p, true, y1, x0 - BARRACKS_WALL_MM / 2, x1 + BARRACKS_WALL_MM / 2, &[(common.0 + common.1) / 2]);
    wall_run(&mut p, false, x0, y0, y1, &[mid_y]);
    wall_run(&mut p, false, x1, y0, y1, &[]);
    for &x in &BARRACKS_SPLIT_MM {
        wall_run(&mut p, false, x, y0, y1, &[mid_y]);
    }
    let item = |p: &mut Vec<Prop>, item: Furnish, x: i64, y: i64, w: i64, d: i64| p.push(Prop { kind: PropKind::Furniture { item, width_mm: w, depth_mm: d }, x_mm: x, y_mm: y, quarter_turns: 0 });
    for bay in [bay0, bay1] {
        let (xs, ys, yn) = bunk_rows(bay);
        for &x in &xs {
            for (y, toward) in [(ys, 1), (yn, -1)] {
                // The bunk (0.9 x 2.0 m), its footlocker at the foot, a locker beside its head.
                item(&mut p, Furnish::Bunk, x, y, 900, 2_000);
                item(&mut p, Furnish::Bench, x, y + toward * 1_250, 900, 450);
                let wall_y = if toward > 0 { BARRACKS_MM.1 } else { BARRACKS_MM.3 };
                for lx in lockers_of(x) {
                    item(&mut p, Furnish::Locker, lx, wall_y + toward * LOCKER_OFF_MM, LOCKER_MM.0, LOCKER_MM.1);
                }
            }
        }
    }
    // The common room: two tables with a bench each side, weapons racks along the south wall.
    let cx = (common.0 + common.1) / 2;
    for ty in [mid_y - 4_000, mid_y + 3_000] {
        item(&mut p, Furnish::Table, cx, ty, 3_000, 1_000);
        item(&mut p, Furnish::Bench, cx, ty - 900, 3_000, 350);
        item(&mut p, Furnish::Bench, cx, ty + 900, 3_000, 350);
    }
    for k in 0..3 {
        item(&mut p, Furnish::Rack, common.0 + 2_500 + k * 3_500, y0 + BARRACKS_WALL_MM / 2 + 350, 2_000, 500);
    }
    p
}

/// Every place in the barracks, with what a man does there.
pub(crate) fn barracks_spots() -> Vec<Spot> {
    let (_, y0, _, _) = BARRACKS_MM;
    let [bay0, common, bay1] = barracks_rooms();
    let mut out = Vec::new();
    for (b, bay) in [bay0, bay1].into_iter().enumerate() {
        let (xs, ys, yn) = bunk_rows(bay);
        let mut man = 0;
        for (y, toward) in [(ys, 1i64), (yn, -1)] {
            for &x in &xs {
                // Two men to a bunk: lower and upper.
                for upper in [false, true] {
                    let m = Some(man);
                    let side = if upper { 1 } else { -1 };
                    out.push(Spot { at: (x + side * 400, y + toward * 2_200), face: (0, toward), what: Use::StandBy, bay: Some(b), man: m });
                    out.push(Spot { at: (x, y), face: (0, toward), what: Use::Lie { upper }, bay: Some(b), man: m });
                    out.push(Spot { at: (x + side * 220, y + toward * 1_250), face: (0, toward), what: Use::Sit, bay: Some(b), man: m });
                    let wall_y = if toward > 0 { BARRACKS_MM.1 } else { BARRACKS_MM.3 };
                    let lx = lockers_of(x)[upper as usize];
                    out.push(Spot { at: (lx, wall_y + toward * (LOCKER_OFF_MM + LOCKER_MM.1 / 2 + 450)), face: (0, -toward), what: Use::Locker, bay: Some(b), man: m });
                    man += 1;
                }
            }
        }
    }
    let (cx, mid_y) = ((common.0 + common.1) / 2, (BARRACKS_MM.1 + BARRACKS_MM.3) / 2);
    for ty in [mid_y - 4_000, mid_y + 3_000] {
        for k in -2..=2i64 {
            out.push(Spot { at: (cx + k * 600, ty - 900), face: (0, 1), what: Use::Sit, bay: None, man: None });
            out.push(Spot { at: (cx + k * 600, ty + 900), face: (0, -1), what: Use::Sit, bay: None, man: None });
        }
    }
    for k in 0..3 {
        out.push(Spot { at: (common.0 + 2_500 + k * 3_500, y0 + BARRACKS_WALL_MM / 2 + 1_100), face: (0, -1), what: Use::Rack, bay: None, man: None });
    }
    out
}

/// Where a man of a barracks squad waits at ease: standing at the foot of his bunk, facing the aisle
/// (squad = bay).
pub(crate) fn billet(squad: usize, man: usize) -> Option<Spot> {
    barracks_spots().into_iter().find(|s| s.bay == Some(squad) && s.man == Some(man) && s.what == Use::StandBy)
}

/// The middle of a bay's floor (the squad's home position).
pub(crate) fn bay_centre(squad: usize) -> (i64, i64) {
    let r = barracks_rooms()[if squad == 0 { 0 } else { 2 }];
    ((r.0 + r.1) / 2, (BARRACKS_MM.1 + BARRACKS_MM.3) / 2)
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

#[cfg(test)]
mod lab_more {
    use super::*;
    use crate::assault::{Assault, DEMO_SQUAD, LAB_RED};
    use scalia_ai::leader::{Mode, Order};

    /// The lab's other scenarios, natively: (blue standing, blue total, red standing, red total).
    fn run(name: &str, seed: u64) -> (usize, usize, usize, usize) {
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        let props = base_props();
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &props);
        map.killhouse = Some(KILLHOUSE_MM);
        let house = name == "contact";
        map.live = if house { Some(RANGE_LIVE_MM) } else { None };
        let mut a = Assault::field_with(map, KILLHOUSE_MM, seed, 0.0, house);
        let block = |x0: i64, y0: i64, n: i64| -> Vec<(i64, i64)> { (0..n).map(|i| (x0 + (i % 4) * 1_500, y0 + (i / 4) * 1_500)).collect() };
        match name {
            "contact" => {
                a.spawn_squad(DEMO_SQUAD, 0, &block(RANGE_MM.2 - 7_000, RANGE_MM.1 + 2_000, 8));
                a.order(DEMO_SQUAD, Order::Move { to: (RANGE_MM.2 - 5_000, RANGE_MM.3 - 3_000), mode: Mode::Plain });
            }
            "standoff-swapped" => {
                // The same, Red brought on first (lower soldier numbers): any difference is order bias.
                a.spawn_squad(LAB_RED, 1, &block(140_000, 92_000, 8));
                a.order(LAB_RED, Order::Move { to: (80_000, 95_000), mode: Mode::Confirmed });
                a.spawn_squad(DEMO_SQUAD, 0, &block(78_000, 92_000, 8));
                a.order(DEMO_SQUAD, Order::Move { to: (142_000, 95_000), mode: Mode::Confirmed });
            }
            "standoff-mirrored" => {
                // Sides swapped on the ground: any difference with "standoff" is the ground.
                a.spawn_squad(DEMO_SQUAD, 0, &block(140_000, 92_000, 8));
                a.order(DEMO_SQUAD, Order::Move { to: (80_000, 95_000), mode: Mode::Confirmed });
                a.spawn_squad(LAB_RED, 1, &block(78_000, 92_000, 8));
                a.order(LAB_RED, Order::Move { to: (142_000, 95_000), mode: Mode::Confirmed });
            }
            _ => {
                a.spawn_squad(DEMO_SQUAD, 0, &block(78_000, 92_000, 8));
                a.order(DEMO_SQUAD, Order::Move { to: (142_000, 95_000), mode: Mode::Confirmed });
                a.spawn_squad(LAB_RED, 1, &block(140_000, 92_000, 8));
                a.order(LAB_RED, Order::Move { to: (80_000, 95_000), mode: Mode::Confirmed });
            }
        }
        for _ in 0..(150_000 / 50) {
            a.battle.step(50);
        }
        let b = &a.battle;
        let up = |team| b.soldiers.iter().filter(|s| s.team == team && s.alive() && !s.down()).count();
        let all = |team| b.soldiers.iter().filter(|s| s.team == team).count();
        (up(0), all(0), up(1), all(1))
    }

    /// `cargo test -p scalia-wasm --release lab_more_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lab_more_report() {
        for name in ["contact", "standoff", "standoff-swapped", "standoff-mirrored"] {
            let (mut bu, mut ru, mut n) = (0, 0, 0);
            for seed in 1..=20 {
                let r = run(name, seed);
                bu += r.0;
                ru += r.2;
                n = r.1.max(n);
            }
            println!("{name}: blue standing {:.1}, red standing {:.1} (of {n} / 8) after 150 s", bu as f64 / 20.0, ru as f64 / 20.0);
        }
    }
}

#[cfg(test)]
mod barracks_tests {
    use super::*;
    use scalia_core::nav::NavGrid;

    fn grid() -> NavGrid {
        let mut g = NavGrid::new((BASE_LO * 1000.0) as i64, (BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 2.0) as usize, ((BASE_HI - BASE_LO) * 2.0) as usize);
        g.rebuild(&base_props());
        g
    }

    #[test]
    fn every_man_of_both_squads_has_a_bunk_a_seat_a_locker_and_a_place_to_stand() {
        let spots = barracks_spots();
        for bay in 0..2 {
            for man in 0..12 {
                for what in [Use::StandBy, Use::Sit, Use::Locker] {
                    assert_eq!(spots.iter().filter(|s| s.bay == Some(bay) && s.man == Some(man) && s.what == what).count(), 1, "bay {bay} man {man} {what:?}");
                }
                assert!(spots.iter().any(|s| s.bay == Some(bay) && s.man == Some(man) && matches!(s.what, Use::Lie { .. })));
            }
        }
        assert!(spots.iter().filter(|s| s.bay.is_none() && s.what == Use::Sit).count() >= 20, "seats at the tables");
    }

    #[test]
    fn the_barracks_sits_south_of_the_containers_inside_the_base_clear_of_everything_else() {
        let (x0, y0, x1, y1) = BARRACKS_MM;
        assert!(x0 > 20_000 && x1 < 160_000 && y0 > 0, "inside the perimeter");
        assert!(y1 < 30_000 - 1_250, "south of the containers");
        let ours = barracks_props();
        for p in base_props().iter().filter(|p| !ours.contains(p)) {
            let (hx, hy) = p.half_extent();
            let clear = p.x_mm + hx < x0 || p.x_mm - hx > x1 || p.y_mm + hy < y0 || p.y_mm - hy > y1;
            assert!(clear, "{p:?} overlaps the barracks");
        }
    }

    #[test]
    fn a_man_can_walk_from_the_base_to_every_standing_spot_and_every_seat() {
        let g = grid();
        let from = (110_000, 40_000);
        for s in barracks_spots().iter().filter(|s| matches!(s.what, Use::StandBy | Use::Sit | Use::Locker | Use::Rack)) {
            // Seats and stands are against furniture: walkable ground within a metre and a half (the last steps are the
            // animation's) is what counts.
            let ring = |r: i64| (0..8).map(move |k| { let a = k as f64 * std::f64::consts::FRAC_PI_4; ((a.cos() * r as f64) as i64, (a.sin() * r as f64) as i64) });
            let near = std::iter::once((0, 0)).chain(ring(600)).chain(ring(1_000)).chain(ring(1_500)).map(|d| (s.at.0 + d.0, s.at.1 + d.1)).find(|&p| g.walkable_at(p.0, p.1, 300));
            let p = near.unwrap_or_else(|| panic!("{s:?} is boxed in"));
            assert!(scalia_core::nav::find_path(&g, from, p, 300).is_some(), "no way to {s:?}");
        }
    }
}
