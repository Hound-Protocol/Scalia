use scalia_core::hash::mix64;
use scalia_core::planet::{chunk, height_mm, TERRAIN_MAX_MM, TERRAIN_MIN_MM};

#[test]
fn same_inputs_same_height() {
    assert_eq!(height_mm(1, 0, 123_456, -789_012), height_mm(1, 0, 123_456, -789_012));
}

fn grid(seed: u64, face: u8) -> Vec<i32> {
    let mut v = Vec::new();
    for j in 0..10i64 {
        for i in 0..10i64 {
            v.push(height_mm(seed, face, i * 3_700_000 - 17_000_000, j * 4_100_000 - 9_000_000));
        }
    }
    v
}

#[test]
fn different_seed_differs_somewhere() {
    assert_ne!(grid(1, 0), grid(2, 0));
}

#[test]
fn different_face_differs_somewhere() {
    assert_ne!(grid(1, 0), grid(1, 1));
}

#[test]
fn heights_within_range() {
    let big = [i64::MIN, -1_000_000_000_000, -1, 0, 1, 1_000_000_000_000, i64::MAX];
    for &x in &big {
        for &y in &big {
            let h = height_mm(9, 3, x, y);
            assert!((TERRAIN_MIN_MM..=TERRAIN_MAX_MM).contains(&h), "{x},{y} -> {h}");
        }
    }
    for k in 0..10_000i64 {
        let h = height_mm(5, 5, k * 977_131 - 4_000_000_000, k * 1_299_709);
        assert!((TERRAIN_MIN_MM..=TERRAIN_MAX_MM).contains(&h));
    }
}

#[test]
fn face_above_five_is_clamped_to_five() {
    assert_eq!(height_mm(3, 9, 1000, 2000), height_mm(3, 5, 1000, 2000));
}

#[test]
fn continuous_across_zero() {
    for y in [-5_000i64, 0, 5_000, 777_777] {
        let (a, b) = (height_mm(4, 0, -1, y), height_mm(4, 0, 0, y));
        assert!((a - b).abs() < 5_000, "{a} vs {b}");
        let (c, d) = (height_mm(4, 0, y, -1), height_mm(4, 0, y, 0));
        assert!((c - d).abs() < 5_000, "{c} vs {d}");
    }
}

#[test]
fn golden_height_hash() {
    let mut h = 0u64;
    for j in 0..64i64 {
        for i in 0..64i64 {
            let z = height_mm(42, 0, i * 1_000_000 - 32_000_000, j * 1_000_000 - 32_000_000);
            h = mix64(h ^ (z as i64 as u64));
        }
    }
    println!("GOLDEN {h}");
    assert_eq!(h, 13530145724189886874, "golden height hash changed");
}

#[test]
fn local_relief_visible_at_soldier_scale() {
    // Every 64 m patch, including those at lattice points where the coarse noise is flat,
    // shows at least 1.5 m of relief, so the ground is never a featureless plane.
    let spots = (0..20i64).map(|k| (k * 37 - 300, k * 11 - 90)).chain([(0, 0), (-1, 0), (0, -1), (-1, -1)]);
    for (cx, cy) in spots {
        let c = chunk(42, 0, cx, cy, 64);
        let (lo, hi) = (*c.heights.iter().min().unwrap(), *c.heights.iter().max().unwrap());
        assert!(hi - lo >= 1_500, "chunk ({cx},{cy}): relief {} mm", hi - lo);
    }
}

#[test]
fn slopes_are_walkable() {
    // Adjacent 1 m samples never differ by more than 0.45 m (about 24 degrees).
    for k in 0..20i64 {
        let c = chunk(7, 2, k * 53 - 500, 300 - k * 29, 64);
        let n = 65usize;
        for j in 0..n {
            for i in 0..n - 1 {
                let d = (c.heights[j * n + i + 1] - c.heights[j * n + i]).abs();
                assert!(d <= 450, "chunk {k} ({i},{j}): step {d} mm");
            }
        }
    }
}
