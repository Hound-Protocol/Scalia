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

/// The plain way: sample every 100 mm (skipping 200 mm at each end) and test the band at each sample.
fn clear_slow(g: &SightGrid, a: Point, b: Point) -> bool {
    let (a, b) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) { (a, b) } else { (b, a) };
    let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
    let len = scalia_ai::sight::isqrt(dx * dx + dy * dy);
    let n = (len / 100).max(1);
    (0..=n).all(|k| {
        let along = len * k / n;
        if along < 200 || len - along < 200 {
            return true;
        }
        let (x, y, z) = (a.x + dx * k / n, a.y + dy * k / n, a.z + dz * k / n);
        let band = z / 100;
        z >= 0 && !(band < 32 && g.bands_at(x, y) & (1u32 << band) != 0)
    })
}

#[test]
fn skipping_empty_ground_never_changes_what_is_seen() {
    use scalia_core::hash::mix64;
    let house = scalia_core::killhouse::Killhouse::new(30_000, 30_000);
    let mut props = house.props();
    for k in 0..60u64 {
        let r = mix64(k);
        props.push(scalia_core::nav::Prop { kind: scalia_core::nav::PropKind::Wall { length_mm: 3_000 + (r % 5_000) as i64, thickness_mm: 600 }, x_mm: (r >> 16) as i64 % 100_000, y_mm: (r >> 36) as i64 % 80_000, quarter_turns: (r >> 60) as u8 % 2 });
    }
    let g = SightGrid::build(0, 0, 400, 320, &props);
    let mut blocked = 0;
    for k in 0..20_000u64 {
        let r = mix64(k ^ 0xbeef);
        let r2 = mix64(r);
        // Points from 10 m outside the grid to inside it, eye or body heights.
        let a = Point { x: (r % 120_000) as i64 - 10_000, y: ((r >> 20) % 100_000) as i64 - 10_000, z: 300 + ((r >> 40) % 1_500) as i64 };
        let b = Point { x: (r2 % 120_000) as i64 - 10_000, y: ((r2 >> 20) % 100_000) as i64 - 10_000, z: 300 + ((r2 >> 40) % 1_500) as i64 };
        let fast = g.clear(a, b);
        assert_eq!(fast, clear_slow(&g, a, b), "ray {a:?} -> {b:?}");
        blocked += !fast as u32;
    }
    assert!(blocked > 2_000, "the test map blocks a good share of rays ({blocked})");
}
