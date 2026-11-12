//! Deterministic planet terrain: fixed-point integer value noise, no floats.
//!
//! The planet is a cube-sphere with six faces; each face is sampled as a flat
//! patch in millimetres. Faces differ because the face index is mixed into the
//! hash. Continuity across face edges is not provided yet.

use crate::fixed::Fix;
use crate::hash::{hash4, unit, STREAM_TERRAIN};

pub const TERRAIN_MIN_MM: i32 = -2_000_000;
pub const TERRAIN_MAX_MM: i32 = 8_000_000;
/// Spacing of the chunk sample grid.
pub const CELL_MM: i64 = 1_000;

const OCTAVES: u64 = 4;
/// Lattice spacing of the coarsest octave (32 km); each octave halves it.
const BASE_LATTICE_MM: i64 = 32_000_000;
/// Octave weights 8, 4, 2, 1 out of 15.
const WEIGHT_TOTAL: i64 = 15;

/// Fine detail on top of the coarse terrain: (lattice spacing, peak amplitude), both in mm.
/// Slopes stay gentle: each layer's steepest slope is 1.5 * 2 * amplitude / spacing, at most 0.094.
const DETAIL: [(i64, i64); 4] = [(512_000, 8_000), (128_000, 3_000), (32_000, 1_000), (8_000, 200)];
/// Octave tags for the detail layers, after the coarse octaves 0..4.
const DETAIL_TAG_BASE: u64 = 8;

fn lattice_value(seed: u64, face: u8, octave: u64, ix: i64, iy: i64) -> Fix {
    let tag = (STREAM_TERRAIN << 8) | ((face as u64) << 4) | octave;
    unit(hash4(seed, tag, ix as u64, iy as u64))
}

/// 3t^2 - 2t^3 for t in [0, 1).
fn smooth(t: Fix) -> Fix {
    let t2 = t.mul(t);
    Fix(3 * t2.0 - 2 * t2.mul(t).0)
}

/// a + (b - a) * t; values are all in [0, 1) so nothing overflows.
fn lerp(a: Fix, b: Fix, t: Fix) -> Fix {
    Fix(a.0 + Fix(b.0 - a.0).mul(t).0)
}

/// Noise in [0, 1) at (x, y) for one octave, using floor division so negative
/// coordinates continue smoothly through zero.
fn octave_noise(seed: u64, face: u8, octave: u64, x: i64, y: i64) -> Fix {
    noise_at(seed, face, octave, BASE_LATTICE_MM >> octave, x, y)
}

fn noise_at(seed: u64, face: u8, octave: u64, l: i64, x: i64, y: i64) -> Fix {
    let (ix, iy) = (x.div_euclid(l), y.div_euclid(l));
    let tx = smooth(Fix::from_ratio(x.rem_euclid(l), l).unwrap());
    let ty = smooth(Fix::from_ratio(y.rem_euclid(l), l).unwrap());
    let v = |dx: i64, dy: i64| lattice_value(seed, face, octave, ix.wrapping_add(dx), iy.wrapping_add(dy));
    let top = lerp(v(0, 0), v(1, 0), tx);
    let bottom = lerp(v(0, 1), v(1, 1), tx);
    lerp(top, bottom, ty)
}

/// Terrain height in millimetres at (x_mm, y_mm) on a face. `face` above 5 counts as 5.
pub fn height_mm(seed: u64, face: u8, x_mm: i64, y_mm: i64) -> i32 {
    let face = face.min(5);
    let mut sum: i128 = 0;
    for o in 0..OCTAVES {
        let weight = 8i128 >> o;
        sum += weight * octave_noise(seed, face, o, x_mm, y_mm).0 as i128;
    }
    let n = Fix((sum / WEIGHT_TOTAL as i128) as i64); // in [0, 1)
    let span = Fix::from_int(TERRAIN_MAX_MM - TERRAIN_MIN_MM);
    let mut h = TERRAIN_MIN_MM as i64 + n.mul(span).floor_to_i64();
    for (k, &(l, amp)) in DETAIL.iter().enumerate() {
        // Centred noise in [-amp, amp).
        let v = noise_at(seed, face, DETAIL_TAG_BASE + k as u64, l, x_mm, y_mm);
        h += ((v.0 as i128 * 2 * amp as i128) >> 32) as i64 - amp;
    }
    h.clamp(TERRAIN_MIN_MM as i64, TERRAIN_MAX_MM as i64) as i32
}

/// A square patch of terrain samples on a regular grid with `CELL_MM` spacing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub cells: u32,
    /// `(cells + 1)^2` heights in millimetres, row-major with y as the outer index.
    pub heights: Vec<i32>,
}

/// Chunk (cx, cy) covers x from `cx * cells * CELL_MM` for `cells * CELL_MM` millimetres,
/// and the same in y; neighbouring chunks share their edge samples exactly.
/// Coordinates wrap on i64 overflow rather than panic.
pub fn chunk(seed: u64, face: u8, cx: i64, cy: i64, cells: u32) -> Chunk {
    let side = cells as i64 * CELL_MM;
    let (x0, y0) = (cx.wrapping_mul(side), cy.wrapping_mul(side));
    let n = cells as usize + 1;
    let mut heights = Vec::with_capacity(n * n);
    for j in 0..n as i64 {
        let y = y0.wrapping_add(j * CELL_MM);
        for i in 0..n as i64 {
            heights.push(height_mm(seed, face, x0.wrapping_add(i * CELL_MM), y));
        }
    }
    Chunk { cells, heights }
}

pub fn chunk_hash(c: &Chunk) -> u64 {
    let mut h = crate::hash::mix64(c.cells as u64);
    for &z in &c.heights {
        h = crate::hash::mix64(h ^ (z as i64 as u64));
    }
    h
}

/// Up to `count` distinct points (cell centres, in millimetres) on cells whose four
/// corner heights differ by at most 500 mm, picked in hash order.
pub fn spawn_points(seed: u64, face: u8, cx: i64, cy: i64, cells: u32, count: usize) -> Vec<(i64, i64)> {
    use crate::hash::{hash4, mix64, range_i64};
    let mut out: Vec<(i64, i64)> = Vec::new();
    if count == 0 || cells == 0 {
        return out;
    }
    let c = chunk(seed, face, cx, cy, cells);
    let side = cells as i64 * CELL_MM;
    let (x0, y0) = (cx.wrapping_mul(side), cy.wrapping_mul(side));
    let n = cells as usize + 1;
    let total = cells as i64 * cells as i64;
    let tag = (STREAM_TERRAIN << 8) | 0xF;
    let salt_a = seed ^ mix64(cx as u64);
    let salt_b = mix64(cy as u64) ^ face as u64;
    for k in 0..(count as u64).saturating_mul(16) {
        if out.len() == count {
            break;
        }
        let idx = range_i64(hash4(salt_a, tag, k, salt_b), 0, total) as usize;
        let (i, j) = (idx % cells as usize, idx / cells as usize);
        let corners = [c.heights[j * n + i], c.heights[j * n + i + 1], c.heights[(j + 1) * n + i], c.heights[(j + 1) * n + i + 1]];
        let spread = corners.iter().max().unwrap() - corners.iter().min().unwrap();
        let p = (x0.wrapping_add(i as i64 * CELL_MM + CELL_MM / 2), y0.wrapping_add(j as i64 * CELL_MM + CELL_MM / 2));
        if spread <= 500 && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}
