use scalia_core::nav::{NavGrid, Prop, PropKind, CELL_MM};

/// A 40 m x 40 m grid with its corner at the origin.
fn grid() -> NavGrid {
    NavGrid::new(0, 0, 80, 80)
}

#[test]
fn empty_ground_is_open_everywhere() {
    let mut g = grid();
    g.rebuild(&[]);
    assert!(g.walkable_at(20_000, 20_000, 2_000));
    assert!(g.clearance_mm_at(20_000, 20_000) >= 10_000);
}

#[test]
fn a_rock_blocks_its_footprint_and_clearance_grows_with_distance() {
    let mut g = grid();
    g.rebuild(&[Prop { kind: PropKind::Rock { radius_mm: 1_000 }, x_mm: 20_000, y_mm: 20_000, quarter_turns: 0 }]);
    assert!(!g.walkable_at(20_000, 20_000, 0), "inside the rock");
    assert!(!g.walkable_at(20_800, 20_000, 0), "inside the rock");
    let c = g.clearance_mm_at(21_500, 20_000);
    assert!((250..=1_000).contains(&c), "1.5 m from the centre, 0.5 m from the edge: {c}");
    let far = g.clearance_mm_at(25_000, 20_000);
    assert!(far > c && (3_500..=4_600).contains(&far), "{far}");
}

#[test]
fn a_quarter_turned_wall_runs_north_south() {
    let mut g = grid();
    // A 10 m wall, 0.4 m thick; unturned it runs east-west, one quarter turn makes it run north-south.
    g.rebuild(&[Prop { kind: PropKind::Wall { length_mm: 10_000, thickness_mm: 400 }, x_mm: 20_000, y_mm: 20_000, quarter_turns: 1 }]);
    assert!(!g.walkable_at(20_000, 24_000, 0));
    assert!(g.walkable_at(24_000, 20_000, 0));
}

#[test]
fn a_gap_lets_soldiers_through_but_not_tanks() {
    let mut g = grid();
    // Two walls along y = 20 m leaving a 3 m gap between x = 18.5 m and 21.5 m.
    let wall = |x| Prop { kind: PropKind::Wall { length_mm: 15_000, thickness_mm: 500 }, x_mm: x, y_mm: 20_000, quarter_turns: 0 };
    g.rebuild(&[wall(18_500 - 7_500), wall(21_500 + 7_500)]);
    let soldier_mm = 300;
    let tank_mm = 2_000;
    assert!(g.walkable_at(20_000, 20_000, soldier_mm), "soldier fits through the gap");
    assert!(!g.walkable_at(20_000, 20_000, tank_mm), "tank does not");
}

#[test]
fn the_grid_edge_counts_as_an_obstacle_and_outside_is_blocked() {
    let mut g = grid();
    g.rebuild(&[]);
    assert!(!g.walkable_at(-1, 5_000, 0));
    assert!(g.clearance_mm_at(CELL_MM / 2, 20_000) <= CELL_MM);
}

#[test]
fn rebuilding_is_deterministic() {
    let props = [
        Prop { kind: PropKind::Rock { radius_mm: 1_700 }, x_mm: 9_000, y_mm: 31_000, quarter_turns: 0 },
        Prop { kind: PropKind::Crate { size_mm: 1_200 }, x_mm: 30_000, y_mm: 12_000, quarter_turns: 3 },
    ];
    let (mut a, mut b) = (grid(), grid());
    a.rebuild(&props);
    b.rebuild(&props);
    assert_eq!(a.fingerprint(), b.fingerprint());
}

#[test]
fn plywood_panels_block_walking_and_a_doorway_between_them_is_open() {
    let mut g = grid();
    // Two 3 m panels along y = 20 m with a 1.2 m doorway between x = 19.4 and 20.6 m; one has a window.
    let panel = |x, window| Prop { kind: PropKind::Panel { length_mm: 3_000, window }, x_mm: x, y_mm: 20_000, quarter_turns: 0 };
    g.rebuild(&[panel(17_900, false), panel(22_100, true)]);
    assert!(!g.walkable_at(17_000, 20_000, 0), "solid panel");
    assert!(!g.walkable_at(22_500, 20_000, 0), "a window panel still blocks walking");
    assert!(g.walkable_at(20_000, 20_000, 0), "the doorway");
    assert_eq!(PropKind::PANEL_MM, 200);
}
