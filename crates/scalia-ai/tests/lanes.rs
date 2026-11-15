use scalia_ai::battle::{Battle, Soldier, Stance, Task};
use scalia_ai::sight::SightGrid;

fn man(team: u8, squad: u16, x: i64, y: i64, facing: (i64, i64), stance: Stance) -> Soldier {
    let mut s = Soldier::new(team, squad, (x, y), stance);
    s.facing = facing;
    s
}

/// A man holding his ground with a friend in his line of fire who will not get down (he is lying behind him,
/// so the friend kneeling would not clear it): he shifts a pace sideways to clear the lane instead of only waiting.
#[test]
fn a_blocked_shooter_steps_aside_to_clear_his_lane() {
    let shooter = man(0, 0, 50_000, 50_000, (0, 1_000), Stance::Prone);
    let friend = man(0, 0, 50_000, 54_000, (0, 1_000), Stance::Prone);
    let foe = man(1, 1, 50_000, 75_000, (0, -1_000), Stance::Stand);
    let mut b = Battle::new(SightGrid::build(0, 0, 400, 400, &[]), vec![shooter, friend, foe], 1);
    b.soldiers[0].task = Task::Hold { at: (50_000, 50_000), face: (0, 1_000) };
    b.soldiers[1].task = Task::Hold { at: (50_000, 54_000), face: (0, 1_000) };
    b.soldiers[2].passive = true;
    for _ in 0..120 {
        b.step(50);
    }
    let dx = (b.soldiers[0].pos.0 - 50_000).abs();
    assert!(dx >= 600, "he shifted {dx} mm sideways");
}

/// A squad walking in file towards a point who learns of the enemy ahead shakes out into a line across the
/// enemy's bearing, so more rifles can bear (instead of carrying on in single file).
#[test]
fn a_squad_on_the_move_shakes_out_into_a_line_on_contact() {
    use scalia_ai::leader::{Mode, Order};
    use scalia_ai::save::Command;
    // Eight men in single file (south to north), the enemy 45 m ahead.
    let mut men: Vec<Soldier> = (0..8).map(|k| man(0, 0, 50_000, 40_000 - k * 1_500, (0, 1_000), Stance::Stand)).collect();
    men.push(man(1, 1, 50_000, 90_000, (0, -1_000), Stance::Stand));
    let mut b = Battle::new(SightGrid::build(0, 0, 400, 400, &[]), men, 3);
    b.soldiers[8].passive = true;
    b.submit(0, Command::Squad { squad: 0, order: Order::Move { to: (50_000, 70_000), mode: Mode::Plain } });
    for _ in 0..200 {
        b.step(50);
    }
    let xs: Vec<i64> = (0..8).filter(|&k| b.soldiers[k].alive()).map(|k| b.soldiers[k].pos.0).collect();
    let spread = xs.iter().max().unwrap() - xs.iter().min().unwrap();
    assert!(spread >= 8_000, "the squad spans {spread} mm across the enemy's bearing");
}
