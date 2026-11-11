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
    let l = BASE_LATTICE_MM >> octave;
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
    let h = TERRAIN_MIN_MM as i64 + n.mul(span).floor_to_i64();
    h.clamp(TERRAIN_MIN_MM as i64, TERRAIN_MAX_MM as i64) as i32
}
