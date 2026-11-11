use scalia_core::hash::mix64;
use scalia_core::planet::{height_mm, TERRAIN_MAX_MM, TERRAIN_MIN_MM};

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
    assert_eq!(h, 15696378756055189239, "golden height hash changed");
}
