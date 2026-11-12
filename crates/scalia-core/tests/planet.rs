use scalia_core::hash::mix64;
use scalia_core::planet::{Planet, CELL_MM, TERRAIN_MAX_MM, TERRAIN_MIN_MM};

fn demo() -> Planet {
    Planet::demo(42)
}

#[test]
fn width_must_be_a_multiple_of_the_lattice() {
    assert!(Planet::new(1, 0).is_none());
    assert!(Planet::new(1, 33).is_none());
    let p = Planet::new(1, 64).unwrap();
    assert_eq!(p.width_mm(), 64_000_000);
    assert_eq!(p.depth_mm(), 32_000_000);
}

#[test]
fn same_inputs_same_height() {
    let p = demo();
    assert_eq!(
        p.height_mm(123_456, 789_012_000),
        p.height_mm(123_456, 789_012_000)
    );
}

#[test]
fn different_seed_differs_somewhere() {
    let grid = |p: &Planet| {
        (0..100i64)
            .map(|k| p.height_mm(k * 3_700_000, k * 4_100_000 + 1_000_000_000))
            .collect::<Vec<_>>()
    };
    assert_ne!(grid(&Planet::demo(1)), grid(&Planet::demo(2)));
}

#[test]
fn wraps_east_west_exactly() {
    let p = demo();
    let w = p.width_mm();
    for k in 0..200i64 {
        let (x, y) = (k * 977_131, p.depth_mm() / 3 + k * 1_299_709);
        let h = p.height_mm(x, y);
        assert_eq!(h, p.height_mm(x + w, y));
        assert_eq!(h, p.height_mm(x - w, y));
        assert_eq!(h, p.height_mm(x - 7 * w, y));
    }
}

#[test]
fn the_seam_is_smooth() {
    // Walking east across x = 0 (the date line) steps no more than an ordinary metre does.
    let p = demo();
    for k in 0..50i64 {
        let y = p.depth_mm() / 4 + k * 13_000_000;
        let (a, b) = (p.height_mm(p.width_mm() - CELL_MM, y), p.height_mm(0, y));
        assert!((a - b).abs() <= 450, "y {y}: {a} vs {b}");
    }
}

#[test]
fn heights_within_range_for_any_input() {
    let p = demo();
    let big = [
        i64::MIN,
        -1_000_000_000_000,
        -1,
        0,
        1,
        1_000_000_000_000,
        i64::MAX,
    ];
    for &x in &big {
        for &y in &big {
            let h = p.height_mm(x, y);
            assert!(
                (TERRAIN_MIN_MM..=TERRAIN_MAX_MM).contains(&h),
                "{x},{y} -> {h}"
            );
        }
    }
}

#[test]
fn beyond_the_poles_clamps() {
    let p = demo();
    assert_eq!(p.height_mm(5_000, -9_000_000), p.height_mm(5_000, 0));
    assert_eq!(
        p.height_mm(5_000, p.depth_mm() + 9_000_000),
        p.height_mm(5_000, p.depth_mm())
    );
}

#[test]
fn polar_ice_at_both_edges_only() {
    let p = demo();
    let d = p.depth_mm();
    assert!(p.is_ice(0) && p.is_ice(d / 20) && p.is_ice(d - d / 20) && p.is_ice(d));
    assert!(!p.is_ice(d / 2) && !p.is_ice(d / 5) && !p.is_ice(d - d / 5));
}

#[test]
fn local_relief_visible_at_soldier_scale() {
    let p = demo();
    let mid = p.depth_mm() / 2 / (64 * CELL_MM);
    for (cx, cy) in (0..20i64)
        .map(|k| (k * 37 - 300, mid + k * 11 - 90))
        .chain([(0, mid), (-1, mid)])
    {
        let c = p.chunk(cx, cy, 64);
        let (lo, hi) = (
            *c.heights.iter().min().unwrap(),
            *c.heights.iter().max().unwrap(),
        );
        assert!(hi - lo >= 1_500, "chunk ({cx},{cy}): relief {} mm", hi - lo);
    }
}

#[test]
fn slopes_are_walkable() {
    let p = Planet::demo(7);
    let mid = p.depth_mm() / 2 / (64 * CELL_MM);
    for k in 0..20i64 {
        let c = p.chunk(k * 53 - 500, mid + 30 - k * 3, 64);
        let n = 65usize;
        for j in 0..n {
            for i in 0..n - 1 {
                let d = (c.heights[j * n + i + 1] - c.heights[j * n + i]).abs();
                assert!(d <= 450, "chunk {k} ({i},{j}): step {d} mm");
            }
        }
    }
}

#[test]
fn golden_height_hash() {
    let p = demo();
    let mut h = 0u64;
    for j in 0..64i64 {
        for i in 0..64i64 {
            let z = p.height_mm(i * 1_000_000 - 32_000_000, p.depth_mm() / 2 + j * 1_000_000);
            h = mix64(h ^ (z as i64 as u64));
        }
    }
    assert_eq!(h, 11417373502760628042, "golden height hash changed");
}

#[test]
fn coarse_height_is_the_terrain_without_its_fine_detail() {
    // The detail layers add at most 8 + 3 + 1 + 0.2 = 12.2 m.
    let p = demo();
    for k in 0..500i64 {
        let (x, y) = (k * 7_919_123, p.depth_mm() / 10 + k * 2_900_000);
        let (fine, coarse) = (p.height_mm(x, y), p.coarse_height_mm(x, y));
        assert!((fine - coarse).abs() <= 12_200, "{x},{y}: {fine} vs {coarse}");
    }
    assert_eq!(p.coarse_height_mm(5, 7), p.coarse_height_mm(5 + p.width_mm(), 7));
}
