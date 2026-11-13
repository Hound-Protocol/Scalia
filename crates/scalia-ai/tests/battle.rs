use scalia_ai::battle::{Battle, Event, Soldier, Stance};
use scalia_ai::sight::SightGrid;
use scalia_core::nav::{Prop, PropKind};

fn field(props: &[Prop]) -> SightGrid {
    SightGrid::build(0, 0, 400, 400, props) // 100 x 100 m
}
fn man(team: u8, x: i64, y: i64, stance: Stance) -> Soldier {
    Soldier::new(team, 0, (x, y), stance)
}
/// Runs until someone dies or `ms` pass; returns the battle.
fn run(mut b: Battle, ms: i64) -> Battle {
    let mut t = 0;
    while t < ms && b.soldiers.iter().all(|s| s.alive()) {
        b.step(50);
        t += 50;
    }
    b
}

#[test]
fn an_open_ground_duel_ends_and_is_deterministic() {
    let mk = || Battle::new(field(&[]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7);
    let (a, b) = (run(mk(), 30_000), run(mk(), 30_000));
    assert!(a.soldiers.iter().any(|s| !s.alive()), "someone falls within 30 s at 30 m");
    assert_eq!(a.log, b.log, "same seed, same fight");
}

#[test]
fn nobody_shoots_through_a_wall() {
    let wall = Prop { kind: PropKind::Panel { length_mm: 20_000, window: false }, x_mm: 50_000, y_mm: 45_000, quarter_turns: 0 };
    let b = run(Battle::new(field(&[wall]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7), 10_000);
    assert!(b.log.iter().all(|e| !matches!(e, Event::Shot { .. })), "no line of sight, no shots");
}

/// Hits landed on an unkillable target over 60 s of fire.
fn hits_on(target: Soldier, props: &[Prop]) -> usize {
    let mut b = Battle::new(field(props), vec![man(0, 50_000, 20_000, Stance::Stand), target], 11);
    b.soldiers[1].health = i32::MAX;
    b.soldiers[1].passive = true;
    b.soldiers[0].mags = 99;
    for _ in 0..1200 {
        b.step(50);
    }
    b.log.iter().filter(|e| matches!(e, Event::Shot { hit: true, .. })).count()
}

#[test]
fn cover_and_stance_make_a_man_harder_to_hit() {
    let bags = Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 700 }, x_mm: 50_000, y_mm: 59_000, quarter_turns: 0 };
    let open = hits_on(man(1, 50_000, 60_000, Stance::Stand), &[]);
    let behind_bags = hits_on(man(1, 50_000, 60_000, Stance::Stand), &[bags]);
    let crouched_behind = hits_on(man(1, 50_000, 60_000, Stance::Crouch), &[bags]);
    assert!(open > behind_bags, "legs behind the bags: {open} vs {behind_bags}");
    assert!(behind_bags > crouched_behind, "crouched behind the bags: {behind_bags} vs {crouched_behind}");
}

#[test]
fn being_shot_at_suppresses_and_suppression_spoils_aim() {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 20_000, Stance::Stand), man(1, 50_000, 80_000, Stance::Stand)], 3);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    for _ in 0..40 {
        b.step(50);
    }
    assert!(b.soldiers[1].suppression > 0, "rounds coming in suppress");
    let calm = b.hit_permille(0, 1);
    b.soldiers[0].suppression = 1000;
    assert!(b.hit_permille(0, 1) < calm, "a suppressed shooter aims worse");
}

#[test]
fn a_rifleman_reloads_after_a_magazine() {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 20_000, Stance::Stand), man(1, 50_000, 80_000, Stance::Stand)], 5);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    for _ in 0..400 {
        b.step(50);
    }
    assert!(b.log.iter().any(|e| matches!(e, Event::Reload { soldier: 0 })));
}

/// Shooter 0's shots at a target `d_mm` away (the target cannot die or fire): their times (ms).
fn shots_at(d_mm: i64, ms: i64) -> Vec<i64> {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 10_000, Stance::Stand), man(1, 50_000, 10_000 + d_mm, Stance::Stand)], 5);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    b.soldiers[0].mags = 99;
    for _ in 0..ms / 50 {
        b.step(50);
    }
    b.log.iter().filter_map(|e| match e { Event::Shot { shooter: 0, at_ms, .. } => Some(*at_ms), _ => None }).collect()
}

/// Groups shot times into bursts (a gap over 200 ms starts a new one); returns the burst sizes.
fn bursts(times: &[i64]) -> Vec<usize> {
    let mut out = vec![];
    for (k, &t) in times.iter().enumerate() {
        if k == 0 || t - times[k - 1] > 200 {
            out.push(1);
        } else {
            *out.last_mut().unwrap() += 1;
        }
    }
    out
}

#[test]
fn close_up_he_fires_full_auto_bursts() {
    let t = shots_at(8_000, 6_000);
    let b = bursts(&t);
    assert!(b.iter().filter(|&&n| n >= 4).count() >= 2, "bursts of 4+ at 8 m: {b:?}");
    let gaps: Vec<i64> = t.windows(2).map(|w| w[1] - w[0]).filter(|&g| g <= 200).collect();
    assert!(gaps.iter().all(|&g| (60..=90).contains(&g)), "about 800 rounds a minute inside a burst: {gaps:?}");
}

#[test]
fn at_mid_range_he_fires_short_controlled_bursts() {
    let b = bursts(&shots_at(35_000, 8_000));
    assert!(b.len() >= 3 && b.iter().all(|&n| (2..=3).contains(&n)), "bursts of 2-3 at 35 m: {b:?}");
}

#[test]
fn far_away_he_takes_single_aimed_shots() {
    let t = shots_at(90_000, 8_000);
    assert!(t.len() >= 5, "{t:?}");
    assert!(bursts(&t).iter().all(|&n| n == 1), "single shots at 90 m: {t:?}");
}

#[test]
fn recoil_makes_the_later_rounds_of_a_burst_less_accurate() {
    // Many close-range bursts at a target that cannot die: hits on first rounds vs later rounds.
    let mut first = (0, 0);
    let mut later = (0, 0);
    for seed in 0..40 {
        let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 10_000, Stance::Stand), man(1, 50_000, 22_000, Stance::Stand)], seed);
        b.soldiers[1].passive = true;
        b.soldiers[1].health = i32::MAX;
        b.soldiers[0].mags = 99;
        for _ in 0..60 {
            b.step(50);
        }
        let mut prev = -1_000;
        for e in &b.log {
            if let Event::Shot { shooter: 0, at_ms, hit, .. } = e {
                let slot = if at_ms - prev > 200 { &mut first } else { &mut later };
                slot.0 += *hit as i32;
                slot.1 += 1;
                prev = *at_ms;
            }
        }
    }
    let (f, l) = (first.0 * 1000 / first.1.max(1), later.0 * 1000 / later.1.max(1));
    assert!(f > l + 100, "first rounds hit clearly more often: first {f}‰ of {}, later {l}‰ of {}", first.1, later.1);
}
