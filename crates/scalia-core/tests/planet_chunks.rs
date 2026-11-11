use scalia_core::planet::{chunk, chunk_hash, height_mm, spawn_points, CELL_MM};

#[test]
fn chunk_has_expected_sample_count() {
    let c = chunk(1, 0, 0, 0, 8);
    assert_eq!(c.cells, 8);
    assert_eq!(c.heights.len(), 81);
}

#[test]
fn same_chunk_same_hash() {
    assert_eq!(chunk_hash(&chunk(3, 1, -4, 9, 16)), chunk_hash(&chunk(3, 1, -4, 9, 16)));
    assert_ne!(chunk_hash(&chunk(3, 1, -4, 9, 16)), chunk_hash(&chunk(3, 1, -4, 10, 16)));
}

fn col(c: &scalia_core::planet::Chunk, i: u32) -> Vec<i32> {
    let n = c.cells + 1;
    (0..n).map(|j| c.heights[(j * n + i) as usize]).collect()
}
fn row(c: &scalia_core::planet::Chunk, j: u32) -> Vec<i32> {
    let n = c.cells + 1;
    (0..n).map(|i| c.heights[(j * n + i) as usize]).collect()
}

#[test]
fn neighbours_agree_on_shared_edge() {
    let n = 16;
    // east edge of (0,0) is the west edge of (1,0); includes negative indices.
    for (ax, ay) in [(0i64, 0i64), (-1, 0), (-1, -1), (5, -3)] {
        let a = chunk(9, 2, ax, ay, n);
        let east = chunk(9, 2, ax + 1, ay, n);
        let south = chunk(9, 2, ax, ay + 1, n);
        assert_eq!(col(&a, n), col(&east, 0), "east of ({ax},{ay})");
        assert_eq!(row(&a, n), row(&south, 0), "south of ({ax},{ay})");
    }
}

#[test]
fn spawn_points_are_flat_and_within_chunk() {
    let (seed, face, cx, cy, cells) = (7u64, 0u8, 3i64, -2i64, 64u32);
    let pts = spawn_points(seed, face, cx, cy, cells, 10);
    assert_eq!(pts.len(), 10);
    let (x0, y0) = (cx * cells as i64 * CELL_MM, cy * cells as i64 * CELL_MM);
    let side = cells as i64 * CELL_MM;
    for (x, y) in pts {
        assert!(x >= x0 && x < x0 + side && y >= y0 && y < y0 + side, "{x},{y}");
        let (cxm, cym) = (x.div_euclid(CELL_MM) * CELL_MM, y.div_euclid(CELL_MM) * CELL_MM);
        let h = [(0, 0), (CELL_MM, 0), (0, CELL_MM), (CELL_MM, CELL_MM)]
            .map(|(dx, dy)| height_mm(seed, face, cxm + dx, cym + dy));
        let spread = h.iter().max().unwrap() - h.iter().min().unwrap();
        assert!(spread <= 500, "spread {spread}");
    }
}

#[test]
fn spawn_points_never_exceed_count() {
    assert_eq!(spawn_points(7, 0, 0, 0, 64, 0).len(), 0);
    assert!(spawn_points(7, 0, 0, 0, 64, 5).len() <= 5);
    assert!(spawn_points(7, 0, 0, 0, 1, 5).len() <= 5);
}

#[test]
fn spawn_points_are_distinct_and_deterministic() {
    let a = spawn_points(11, 0, 1, 1, 64, 20);
    let mut d = a.clone();
    d.sort();
    d.dedup();
    assert_eq!(d.len(), a.len());
    assert_eq!(a, spawn_points(11, 0, 1, 1, 64, 20));
}

#[test]
fn golden_chunk_hash() {
    let h = chunk_hash(&chunk(7, 0, 3, -2, 16));
    println!("GOLDEN {h}");
    assert_eq!(h, 15242347213423968789, "golden chunk hash changed");
}
