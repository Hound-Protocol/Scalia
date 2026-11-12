use scalia_ai::goap::{plan, Action, Goal, Limits, State};

// Facts for a F.E.A.R.-style soldier.
const ARMED: u32 = 0;
const LOADED: u32 = 1;
const IN_COVER: u32 = 2;
const IN_RANGE: u32 = 3;
const ENEMY_DOWN: u32 = 4;
const SUPPRESSED: u32 = 5;

fn fact(f: u32) -> u64 {
    1 << f
}

fn soldier_actions() -> Vec<Action> {
    vec![
        Action::new("draw weapon", 1).sets(ARMED, true),
        Action::new("reload", 2).needs(ARMED, true).sets(LOADED, true),
        Action::new("move into range", 4).sets(IN_RANGE, true).sets(IN_COVER, false),
        Action::new("run to cover", 3).sets(IN_COVER, true),
        Action::new("shoot from cover", 2).needs(LOADED, true).needs(IN_COVER, true).needs(IN_RANGE, true).sets(ENEMY_DOWN, true),
        Action::new("shoot in the open", 1).needs(LOADED, true).needs(IN_RANGE, true).needs(SUPPRESSED, false).sets(ENEMY_DOWN, true),
    ]
}

fn names(actions: &[Action], p: &[usize]) -> Vec<&'static str> {
    p.iter().map(|&i| actions[i].name).collect()
}

#[test]
fn nothing_to_do_when_the_goal_holds() {
    let a = soldier_actions();
    let p = plan(State(fact(ENEMY_DOWN)), Goal::new().want(ENEMY_DOWN, true), &a, Limits::default()).unwrap();
    assert!(p.actions.is_empty());
    assert_eq!(p.cost, 0);
}

#[test]
fn finds_the_cheapest_ordered_plan() {
    let a = soldier_actions();
    let p = plan(State(0), Goal::new().want(ENEMY_DOWN, true), &a, Limits::default()).unwrap();
    assert_eq!(names(&a, &p.actions), ["draw weapon", "reload", "move into range", "shoot in the open"]);
    assert_eq!(p.cost, 1 + 2 + 4 + 1);
}

#[test]
fn under_fire_it_fights_from_cover_instead() {
    let a = soldier_actions();
    let p = plan(State(fact(SUPPRESSED) | fact(ARMED)), Goal::new().want(ENEMY_DOWN, true), &a, Limits::default()).unwrap();
    let n = names(&a, &p.actions);
    assert_eq!(*n.last().unwrap(), "shoot from cover");
    // Cover must come after moving into range, which breaks cover.
    let cover = n.iter().position(|&x| x == "run to cover").unwrap();
    let range = n.iter().position(|&x| x == "move into range").unwrap();
    assert!(range < cover, "{n:?}");
    assert_eq!(p.cost, 2 + 4 + 3 + 2);
}

#[test]
fn every_plan_is_valid_step_by_step() {
    let a = soldier_actions();
    for start in 0..64u64 {
        let goal = Goal::new().want(ENEMY_DOWN, true).want(IN_COVER, true);
        if let Some(p) = plan(State(start), goal, &a, Limits::default()) {
            let mut s = State(start);
            for &i in &p.actions {
                assert!(a[i].applicable(s), "start {start:b}: {} not applicable", a[i].name);
                s = a[i].apply(s);
            }
            assert!(goal.met(s), "start {start:b}");
        }
    }
}

#[test]
fn impossible_goals_and_tight_budgets_give_none() {
    let a = vec![Action::new("wait", 1).sets(0, true)];
    assert!(plan(State(0), Goal::new().want(1, true), &a, Limits::default()).is_none());
    let b = soldier_actions();
    assert!(plan(State(0), Goal::new().want(ENEMY_DOWN, true), &b, Limits { max_nodes: 2, max_depth: 8 }).is_none());
    assert!(plan(State(0), Goal::new().want(ENEMY_DOWN, true), &b, Limits { max_nodes: 10_000, max_depth: 2 }).is_none());
}

#[test]
fn deterministic_including_ties() {
    // Two equally cheap routes: the lower action index wins, every time.
    let a = vec![Action::new("left", 2).sets(0, true), Action::new("right", 2).sets(0, true)];
    for _ in 0..10 {
        let p = plan(State(0), Goal::new().want(0, true), &a, Limits::default()).unwrap();
        assert_eq!(names(&a, &p.actions), ["left"]);
    }
}

#[test]
fn the_example_squad_can_take_its_objective_from_any_start() {
    use scalia_ai::squad::{actions, take_objective};
    let a = actions();
    for start in 0..4096u64 {
        let p = plan(State(start), take_objective(), &a, Limits::default());
        let p = p.unwrap_or_else(|| panic!("no plan from {start:012b}"));
        let mut s = State(start);
        for &i in &p.actions {
            assert!(a[i].applicable(s));
            s = a[i].apply(s);
        }
        assert!(take_objective().met(s));
    }
}
