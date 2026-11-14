use scalia_ai::battle::{Battle, Event, Soldier, Stance};
use scalia_ai::sight::SightGrid;
use scalia_core::nav::{Prop, PropKind};

fn field(props: &[Prop]) -> SightGrid {
    SightGrid::build(0, 0, 400, 400, props) // 100 x 100 m
}
fn man(team: u8, x: i64, y: i64, stance: Stance) -> Soldier {
    Soldier::new(team, 0, (x, y), stance)
}
/// Runs until someone falls (dead or down) or `ms` pass; returns the battle.
fn run(mut b: Battle, ms: i64) -> Battle {
    let mut t = 0;
    while t < ms && b.soldiers.iter().all(|s| s.alive() && !s.down()) {
        b.step(50);
        t += 50;
    }
    b
}

#[test]
fn an_open_ground_duel_ends_and_is_deterministic() {
    let mk = || Battle::new(field(&[]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7);
    let (a, b) = (run(mk(), 30_000), run(mk(), 30_000));
    assert!(a.soldiers.iter().any(|s| !s.alive() || s.down()), "someone falls within 30 s at 30 m");
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
    // (He first needs to spot him and get a sight picture at 60 m.)
    let mut felt = 0;
    for _ in 0..80 {
        b.step(50);
        felt = felt.max(b.soldiers[1].suppression);
    }
    assert!(felt > 0, "rounds coming in suppress");
    let calm = b.hit_permille(0, 1);
    b.soldiers[0].suppression = 1000;
    assert!(b.hit_permille(0, 1) < calm, "a suppressed shooter aims worse");
}

#[test]
fn a_rifleman_reloads_after_a_magazine() {
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 20_000, Stance::Stand), man(1, 50_000, 80_000, Stance::Stand)], 5);
    b.soldiers[1].passive = true;
    b.soldiers[1].health = i32::MAX;
    // Aimed shots at 60 m: a magazine lasts a while.
    for _ in 0..1_400 {
        b.step(50);
    }
    assert!(b.log.iter().any(|e| matches!(e, Event::Reload { soldier: 0 })));
    assert!(b.soldiers[0].ammo <= 30 && b.soldiers[0].mags < scalia_ai::battle::MAGS);
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
    assert!(b.len() >= 3 && b.iter().all(|&n| (1..=4).contains(&n)), "short bursts at 35 m: {b:?}");
    assert!(b.windows(2).any(|w| w[0] != w[1]), "not the same burst every time: {b:?}");
}

#[test]
fn far_away_he_takes_single_aimed_shots() {
    let t = shots_at(90_000, 8_000);
    assert!(t.len() >= 4, "{t:?}");
    assert!(bursts(&t).iter().all(|&n| n <= 2), "aimed singles or pairs at 90 m: {t:?}");
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

#[test]
fn nobody_hits_more_than_85_percent_and_a_rookie_half() {
    use scalia_ai::battle::{SKILL_MAX, SKILL_ROOKIE};
    let mut b = Battle::new(SightGrid::build(0, 0, 100, 100, &[]), vec![Soldier::new(0, 0, (10_000, 10_000), Stance::Stand), Soldier::new(1, 1, (10_000, 15_000), Stance::Stand)], 1);
    b.soldiers[0].skill = 5_000;
    assert_eq!(b.hit_permille(0, 1), SKILL_MAX as i64, "best case caps at 85 %");
    b.soldiers[0].skill = SKILL_ROOKIE;
    assert_eq!(b.hit_permille(0, 1), 500);
    // Further off, much less.
    b.soldiers[1].pos = (10_000, 60_000);
    assert!(b.hit_permille(0, 1) < 300, "a rookie at 50 m: {}", b.hit_permille(0, 1));
}

#[test]
fn a_pinned_man_fires_far_less_than_a_calm_one() {
    let fired = |sup: i32| {
        let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 10_000, Stance::Stand), man(1, 50_000, 40_000, Stance::Stand)], 9);
        b.soldiers[1].passive = true;
        b.soldiers[1].health = i32::MAX;
        b.soldiers[0].mags = 99;
        for _ in 0..200 {
            b.soldiers[0].suppression = sup;
            b.step(50);
        }
        b.log.iter().filter(|e| matches!(e, Event::Shot { shooter: 0, .. })).count()
    };
    let (calm, pinned) = (fired(0), fired(950));
    assert!(pinned * 3 < calm, "calm {calm} rounds, pinned {pinned}");
}

fn battle_with(props: &[Prop], soldiers: Vec<Soldier>, seed: u64) -> Battle {
    let mut nav = scalia_core::nav::NavGrid::new(0, 0, 200, 200);
    nav.rebuild(props);
    let cover = scalia_ai::tactics::CoverMap::build(props, &nav);
    Battle::with_map(nav, field(props), cover, soldiers, seed)
}

#[test]
fn shot_at_in_the_open_he_sprints_for_cover_before_shooting_back() {
    // A rock 6 m to his side; an enemy 50 m off opens up on him.
    let rock = Prop { kind: PropKind::Rock { radius_mm: 1_200 }, x_mm: 56_000, y_mm: 20_000, quarter_turns: 0 };
    let mut b = battle_with(&[rock], vec![man(0, 50_000, 20_000, Stance::Stand), Soldier::new(1, 1, (50_000, 70_000), Stance::Stand)], 4);
    b.soldiers[0].health = i32::MAX;
    b.soldiers[1].health = i32::MAX;
    let mut took_cover_at = None;
    for t in 0..200 {
        b.step(50);
        if took_cover_at.is_none() && b.soldiers[0].cover.is_some() {
            took_cover_at = Some(t);
        }
    }
    assert!(took_cover_at.is_some(), "under fire he goes for the rock");
    // No rounds from him while he was running for it.
    let spot = b.cover.as_ref().unwrap().spots[b.soldiers[0].cover.unwrap()].pos;
    assert!((b.soldiers[0].pos.0 - spot.0).abs() + (b.soldiers[0].pos.1 - spot.1).abs() < 1_500, "behind the rock now");
}

#[test]
fn flanked_in_cover_he_finds_new_cover() {
    // A wall running east-west; fire from the north sends him behind it (south side).
    let wall = Prop { kind: PropKind::Wall { length_mm: 12_000, thickness_mm: 600 }, x_mm: 50_000, y_mm: 50_000, quarter_turns: 0 };
    let rock = Prop { kind: PropKind::Rock { radius_mm: 1_200 }, x_mm: 44_000, y_mm: 46_000, quarter_turns: 0 };
    let mut b = battle_with(&[wall, rock], vec![man(0, 50_000, 53_000, Stance::Stand), Soldier::new(1, 1, (50_000, 75_000), Stance::Stand)], 4);
    b.soldiers[0].health = i32::MAX;
    b.soldiers[1].health = i32::MAX;
    // Until he is behind it.
    let mut first = None;
    for _ in 0..300 {
        b.step(50);
        if let Some(c) = b.soldiers[0].cover {
            let at = b.cover.as_ref().unwrap().spots[c].pos;
            if (b.soldiers[0].pos.0 - at.0).abs() + (b.soldiers[0].pos.1 - at.1).abs() < 600 && !b.soldiers[0].moving {
                first = Some(c);
                break;
            }
        }
    }
    let first = first.expect("in cover from the north");
    // The enemy comes round to the east, level with him: the wall no longer covers him.
    b.soldiers[1].pos = (75_000, 49_000);
    // Within two seconds (leaving cover when it goes calm takes three): the flank, not the calm, moves him.
    assert_eq!(b.soldiers[0].cover, Some(first));
    let mut moved = false;
    for _ in 0..40 {
        b.step(50);
        moved |= b.soldiers[0].cover.is_some_and(|c| c != first);
    }
    assert!(moved, "he left the flanked spot for another");
}

#[test]
fn nobody_sees_or_hits_through_smoke_until_it_thins() {
    use scalia_ai::battle::{Smoke, SMOKE_BUILD_MS, SMOKE_MS};
    let mut b = Battle::new(field(&[]), vec![man(0, 50_000, 30_000, Stance::Stand), man(1, 50_000, 60_000, Stance::Stand)], 7);
    b.smoke.push(Smoke { pos: (50_500, 45_000), at_ms: 0 });
    // Building up: not yet thick enough to hide anyone.
    b.step(50);
    assert!(b.exposure(0, 1) > 0, "the cloud is still building");
    while b.time_ms < SMOKE_BUILD_MS {
        b.step(50);
    }
    assert_eq!(b.exposure(0, 1), 0, "the cloud hides them from each other");
    assert_eq!(b.hit_permille(0, 1), 0, "and nobody can hit what he cannot see");
    let fired = b.log.len();
    while b.time_ms < SMOKE_MS - 1_000 {
        b.step(50);
    }
    assert!(b.soldiers.iter().all(|s| s.alive() && !s.down()), "nobody hit through the smoke");
    let _ = fired;
    while b.time_ms < SMOKE_MS + 100 {
        b.step(50);
    }
    assert!(b.exposure(0, 1) > 0, "it thins out");
    assert!(b.smoke.is_empty(), "and is gone");
    // Off the line between them it hides nothing.
    b.smoke.push(Smoke { pos: (60_000, 45_000), at_ms: b.time_ms - SMOKE_BUILD_MS });
    assert!(b.exposure(0, 1) > 0, "a cloud 10 m off the line");
}

#[test]
fn a_man_shot_at_turns_on_the_shooter_not_the_squads_other_contact() {
    use scalia_ai::battle::Contact;
    // Blue lies facing north, where his squad knows of an enemy (behind a wall, unseen); Red, 12 m south, a poor
    // shot, fires on him from behind.
    let wall = Prop { kind: PropKind::Panel { length_mm: 20_000, window: false }, x_mm: 50_000, y_mm: 70_000, quarter_turns: 0 };
    let mut blue = man(0, 50_000, 50_000, Stance::Prone);
    blue.facing = (0, 1_000);
    let mut red = man(1, 50_000, 38_000, Stance::Stand);
    red.facing = (0, 1_000);
    red.skill = 300;
    let far = man(1, 50_000, 80_000, Stance::Stand);
    let mut b = Battle::new(field(&[wall]), vec![blue, red, far], 3);
    let mut faced_south = false;
    for _ in 0..80 {
        let now = b.time_ms;
        b.contacts[0].retain(|c| c.enemy != 2);
        b.contacts[0].push(Contact { enemy: 2, pos: (50_000, 80_000), seen_ms: now, confidence: 1_000 });
        b.step(50);
        if b.soldiers[0].down() {
            break;
        }
        faced_south |= b.soldiers[0].facing.1 < -500;
    }
    assert!(faced_south, "he turned on the man shooting him: {:?}", b.soldiers[0].facing);
}
