use scalia_core::goap::{plan, Action, Goal};

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 4;

fn act(id: u8, need: u32, need_not: u32, add: u32, remove: u32, cost: u32) -> Action {
    Action { id, need, need_not, add, remove, cost }
}

#[test]
fn chains_actions_to_reach_the_goal() {
    let acts = [act(0, 0, 0, A, 0, 1), act(1, A, 0, B, 0, 1), act(2, B, 0, C, 0, 1)];
    assert_eq!(plan(0, Goal { want: C, want_not: 0 }, &acts), Some(vec![0, 1, 2]));
}

#[test]
fn picks_the_cheapest_plan() {
    // Direct route costs 5; the two-step route costs 2.
    let acts = [act(0, 0, 0, C, 0, 5), act(1, 0, 0, A, 0, 1), act(2, A, 0, C, 0, 1)];
    assert_eq!(plan(0, Goal { want: C, want_not: 0 }, &acts), Some(vec![1, 2]));
}

#[test]
fn respects_negative_preconditions_and_goals() {
    // Action 0 only works while B is false; the goal wants C and not A.
    let acts = [act(0, 0, B, C | A, 0, 1), act(1, 0, 0, C, 0, 3), act(2, A, 0, 0, A, 1)];
    assert_eq!(plan(B, Goal { want: C, want_not: A }, &acts), Some(vec![1]));
    assert_eq!(plan(0, Goal { want: C, want_not: A }, &acts), Some(vec![0, 2]));
}

#[test]
fn an_unreachable_goal_has_no_plan() {
    let acts = [act(0, 0, 0, A, 0, 1)];
    assert_eq!(plan(0, Goal { want: B, want_not: 0 }, &acts), None);
}

#[test]
fn a_goal_already_met_needs_no_steps() {
    assert_eq!(plan(A, Goal { want: A, want_not: 0 }, &[]), Some(vec![]));
}

#[test]
fn equal_cost_ties_break_the_same_way_every_time() {
    let acts = [act(0, 0, 0, A, 0, 1), act(1, 0, 0, A, 0, 1)];
    let first = plan(0, Goal { want: A, want_not: 0 }, &acts);
    for _ in 0..20 {
        assert_eq!(plan(0, Goal { want: A, want_not: 0 }, &acts), first);
    }
    assert_eq!(first, Some(vec![0]));
}
