use scalia_core::planet::{chunk_hash, Chunk, Planet, CELL_MM};

fn demo() -> Planet {
    Planet::demo(9)
}

#[test]
fn chunk_has_expected_sample_count() {
    assert_eq!(demo().chunk(0, 0, 8).heights.len(), 81);
}

#[test]
fn same_chunk_same_hash() {
    let p = demo();
    assert_eq!(
        chunk_hash(&p.chunk(-4, 9, 16)),
        chunk_hash(&p.chunk(-4, 9, 16))
    );
    assert_ne!(
        chunk_hash(&p.chunk(-4, 9, 16)),
        chunk_hash(&p.chunk(-4, 10, 16))
    );
}

fn col(c: &Chunk, i: u32) -> Vec<i32> {
    let n = c.cells + 1;
    (0..n).map(|j| c.heights[(j * n + i) as usize]).collect()
}

#[test]
fn neighbours_agree_on_shared_edge() {
    let p = demo();
    let n = 16;
    for (ax, ay) in [(0, 500), (-3, 700), (11, 900)] {
        let (a, east) = (p.chunk(ax, ay, n), p.chunk(ax + 1, ay, n));
        assert_eq!(col(&a, n), col(&east, 0));
    }
}

#[test]
fn chunks_wrap_around_the_planet() {
    let p = demo();
    let per_lap = p.width_mm() / (16 * CELL_MM);
    assert_eq!(p.chunk(-1, 600, 16), p.chunk(per_lap - 1, 600, 16));
    assert_eq!(p.chunk(3, 600, 16), p.chunk(3 + per_lap, 600, 16));
}

#[test]
fn spawn_points_are_flat_inside_and_off_the_ice() {
    let p = demo();
    let mid = p.depth_mm() / 2 / (64 * CELL_MM);
    let pts = p.spawn_points(2, mid, 64, 10);
    assert!(!pts.is_empty());
    for &(x, y) in &pts {
        assert!(!p.is_ice(y));
        assert!((2 * 64 * CELL_MM..3 * 64 * CELL_MM).contains(&x));
    }
    assert!(
        p.spawn_points(2, 0, 64, 10).is_empty(),
        "no spawns on the polar ice"
    );
    assert_eq!(pts, p.spawn_points(2, mid, 64, 10));
    assert_eq!(p.spawn_points(2, mid, 64, 0).len(), 0);
}

#[test]
fn golden_chunk_hash() {
    let p = Planet::demo(7);
    assert_eq!(chunk_hash(&p.chunk(3, 1000, 16)), 9283842558404932274, "golden chunk hash changed");
}
