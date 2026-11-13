use scalia_ai::sight::{Point, SightGrid};
use scalia_core::nav::{Prop, PropKind};

fn grid(props: &[Prop]) -> SightGrid {
    SightGrid::build(0, 0, 160, 160, props) // 40 x 40 m of 250 mm cells
}
fn panel(x: i64, y: i64, window: bool) -> Prop {
    Prop { kind: PropKind::Panel { length_mm: 4_000, window }, x_mm: x, y_mm: y, quarter_turns: 0 }
}
fn p(x: i64, y: i64, z: i64) -> Point {
    Point { x, y, z }
}

#[test]
fn open_ground_is_clear() {
    let g = grid(&[]);
    assert!(g.clear(p(5_000, 5_000, 1_600), p(35_000, 30_000, 1_000)));
}

#[test]
fn a_plywood_panel_blocks_sight_at_every_height_it_covers() {
    let g = grid(&[panel(20_000, 20_000, false)]);
    assert!(!g.clear(p(20_000, 15_000, 1_600), p(20_000, 25_000, 1_600)));
    assert!(!g.clear(p(20_000, 15_000, 300), p(20_000, 25_000, 300)));
    assert!(g.clear(p(20_000, 15_000, 3_000), p(20_000, 25_000, 3_000)), "over the 2.4 m top");
}

#[test]
fn a_window_lets_sight_through_between_sill_and_header() {
    let g = grid(&[panel(20_000, 20_000, true)]);
    assert!(g.clear(p(20_000, 15_000, 1_500), p(20_000, 25_000, 1_500)), "through the opening");
    assert!(!g.clear(p(20_000, 15_000, 600), p(20_000, 25_000, 600)), "below the sill");
    assert!(!g.clear(p(20_000, 15_000, 2_200), p(20_000, 25_000, 2_200)), "the header");
}

#[test]
fn sandbags_hide_a_crouching_man_but_not_a_standing_one() {
    let bags = Prop { kind: PropKind::Wall { length_mm: 6_000, thickness_mm: 700 }, x_mm: 20_000, y_mm: 20_000, quarter_turns: 0 };
    let g = grid(&[bags]);
    let eye = p(20_000, 10_000, 1_600);
    assert!(!g.clear(eye, p(20_000, 20_800, 700)), "crouched behind the bags");
    assert!(g.clear(eye, p(20_000, 20_800, 1_500)), "standing behind them");
}

#[test]
fn sight_is_symmetric() {
    let g = grid(&[panel(20_000, 20_000, true)]);
    for (a, b) in [(p(18_000, 15_000, 1_500), p(22_000, 26_000, 1_400)), (p(3_000, 3_000, 1_600), p(30_000, 33_000, 900))] {
        assert_eq!(g.clear(a, b), g.clear(b, a));
    }
}
