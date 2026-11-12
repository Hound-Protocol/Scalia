//! Deterministic planet terrain: fixed-point integer value noise, no floats.
//!
//! A planet is one flat rectangle that wraps east-west (x) and runs from pole to
//! pole (y). Its width is a multiple of the coarsest lattice, so every noise layer
//! tiles exactly and the seam is invisible. The top and bottom tenth are polar ice,
//! where nothing can stand. From orbit the same map is drawn as a globe.

use crate::fixed::Fix;
use crate::hash::{hash4, mix64, range_i64, unit, STREAM_TERRAIN};

pub const TERRAIN_MIN_MM: i32 = -2_000_000;
pub const TERRAIN_MAX_MM: i32 = 8_000_000;
/// Spacing of the chunk sample grid.
pub const CELL_MM: i64 = 1_000;
/// Coarsest lattice spacing (32 km); planet widths are multiples of it.
pub const LATTICE_MM: i64 = 32_000_000;

const OCTAVES: u64 = 4;
/// Octave weights 8, 4, 2, 1 out of 15; each octave halves the lattice.
const WEIGHT_TOTAL: i64 = 15;

/// Fine detail on top of the coarse terrain: (lattice spacing, peak amplitude), both in mm.
/// Slopes stay gentle: each layer's steepest slope is 1.5 * 2 * amplitude / spacing, at most 0.094.
const DETAIL: [(i64, i64); 4] = [(512_000, 8_000), (128_000, 3_000), (32_000, 1_000), (8_000, 200)];
/// Octave tags for the detail layers, after the coarse octaves 0..4.
const DETAIL_TAG_BASE: u64 = 8;

fn lattice_value(seed: u64, octave: u64, ix: i64, iy: i64) -> Fix {
    let tag = (STREAM_TERRAIN << 8) | octave;
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

/// The four lattice values around one cell, remembered so neighbouring samples skip the hashing.
type Corners = Option<((i64, i64), [Fix; 4])>;

/// Noise in [0, 1) at (x, y) on a lattice of spacing `l`. `x` is already in [0, width) and lattice
/// columns wrap modulo `width / l`, so the noise tiles east-west.
fn noise_cached(seed: u64, width: i64, octave: u64, l: i64, x: i64, y: i64, cache: &mut Corners) -> Fix {
    let (ix, iy) = (x.div_euclid(l), y.div_euclid(l));
    // 0 <= rem < l < 2^31, so rem << 32 fits in u64: same value as Fix::from_ratio, without i128 division.
    let frac = |r: i64| Fix((((r as u64) << 32) / l as u64) as i64);
    let tx = smooth(frac(x.rem_euclid(l)));
    let ty = smooth(frac(y.rem_euclid(l)));
    let c = match cache {
        Some((key, c)) if *key == (ix, iy) => *c,
        _ => {
            let cols = width / l;
            let v = |dx: i64, dy: i64| lattice_value(seed, octave, (ix + dx).rem_euclid(cols), iy + dy);
            let c = [v(0, 0), v(1, 0), v(0, 1), v(1, 1)];
            *cache = Some(((ix, iy), c));
            c
        }
    };
    let top = lerp(c[0], c[1], tx);
    let bottom = lerp(c[2], c[3], tx);
    lerp(top, bottom, ty)
}

/// A square patch of terrain samples on a regular grid with `CELL_MM` spacing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub cells: u32,
    /// `(cells + 1)^2` heights in millimetres, row-major with y as the outer index.
    pub heights: Vec<i32>,
}

pub fn chunk_hash(c: &Chunk) -> u64 {
    let mut h = mix64(c.cells as u64);
    for &z in &c.heights {
        h = mix64(h ^ (z as i64 as u64));
    }
    h
}

/// One wrapping planet map.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Planet {
    pub seed: u64,
    width_mm: i64,
}

impl Planet {
    /// A planet `width_km` around; `None` unless it is a positive multiple of 32 km.
    pub fn new(seed: u64, width_km: u32) -> Option<Planet> {
        let width_mm = width_km as i64 * 1_000_000;
        (width_km > 0 && width_mm % LATTICE_MM == 0).then_some(Planet { seed, width_mm })
    }

    /// The demo planet: 4,096 km around.
    pub fn demo(seed: u64) -> Planet {
        Planet::new(seed, 4096).expect("multiple of 32 km")
    }

    /// East-west circumference.
    pub fn width_mm(&self) -> i64 {
        self.width_mm
    }

    /// Pole-to-pole distance: half the circumference.
    pub fn depth_mm(&self) -> i64 {
        self.width_mm / 2
    }

    /// The top and bottom tenth of the map are impassable polar ice.
    pub fn is_ice(&self, y_mm: i64) -> bool {
        let band = self.depth_mm() / 10;
        y_mm < band || y_mm > self.depth_mm() - band
    }

    /// Terrain height at (x, y): x wraps around the planet, y is clamped to the poles.
    pub fn height_mm(&self, x_mm: i64, y_mm: i64) -> i32 {
        self.height_cached(x_mm, y_mm, &mut [None; 8])
    }

    /// `height_mm` with one corner cache per noise layer (4 coarse octaves, then the detail layers).
    fn height_cached(&self, x_mm: i64, y_mm: i64, cache: &mut [Corners; 8]) -> i32 {
        let (seed, w) = (self.seed, self.width_mm);
        let x = x_mm.rem_euclid(w);
        let y = y_mm.clamp(0, self.depth_mm());
        let mut sum: i128 = 0;
        for o in 0..OCTAVES {
            let v = noise_cached(seed, w, o, LATTICE_MM >> o, x, y, &mut cache[o as usize]);
            sum += (8i128 >> o) * v.0 as i128;
        }
        let n = Fix((sum / WEIGHT_TOTAL as i128) as i64); // in [0, 1)
        let span = Fix::from_int(TERRAIN_MAX_MM - TERRAIN_MIN_MM);
        let mut h = TERRAIN_MIN_MM as i64 + n.mul(span).floor_to_i64();
        for (k, &(l, amp)) in DETAIL.iter().enumerate() {
            // Centred noise in [-amp, amp).
            let v = noise_cached(seed, w, DETAIL_TAG_BASE + k as u64, l, x, y, &mut cache[OCTAVES as usize + k]);
            h += ((v.0 as i128 * 2 * amp as i128) >> 32) as i64 - amp;
        }
        h.clamp(TERRAIN_MIN_MM as i64, TERRAIN_MAX_MM as i64) as i32
    }

    /// Chunk (cx, cy) covers x from `cx * cells * CELL_MM` for `cells * CELL_MM` millimetres, and
    /// the same in y. Neighbours share edge samples, and chunks repeat once per lap of the planet.
    pub fn chunk(&self, cx: i64, cy: i64, cells: u32) -> Chunk {
        let side = cells as i64 * CELL_MM;
        let (x0, y0) = (cx.wrapping_mul(side), cy.wrapping_mul(side));
        let n = cells as usize + 1;
        let mut heights = Vec::with_capacity(n * n);
        let mut cache = [None; 8];
        for j in 0..n as i64 {
            let y = y0.wrapping_add(j * CELL_MM);
            for i in 0..n as i64 {
                heights.push(self.height_cached(x0.wrapping_add(i * CELL_MM), y, &mut cache));
            }
        }
        Chunk { cells, heights }
    }

    /// Up to `count` distinct cell centres in a chunk that are flat (corner spread at most 0.5 m)
    /// and off the polar ice; deterministic for the same planet and chunk.
    pub fn spawn_points(&self, cx: i64, cy: i64, cells: u32, count: usize) -> Vec<(i64, i64)> {
        let mut out: Vec<(i64, i64)> = Vec::new();
        if count == 0 || cells == 0 {
            return out;
        }
        let c = self.chunk(cx, cy, cells);
        let side = cells as i64 * CELL_MM;
        let (x0, y0) = (cx.wrapping_mul(side), cy.wrapping_mul(side));
        let n = cells as usize + 1;
        let total = cells as i64 * cells as i64;
        let tag = (STREAM_TERRAIN << 8) | 0xF;
        let salt_a = self.seed ^ mix64(cx as u64);
        let salt_b = mix64(cy as u64);
        for k in 0..(count as u64).saturating_mul(16) {
            if out.len() == count {
                break;
            }
            let idx = range_i64(hash4(salt_a, tag, k, salt_b), 0, total) as usize;
            let (i, j) = (idx % cells as usize, idx / cells as usize);
            let corners = [c.heights[j * n + i], c.heights[j * n + i + 1], c.heights[(j + 1) * n + i], c.heights[(j + 1) * n + i + 1]];
            let spread = corners.iter().max().unwrap() - corners.iter().min().unwrap();
            let p = (x0.wrapping_add(i as i64 * CELL_MM + CELL_MM / 2), y0.wrapping_add(j as i64 * CELL_MM + CELL_MM / 2));
            if spread <= 500 && !self.is_ice(p.1) && !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }
}
