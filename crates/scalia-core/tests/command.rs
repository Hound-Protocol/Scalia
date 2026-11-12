use scalia_core::command::{captain_post, step_orders, Command, CommandError, Kind, Stance, Step, Task};
use scalia_core::groups::GroupId;

const CAPT: GroupId = GroupId(8);

fn chain() -> Command {
    let mut c = Command::default();
    c.add_captain(CAPT);
    c
}

#[test]
fn units_answer_to_the_commander_until_attached() {
    let mut c = chain();
    assert_eq!(c.superior(GroupId(0)), None);
    c.attach(GroupId(0), CAPT).unwrap();
    c.attach(GroupId(2), CAPT).unwrap();
    assert_eq!(c.superior(GroupId(0)), Some(CAPT));
    assert_eq!(c.squads_of(CAPT), vec![GroupId(0), GroupId(2)]);
    c.detach(GroupId(0));
    assert_eq!(c.superior(GroupId(0)), None);
    assert_eq!(c.squads_of(CAPT), vec![GroupId(2)]);
}

#[test]
fn only_captains_command_and_captains_are_not_attached() {
    let mut c = chain();
    assert_eq!(c.attach(GroupId(0), GroupId(1)), Err(CommandError::NotACaptain));
    assert_eq!(c.attach(CAPT, CAPT), Err(CommandError::CaptainsAnswerToYou));
    let other = GroupId(9);
    c.add_captain(other);
    assert_eq!(c.attach(other, CAPT), Err(CommandError::CaptainsAnswerToYou));
}

#[test]
fn reattaching_moves_a_squad_to_its_new_captain() {
    let mut c = chain();
    let other = GroupId(9);
    c.add_captain(other);
    c.attach(GroupId(1), CAPT).unwrap();
    c.attach(GroupId(1), other).unwrap();
    assert!(c.squads_of(CAPT).is_empty());
    assert_eq!(c.squads_of(other), vec![GroupId(1)]);
}

#[test]
fn take_with_no_enemy_known_approaches_then_advances() {
    let t = Task::new(Kind::Take, (100_000, 0), false).unwrap();
    assert_eq!(t.plan, vec![Step::Approach, Step::Advance]);
}

#[test]
fn take_with_a_known_enemy_suppresses_before_the_assault() {
    let t = Task::new(Kind::Take, (100_000, 0), true).unwrap();
    assert_eq!(t.plan, vec![Step::Approach, Step::Suppress, Step::Assault]);
}

#[test]
fn hold_occupies_and_digs_in_or_fights_its_way_there() {
    assert_eq!(Task::new(Kind::Hold, (0, 0), false).unwrap().plan, vec![Step::Occupy, Step::DigIn]);
    assert_eq!(Task::new(Kind::Hold, (0, 0), true).unwrap().plan, vec![Step::Approach, Step::Suppress, Step::Assault, Step::DigIn]);
}

#[test]
fn a_task_walks_its_plan_and_replans_when_the_enemy_shows() {
    let mut t = Task::new(Kind::Take, (100_000, 0), false).unwrap();
    assert_eq!(t.current(), Some(Step::Approach));
    t.advance();
    // Contact at the line of departure: the rest of the plan changes, what is done stays done.
    assert!(t.replan(true));
    assert_eq!(t.current(), Some(Step::Suppress));
    t.advance();
    assert_eq!(t.current(), Some(Step::Assault));
    t.advance();
    assert_eq!(t.current(), None);
    assert!(t.done());
}

#[test]
fn approach_stops_short_of_the_objective_with_squads_spread_across_the_line() {
    let squads = [(GroupId(0), (0, -5_000)), (GroupId(1), (0, 5_000))];
    let orders = step_orders(Step::Approach, (100_000, 0), &squads);
    assert_eq!(orders.len(), 2);
    for o in &orders {
        // 40 m short of the objective, coming from the west.
        assert_eq!(o.to.0, 60_000, "{o:?}");
        assert!(!o.run);
    }
    assert_ne!(orders[0].to.1, orders[1].to.1, "squads spread across the line");
    assert!((orders[0].to.1 - orders[1].to.1).abs() >= 10_000);
}

#[test]
fn the_first_squad_suppresses_while_the_rest_assault() {
    let squads = [(GroupId(0), (60_000, -6_000)), (GroupId(1), (60_000, 6_000)), (GroupId(2), (60_000, 18_000))];
    let sup = step_orders(Step::Suppress, (100_000, 0), &squads);
    assert_eq!(sup[0].stance, Stance::CoverAim);
    assert!(sup[1..].iter().all(|o| o.stance == Stance::Crouch));
    let ass = step_orders(Step::Assault, (100_000, 0), &squads);
    assert_eq!(ass.len(), 2, "the support squad stays put: {ass:?}");
    assert!(ass.iter().all(|o| o.run && o.stance == Stance::StandAim));
    for o in &ass {
        let d = (((o.to.0 - 100_000).pow(2) + o.to.1.pow(2)) as f64).sqrt();
        assert!(d <= 10_000.0, "assault ends on the objective: {o:?}");
    }
}

#[test]
fn dig_in_holds_in_cover_without_moving() {
    let squads = [(GroupId(0), (100_000, 0))];
    let o = step_orders(Step::DigIn, (100_000, 0), &squads);
    assert_eq!(o[0].to, (100_000, 0));
    assert_eq!(o[0].stance, Stance::Cover);
}

#[test]
fn the_captain_holds_back_behind_his_squads() {
    let targets = [(60_000, -6_000), (60_000, 6_000)];
    let p = captain_post((100_000, 0), &targets);
    assert_eq!(p, (45_000, 0));
}

#[test]
fn orders_are_the_same_every_time() {
    let squads = [(GroupId(3), (5_000, 7_000)), (GroupId(1), (-3_000, 2_000)), (GroupId(2), (9_000, -4_000))];
    let a = step_orders(Step::Approach, (80_000, 30_000), &squads);
    for _ in 0..10 {
        assert_eq!(step_orders(Step::Approach, (80_000, 30_000), &squads), a);
    }
}
