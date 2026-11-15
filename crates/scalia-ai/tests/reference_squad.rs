//! AI-3: the reference squad. Twelve riflemen and their sergeant are sent across open ground towards a dug-in
//! enemy section; the run is checked against the ten steps of a squad action (order, move, detect, own cover,
//! coordinated fire, react to fire, suppression, casualties, regroup, complete or abandon).
use scalia_ai::battle::{Battle, Event, Soldier, Stance, Task};
use scalia_ai::leader::{Mode, Order};
use scalia_ai::save::Command;
use scalia_ai::sight::SightGrid;

const SQUAD: usize = 13;

fn man(team: u8, squad: u16, x: i64, y: i64, facing: (i64, i64), stance: Stance) -> Soldier {
    let mut s = Soldier::new(team, squad, (x, y), stance);
    s.facing = facing;
    s
}

#[derive(Default, Debug)]
struct Report {
    ordered: bool,
    moved_mm: i64,
    detect_s: Option<f64>,
    covered: usize,
    shooters: std::collections::BTreeSet<usize>,
    enemy_first_shot_s: Option<f64>,
    went_down_after_fire: usize,
    suppressed_foes: usize,
    killed: (usize, usize),
    regroups: usize,
    outcome: Option<&'static str>,
}

/// Runs one engagement; `foes` is the size of the enemy section (dug in, prone, 70 m ahead).
fn run(seed: u64, foes: usize, secs: i64) -> Report {
    let mut men: Vec<Soldier> = (0..SQUAD as i64).map(|k| man(0, 0, 50_000 + (k % 3 - 1) * 2_000, 40_000 - (k / 3) * 2_500, (0, 1_000), Stance::Stand)).collect();
    for k in 0..foes as i64 {
        men.push(man(1, 1, 50_000 + (k - foes as i64 / 2) * 4_000, 115_000, (0, -1_000), Stance::Prone));
    }
    let mut b = Battle::new(SightGrid::build(0, 0, 400, 400, &[]), men, seed);
    for i in SQUAD..SQUAD + foes {
        let p = b.soldiers[i].pos;
        b.soldiers[i].task = Task::Hold { at: p, face: (0, -1_000) };
    }
    let start = centroid(&b);
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (50_000, 112_000), mode: Mode::Plain } });
    let mut r = Report::default();
    let mut down_at_fire = 0;
    for step in 0..secs * 20 {
        b.step(50);
        let t = step as f64 * 0.05;
        for e in std::mem::take(&mut b.log) {
            match e {
                Event::Shot { shooter, .. } if shooter < SQUAD => {
                    r.shooters.insert(shooter);
                }
                Event::Shot { shooter, .. } if shooter >= SQUAD && r.enemy_first_shot_s.is_none() => {
                    r.enemy_first_shot_s = Some(t);
                    down_at_fire = lowered(&b);
                }
                Event::Killed { soldier, .. } => {
                    if soldier < SQUAD { r.killed.0 += 1 } else { r.killed.1 += 1 }
                }
                Event::Order { squad: 0, step: "regroup" } => r.regroups += 1,
                Event::Order { squad: 0, step } if step == "secure" || step == "abandon" => r.outcome = r.outcome.or(Some(step)),
                _ => {}
            }
        }
        r.ordered |= (0..SQUAD).any(|i| matches!(b.soldiers[i].task, Task::Move { .. }));
        if r.detect_s.is_none() && (0..SQUAD).any(|i| b.soldiers[i].target.is_some()) {
            r.detect_s = Some(t);
        }
        r.covered = r.covered.max((0..SQUAD).filter(|&i| b.soldiers[i].alive() && (b.soldiers[i].cover.is_some() || b.soldiers[i].stance != Stance::Stand)).count());
        if r.enemy_first_shot_s.is_some_and(|f| t <= f + 3.0) {
            r.went_down_after_fire = r.went_down_after_fire.max(lowered(&b).saturating_sub(down_at_fire));
        }
        r.suppressed_foes = r.suppressed_foes.max((SQUAD..SQUAD + foes).filter(|&i| b.soldiers[i].suppression > 0).count());
        if r.outcome.is_some() {
            break;
        }
    }
    let c = centroid(&b);
    r.moved_mm = c.1 - start.1;
    r
}

fn centroid(b: &Battle) -> (i64, i64) {
    let ms: Vec<_> = (0..SQUAD).filter(|&i| b.soldiers[i].alive()).collect();
    let n = ms.len().max(1) as i64;
    (ms.iter().map(|&i| b.soldiers[i].pos.0).sum::<i64>() / n, ms.iter().map(|&i| b.soldiers[i].pos.1).sum::<i64>() / n)
}

fn lowered(b: &Battle) -> usize {
    (0..SQUAD).filter(|&i| b.soldiers[i].alive() && b.soldiers[i].stance != Stance::Stand).count()
}

/// Against an even-sized section the squad goes through every step and either takes the position or, bled
/// too badly, abandons the attack: it never just stands about until the clock runs out.
#[test]
fn the_reference_squad_goes_through_all_ten_steps() {
    let r = run(7, 6, 180);
    eprintln!("{r:?}");
    assert!(r.ordered, "1 order: the men took the move");
    assert!(r.moved_mm >= 10_000, "2 move: the squad advanced {} mm", r.moved_mm);
    assert!(r.detect_s.is_some(), "3 detect: someone picked a target");
    assert!(r.covered >= 6, "4 own cover: {} men low or in cover", r.covered);
    assert!(r.shooters.len() >= 6, "5 coordinated fire: {} rifles fired", r.shooters.len());
    assert!(r.went_down_after_fire >= 3, "6 react to fire: {} got down within 3 s", r.went_down_after_fire);
    assert!(r.suppressed_foes >= 3, "7 suppression: {} foes under fire", r.suppressed_foes);
    assert!(r.killed.0 + r.killed.1 >= 1, "8 casualties: none");
    assert!(r.regroups >= 1 || r.killed.0 == 0, "9 regroup: lost {} men, never regrouped", r.killed.0);
    assert!(r.outcome.is_some(), "10 complete or abandon: neither within the time");
}

/// Facing far more than it can take, the squad abandons the attack once half of it is out of action, falling
/// back instead of being shot to the last man.
#[test]
fn an_outmatched_squad_abandons_the_attack() {
    let r = run(3, 14, 240);
    eprintln!("{r:?}");
    assert!(r.killed.0 < SQUAD, "the whole squad was lost");
    assert_eq!(r.outcome, Some("abandon"), "the attack was not called off");
}
