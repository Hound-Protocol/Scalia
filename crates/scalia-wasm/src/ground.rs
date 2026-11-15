//! The testing ground's layout, read from the world map (scalia-world: `data/testing_ground.ron`, objects from
//! `data/catalog.ron`): the walled base, the live-fire range and its training house, the barracks. Millimetres.
use scalia_core::nav::Prop;
use scalia_world::{Spot, Use, World};

pub(crate) const BASE_LO: f32 = -20.0;
pub(crate) const BASE_HI: f32 = 220.0;

fn world() -> &'static World {
    World::testing_ground()
}

fn area(name: &str) -> (i64, i64, i64, i64) {
    world().area(name).unwrap_or_else(|| panic!("the testing ground has an area '{name}'"))
}

/// Everything solid on the testing ground that does not open (doors are the world's `doors()`).
pub(crate) fn base_props() -> Vec<Prop> {
    world().props()
}

/// The training house on the testing ground (see scalia_core::killhouse).
pub(crate) fn killhouse() -> scalia_core::killhouse::Killhouse {
    world().killhouses().into_iter().next().expect("the testing ground has its training house")
}
/// Its south-west corner (absolute millimetres): inside the live-fire range.
pub(crate) fn killhouse_mm() -> (i64, i64) {
    killhouse().origin
}
/// The live-fire range's walls (x0, y0, x1, y1).
pub(crate) fn range_mm() -> (i64, i64, i64, i64) {
    area("range")
}
/// Inside the range's walls: live (soldiers see and fight each other only in here).
pub(crate) fn range_live_mm() -> (i64, i64, i64, i64) {
    area("range_live")
}
/// The door in from the base: the door in the range's east wall.
pub(crate) fn range_door() -> (i64, i64) {
    let r = range_mm();
    world().doors().iter().find(|d| d.at.0 == r.2).map(|d| d.at).expect("the range has its door in the east wall")
}
/// The squads' barracks (x0, y0, x1, y1), south of the container stacks.
pub(crate) fn barracks_mm() -> (i64, i64, i64, i64) {
    area("barracks")
}

fn inside(r: (i64, i64, i64, i64), p: (i64, i64)) -> bool {
    p.0 > r.0 && p.0 < r.2 && p.1 > r.1 && p.1 < r.3
}

/// The bunks of a squad's bay in order: the south row west to east, then the north row.
fn bunks_of(squad: usize) -> Vec<usize> {
    let bay = area(&format!("barracks_bay_{squad}"));
    let mut b: Vec<(usize, (i64, i64))> = world().of_kind("bunk").filter(|(_, o)| inside(bay, o.at)).map(|(i, o)| (i, o.at)).collect();
    b.sort_by_key(|&(_, (x, y))| (y, x));
    b.into_iter().map(|(i, _)| i).collect()
}

/// A barracks man's places: man k of squad s has bunk k/2 of the squad's bay, the lower bunk if k is even.
/// Returns the place of `what` that is his (where he stands at its foot, which bunk he lies in, the seat on
/// its footlocker, his locker).
pub(crate) fn place_of(squad: usize, man: usize, what: Use) -> Option<Spot> {
    let bunk = *bunks_of(squad).get(man / 2)?;
    let w = world();
    let o = w.objects[bunk];
    let which = man % 2;
    match what {
        Use::Stand | Use::Lie => w.spots().into_iter().filter(|s| s.object == bunk && s.what == what).nth(which),
        Use::Sit => {
            // The footlocker at the foot of his bunk.
            let (f, _) = w.of_kind("footlocker").min_by_key(|(_, f)| (f.at.0 - o.at.0).abs() + (f.at.1 - o.at.1).abs())?;
            w.spots().into_iter().filter(|s| s.object == f && s.what == Use::Sit).nth(which)
        }
        Use::Work => {
            // The lockers beside his bunk's head, along the same wall: the nearer one is the lower man's.
            let mut l: Vec<(usize, i64)> = w.of_kind("locker").filter(|(_, l)| (l.at.1 - o.at.1).abs() < 2_000).map(|(i, l)| (i, (l.at.0 - o.at.0).abs())).filter(|&(_, d)| d < 2_000).collect();
            l.sort_by_key(|&(i, d)| (d, i));
            let (locker, _) = *l.get(which)?;
            w.spots().into_iter().find(|s| s.object == locker)
        }
        Use::Handle => None,
    }
}

/// Where a man of a barracks squad waits at ease: standing at the foot of his bunk, facing the aisle.
pub(crate) fn billet(squad: usize, man: usize) -> Option<Spot> {
    place_of(squad, man, Use::Stand)
}

/// The testing ground's doors as a battle on it starts (from the world map).
pub(crate) fn battle_doors() -> Vec<scalia_ai::doors::Door> {
    use scalia_ai::doors::{Door, DoorState};
    world()
        .doors()
        .iter()
        .map(|d| {
            let state = match d.state {
                scalia_world::State::Open => DoorState::Open,
                scalia_world::State::Closed => DoorState::Closed,
                scalia_world::State::Locked => DoorState::Locked,
                scalia_world::State::Breached => DoorState::Breached,
            };
            Door { lockable: d.lockable, ..Door::new(d.at, d.along_x, d.width_mm, d.height_mm, state) }
        })
        .collect()
}

/// Whether a point (metres) is on a concrete floor (the range, the barracks), not grass.
pub(crate) fn concrete_at(x: f64, y: f64) -> bool {
    let p = ((x * 1000.0) as i64, (y * 1000.0) as i64);
    inside(range_mm(), p) || inside(barracks_mm(), p)
}

/// Which of `battle_doors()` are steel.
pub(crate) fn door_steel() -> Vec<bool> {
    world().doors().iter().map(|d| d.material == scalia_world::Material::Steel).collect()
}

/// The middle of a bay's floor (the squad's home position).
pub(crate) fn bay_centre(squad: usize) -> (i64, i64) {
    let r = area(&format!("barracks_bay_{squad}"));
    ((r.0 + r.2) / 2, (r.1 + r.3) / 2)
}

#[cfg(test)]
mod commander_tests {
    use super::*;

    /// The commander on the testing ground, played roughly: he walks everywhere (base, range, barracks, the
    /// house), changes stance, aims and fires in every direction and reloads, while the battle runs and is
    /// drawn every frame. Nothing may panic.
    #[test]
    fn the_commander_fights_anywhere_without_breaking_anything() {
        use crate::assault::Assault;
        use scalia_ai::battle::Stance;
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &base_props());
        map.doors = battle_doors();
        map.killhouse = Some(killhouse_mm());
        map.live = Some(range_live_mm());
        let mut a = Assault::field(map, killhouse_mm(), 7, 0.0);
        let mut now = 0.0;
        let mut rng = 0x9e37_79b9_7f4a_7c15u64;
        let mut r = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
        let mut at = (90_000i64, 20_000i64);
        let me = a.battle.join_player(0, 1_000, at);
        for frame in 0..6_000 {
            if frame % 120 == 0 {
                // A new heading and stance now and then; sometimes far off the map edge on purpose.
                at = (lo + (r() % size as u64) as i64, lo + (r() % size as u64) as i64);
            }
            let stance = [Stance::Stand, Stance::Crouch, Stance::Prone][(r() % 3) as usize];
            let dir = ((r() % 2001) as i64 - 1000, (r() % 2001) as i64 - 1000);
            a.battle.player_set(me, at, dir, stance, (r() % 6000) as i64);
            let _ = a.battle.player_fire(me, dir, (r() % 1001) as i64);
            if r() % 50 == 0 {
                a.battle.player_reload(me);
            }
            now += 16.7;
            a.advance(now);
            for team in 0..2 {
                let _ = a.instances(team, now as f32 / 1000.0, |_| 6.0, |x, y| Some((x as f32, y as f32, 0.0)));
            }
            for i in 0..a.battle.soldiers.len() {
                let _ = a.position(i);
            }
        }
        assert!(a.battle.soldiers[me].player);
        // A squad sent in after the commander joined (the user's crash: ordering a unit to clear a building).
        let n = a.battle.soldiers.len();
        a.spawn_squad(42, 0, &[(80_000, 30_000), (81_000, 30_000), (82_000, 30_000)]);
        for _ in 0..30 {
            now += 16.7;
            a.advance(now);
            let _ = a.instances(0, now as f32 / 1000.0, |_| 6.0, |x, y| Some((x as f32, y as f32, 0.0)));
        }
        assert_eq!(a.battle.soldiers.len(), n + 3);
        assert!(a.battle.time_ms > 90_000, "the battle ran: {} ms", a.battle.time_ms);
    }
}

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
        map.doors = if std::env::var("NODOORS").is_ok() { Vec::new() } else { battle_doors() };
        map.killhouse = Some(killhouse_mm());
        map.live = Some(range_live_mm());
        let mut a = Assault::field_with(map, killhouse_mm(), seed, 0.0, true);
        let door = (range_mm().2, range_door().1);
        let block: Vec<(i64, i64)> = (0..12).map(|i| (door.0 + 8_000 + (i % 4) * 1_500, door.1 - 3_000 + (i / 4) * 1_500)).collect();
        a.spawn_squad(DEMO_SQUAD, 0, &block);
        a.order(DEMO_SQUAD, scalia_ai::leader::Order::Capture);
        let b = &mut a.battle;
        let mut seen = 0;
        let mut was_down = vec![false; 64];
        let mut was_down2 = vec![false; 64];
        for _ in 0..(240_000 / 50) {
            b.step(50);
            if print && std::env::var("AT").ok().and_then(|v| v.parse::<i64>().ok()) == Some(b.time_ms) {
                for (i, s) in b.soldiers.iter().enumerate().filter(|(_, s)| s.team == 0) {
                    println!("  #{i} ({:.1},{:.1}) {:?} act {:?} cover {:?} [{}]", s.pos.0 as f64 / 1e3, s.pos.1 as f64 / 1e3, s.task, s.act, s.cover.map(|c| b.cover.as_ref().map(|m| m.spots[c].pos)), crate::assault::lab_state(b, i));
                }
                println!("  doors {:?}", b.doors.iter().map(|d| d.state).collect::<Vec<_>>());
            }
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
                        Event::Door { door, state, by } => println!("{t:6.1}s  door {door} at {:?} -> {state:?} by {by:?}", b.doors[*door].at),
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
        let mut a = setup(name, seed);
        for _ in 0..(150_000 / 50) {
            a.battle.step(50);
        }
        let b = &a.battle;
        let up = |team| b.soldiers.iter().filter(|s| s.team == team && s.alive() && !s.down()).count();
        let all = |team| b.soldiers.iter().filter(|s| s.team == team).count();
        (up(0), all(0), up(1), all(1))
    }

    /// A lab scenario set up and ordered, not yet run.
    fn setup(name: &str, seed: u64) -> Assault {
        let (lo, size) = ((BASE_LO * 1000.0) as i64, ((BASE_HI - BASE_LO) * 1000.0) as i64);
        let props = base_props();
        let mut map = scalia_ai::save::MapSpec::new(lo, lo, size, size, &props);
        map.doors = if std::env::var("NODOORS").is_ok() { Vec::new() } else { battle_doors() };
        map.killhouse = Some(killhouse_mm());
        let house = name == "contact";
        map.live = if house { Some(range_live_mm()) } else { None };
        let mut a = Assault::field_with(map, killhouse_mm(), seed, 0.0, house);
        let block = |x0: i64, y0: i64, n: i64| -> Vec<(i64, i64)> { (0..n).map(|i| (x0 + (i % 4) * 1_500, y0 + (i / 4) * 1_500)).collect() };
        match name {
            "contact" => {
                a.spawn_squad(DEMO_SQUAD, 0, &block(range_mm().2 - 7_000, range_mm().1 + 2_000, 8));
                a.order(DEMO_SQUAD, Order::Move { to: (range_mm().2 - 5_000, range_mm().3 - 3_000), mode: Mode::Plain });
                // The house's doors stand open: its defenders see (and are seen) through them as Blue goes by.
                for d in 0..a.battle.doors.len() {
                    a.battle.set_door(d, scalia_ai::doors::DoorState::Open, None);
                }
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
        a
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

    /// Behaviour that reads as odd, measured over the lab scenarios (per run, 20 seeds):
    /// `cargo test -p scalia-wasm --release lab_behaviour_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lab_behaviour_report() {
        use scalia_ai::battle::Event;
        for name in ["contact", "standoff"] {
            let mut m = [0f64; 8];
            let seeds = 20;
            for seed in 1..=seeds {
                let mut a = setup(name, seed);
                let mut was_moving = Vec::new();
                let mut flips: Vec<u32> = Vec::new();
                let mut idle_contact: Vec<i64> = Vec::new();
                for _ in 0..(150_000 / 50) {
                    let seen = a.battle.log.len();
                    a.battle.step(50);
                    let b = &a.battle;
                    let n = b.soldiers.len();
                    was_moving.resize(n, false);
                    flips.resize(n, 0);
                    idle_contact.resize(n, 0);
                    let up = |s: &scalia_ai::battle::Soldier| s.alive() && !s.down();
                    // Lockstep: many men of one squad setting off in the same step.
                    let mut starts = std::collections::HashMap::new();
                    for (i, s) in b.soldiers.iter().enumerate() {
                        if !up(s) {
                            continue;
                        }
                        if s.moving != was_moving[i] {
                            flips[i] += 1;
                            if s.moving {
                                *starts.entry(s.squad).or_insert(0) += 1;
                            }
                        }
                        was_moving[i] = s.moving;
                        // Standing in the open under fire, not moving and not in cover (seconds).
                        if s.suppression >= 100 && s.cover.is_none() && !s.moving && s.stance == scalia_ai::battle::Stance::Stand {
                            m[0] += 0.05;
                        }
                        // Squad knows of enemies, he is up, has no target, not moving, not in cover (seconds).
                        let knows = b.contacts.get(s.squad as usize).is_some_and(|c| !c.is_empty());
                        if knows && s.target.is_none() && !s.moving && s.cover.is_none() {
                            idle_contact[i] += 50;
                        } else {
                            idle_contact[i] = 0;
                        }
                        if idle_contact[i] == 5_000 {
                            m[1] += 1.0;
                        }
                        // Crowding: another friendly within 0.5 m.
                        if b.soldiers.iter().enumerate().any(|(j, o)| j > i && up(o) && o.team == s.team && (o.pos.0 - s.pos.0).abs() < 500 && (o.pos.1 - s.pos.1).abs() < 500) {
                            m[2] += 0.05;
                        }
                    }
                    m[3] += starts.values().filter(|&&c| c >= 4).count() as f64;
                    // Rounds fired past a friendly within a metre of the line (before the target).
                    for e in &b.log[seen..] {
                        if let Event::Shot { shooter, from, to, .. } = *e {
                            m[4] += 1.0;
                            let team = b.soldiers[shooter].team;
                            let (dx, dy) = ((to.x - from.x) as f64, (to.y - from.y) as f64);
                            let len2 = dx * dx + dy * dy;
                            if len2 <= 0.0 {
                                continue;
                            }
                            let risky = b.soldiers.iter().enumerate().any(|(j, o)| {
                                if j == shooter || o.team != team || !o.alive() {
                                    return false;
                                }
                                let (px, py) = ((o.pos.0 - from.x) as f64, (o.pos.1 - from.y) as f64);
                                let t = (px * dx + py * dy) / len2;
                                if !(0.05..0.95).contains(&t) {
                                    return false;
                                }
                                let (cx, cy) = (px - t * dx, py - t * dy);
                                cx * cx + cy * cy < 1_000.0 * 1_000.0
                            });
                            if risky {
                                m[5] += 1.0;
                            }
                        }
                    }
                }
                // Dithering: men starting and stopping a lot (flips per man per minute over 2.5 min).
                m[6] += flips.iter().map(|&f| f as f64 / 2.5).sum::<f64>() / flips.len().max(1) as f64;
                m[7] += flips.iter().filter(|&&f| f as f64 / 2.5 > 12.0).count() as f64;
            }
            let k = seeds as f64;
            println!("{name} (per run): exposed standing still under fire {:.1} man-s | idle with known enemy >5 s {:.1} times | crowded <0.5 m {:.1} man-s | 4+ men setting off in the same step {:.1} times | rounds {:.0}, past a friendly within 1 m {:.1} | start/stop per man-minute {:.1}, men dithering (>12/min) {:.1}",
                m[0] / k, m[1] / k, m[2] / k, m[3] / k, m[4] / k, m[5] / k, m[6] / k, m[7] / k);
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
    fn every_man_of_both_squads_has_his_own_bunk_seat_locker_and_place_to_stand() {
        for what in [Use::Stand, Use::Lie, Use::Sit, Use::Work] {
            let mut seen = Vec::new();
            for squad in 0..2 {
                for man in 0..12 {
                    let s = place_of(squad, man, what).unwrap_or_else(|| panic!("squad {squad} man {man} has no {what:?} place"));
                    assert!(inside(area(&format!("barracks_bay_{squad}")), s.at), "{what:?} of {squad}/{man} in his bay");
                    let key = (s.at, s.height_mm);
                    assert!(!seen.contains(&key), "{what:?} of {squad}/{man} shared");
                    seen.push(key);
                }
            }
        }
        let seats = world().spots().into_iter().filter(|s| s.what == Use::Sit && inside(area("barracks_common"), s.at)).count();
        assert!(seats >= 20, "seats at the tables: {seats}");
    }

    #[test]
    fn the_barracks_sits_south_of_the_containers_inside_the_base_clear_of_everything_else() {
        let (x0, y0, x1, y1) = barracks_mm();
        let base = area("base");
        assert!(x0 > base.0 && x1 < base.2 && y0 > base.1, "inside the perimeter");
        let containers = world().of_kind("container").map(|(_, o)| o.at.1 - o.size.1 / 2).min().expect("containers");
        assert!(y1 < containers, "south of the containers");
        for p in base_props() {
            let (hx, hy) = p.half_extent();
            let clear = p.x_mm + hx < x0 || p.x_mm - hx > x1 || p.y_mm + hy < y0 || p.y_mm - hy > y1;
            let ours = inside((x0 - 200, y0 - 200, x1 + 200, y1 + 200), (p.x_mm, p.y_mm));
            assert!(clear || ours, "{p:?} overlaps the barracks");
        }
    }

    #[test]
    fn a_man_can_walk_from_the_base_to_every_place_in_the_barracks() {
        let g = grid();
        let from = (110_000, 40_000);
        for s in world().spots().iter().filter(|s| inside(barracks_mm(), s.at) && s.what != Use::Lie) {
            // Places are against furniture: walkable ground within a metre and a half (the last steps are the
            // animation's) is what counts.
            let ring = |r: i64| (0..8).map(move |k| { let a = k as f64 * std::f64::consts::FRAC_PI_4; ((a.cos() * r as f64) as i64, (a.sin() * r as f64) as i64) });
            let near = std::iter::once((0, 0)).chain(ring(600)).chain(ring(1_000)).chain(ring(1_500)).map(|d| (s.at.0 + d.0, s.at.1 + d.1)).find(|&p| g.walkable_at(p.0, p.1, 300));
            let p = near.unwrap_or_else(|| panic!("{s:?} is boxed in"));
            assert!(scalia_core::nav::find_path(&g, from, p, 300).is_some(), "no way to {s:?}");
        }
    }
}
