use scalia_core::nav::{find_path, NavGrid, Prop, PropKind};

const SOLDIER: i64 = 300;
const TANK: i64 = 2_000;

fn wall(x: i64, y: i64, len: i64, turns: u8) -> Prop {
    Prop { kind: PropKind::Wall { length_mm: len, thickness_mm: 500 }, x_mm: x, y_mm: y, quarter_turns: turns }
}

fn length(path: &[(i64, i64)]) -> f64 {
    path.windows(2).map(|w| (((w[1].0 - w[0].0) as f64).powi(2) + ((w[1].1 - w[0].1) as f64).powi(2)).sqrt()).sum()
}

#[test]
fn open_ground_is_a_straight_line() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    g.rebuild(&[]);
    let p = find_path(&g, (3_000, 3_000), (33_000, 3_000), SOLDIER).expect("a path");
    assert_eq!(p.first(), Some(&(3_000, 3_000)));
    assert_eq!(p.last(), Some(&(33_000, 3_000)));
    assert!(p.len() <= 3, "smoothed to a line: {p:?}");
    assert!((length(&p) - 30_000.0).abs() < 600.0);
}

#[test]
fn a_wall_is_walked_round_through_its_gap() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    // A wall across the middle (y = 20 m) from x = 0 to 36 m, open from 36 m to 40 m.
    g.rebuild(&[wall(18_000, 20_000, 36_000, 0)]);
    let p = find_path(&g, (5_000, 5_000), (5_000, 35_000), SOLDIER).expect("a path");
    // Somewhere it crosses y = 20 m, and that crossing is in the gap.
    let crossing = p.windows(2).find(|w| (w[0].1 - 20_000).signum() != (w[1].1 - 20_000).signum()).expect("crosses");
    let x = crossing[0].0 + (crossing[1].0 - crossing[0].0) * (20_000 - crossing[0].1) / (crossing[1].1 - crossing[0].1);
    assert!(x > 36_000, "goes through the gap, not the wall: crosses at x {x}");
    // Every straight leg stays clear of the wall.
    for w in p.windows(2) {
        for k in 0..=50 {
            let (x, y) = (w[0].0 + (w[1].0 - w[0].0) * k / 50, w[0].1 + (w[1].1 - w[0].1) * k / 50);
            assert!(g.walkable_at(x, y, SOLDIER), "leg {w:?} cuts through blocked ground at ({x}, {y})");
        }
    }
}

#[test]
fn a_tank_does_not_fit_where_a_soldier_does() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    // Walls across y = 20 m leaving only a 2 m gap at x = 19-21 m.
    g.rebuild(&[wall(9_500, 20_000, 19_000, 0), wall(30_500, 20_000, 19_000, 0)]);
    assert!(find_path(&g, (20_000, 5_000), (20_000, 35_000), SOLDIER).is_some());
    assert!(find_path(&g, (20_000, 5_000), (20_000, 35_000), TANK).is_none());
}

#[test]
fn unreachable_or_blocked_ends_give_no_path() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    g.rebuild(&[Prop { kind: PropKind::Rock { radius_mm: 3_000 }, x_mm: 20_000, y_mm: 20_000, quarter_turns: 0 }]);
    assert!(find_path(&g, (2_000, 2_000), (20_000, 20_000), SOLDIER).is_none(), "goal inside a rock");
    assert!(find_path(&g, (2_000, 2_000), (90_000, 2_000), SOLDIER).is_none(), "goal off the grid");
}

#[test]
fn paths_are_deterministic() {
    let mut g = NavGrid::new(0, 0, 80, 80);
    g.rebuild(&[wall(20_000, 20_000, 20_000, 1), Prop { kind: PropKind::Rock { radius_mm: 2_000 }, x_mm: 10_000, y_mm: 30_000, quarter_turns: 0 }]);
    let a = find_path(&g, (2_000, 2_000), (37_000, 37_000), SOLDIER);
    let b = find_path(&g, (2_000, 2_000), (37_000, 37_000), SOLDIER);
    assert!(a.is_some());
    assert_eq!(a, b);
}
