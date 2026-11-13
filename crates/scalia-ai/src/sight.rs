//! Line of sight on the battlefield: a grid of 250 mm cells, each holding which 100 mm height bands (0 to
//! 3.2 m) something solid fills. A sight line is sampled every 100 mm and blocked where it passes through a
//! filled band, so windows (open between sill and header), sandbags (low) and walls (high) all work the same way.
//! Integer millimetres, endpoints ordered before sampling: deterministic and symmetric.

use scalia_core::nav::{Prop, PropKind};

pub const CELL_MM: i64 = 250;
const BAND_MM: i64 = 100;
const SAMPLE_MM: i64 = 100;
/// Samples this close to either end are skipped: a man standing against a wall still sees past it.
const END_SKIP_MM: i64 = 200;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

pub struct SightGrid {
    pub x_mm: i64,
    pub y_mm: i64,
    pub w: usize,
    pub h: usize,
    bands: Vec<u32>,
    /// Per block of BLOCK×BLOCK cells (4 m): any band filled anywhere in it. Rays cross empty blocks in one jump.
    coarse: Vec<u32>,
    bw: usize,
}

/// Cells per coarse block side.
const BLOCK: usize = 16;

/// How far a prop can reach from its centre in any direction (mm), turned or not.
fn reach(p: &Prop) -> i64 {
    match p.kind {
        PropKind::Rock { radius_mm } => radius_mm,
        PropKind::Wall { length_mm, thickness_mm } => length_mm / 2 + thickness_mm,
        PropKind::Crate { size_mm } => size_mm,
        PropKind::Panel { length_mm, .. } => length_mm / 2 + PropKind::PANEL_MM,
    }
}

/// Bands 0..top filled (top in millimetres).
fn up_to(top_mm: i64) -> u32 {
    let n = (top_mm / BAND_MM).clamp(0, 32) as u32;
    if n >= 32 { u32::MAX } else { (1u32 << n) - 1 }
}

/// Which height bands a prop fills (the same shapes base_mesh draws).
pub fn bands_of(p: &Prop) -> u32 {
    match p.kind {
        PropKind::Panel { window: false, .. } => up_to(2_400),
        PropKind::Panel { window: true, .. } => up_to(1_000) | (up_to(2_400) & !up_to(2_000)),
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 650 => up_to(2_750),
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 800 => up_to(1_000),
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 3_000 => up_to(2_600),
        PropKind::Wall { .. } => u32::MAX,
        // Guard towers stand on open legs; the platform is above the bands.
        PropKind::Crate { size_mm } if size_mm >= 2_500 => 0,
        PropKind::Crate { size_mm } => up_to(size_mm),
        PropKind::Rock { radius_mm } => up_to(radius_mm * 9 / 10),
    }
}

impl SightGrid {
    pub fn build(x_mm: i64, y_mm: i64, w: usize, h: usize, props: &[Prop]) -> SightGrid {
        let bw = w.div_ceil(BLOCK);
        let mut g = SightGrid { x_mm, y_mm, w, h, bands: vec![0; w * h], coarse: vec![0; bw * h.div_ceil(BLOCK)], bw };
        let hc = CELL_MM / 2;
        for p in props {
            let b = bands_of(p);
            if b == 0 {
                continue;
            }
            // Only the cells around the prop (one cell of margin).
            let r = reach(p) + CELL_MM;
            let span = |lo: i64, origin: i64, n: usize| (lo - origin).div_euclid(CELL_MM).clamp(0, n as i64) as usize;
            let (i0, i1) = (span(p.x_mm - r, x_mm, w), (span(p.x_mm + r, x_mm, w) + 1).min(w));
            let (j0, j1) = (span(p.y_mm - r, y_mm, h), (span(p.y_mm + r, y_mm, h) + 1).min(h));
            for j in j0..j1 {
                for i in i0..i1 {
                    let (cx, cy) = (x_mm + i as i64 * CELL_MM + hc, y_mm + j as i64 * CELL_MM + hc);
                    // A cell is filled if the prop covers its centre or any corner (thin panels too).
                    let hit = [(0, 0), (-hc, -hc), (hc, -hc), (-hc, hc), (hc, hc)].iter().any(|&(dx, dy)| p.covers(cx + dx, cy + dy));
                    if hit {
                        g.bands[j * w + i] |= b;
                        g.coarse[(j / BLOCK) * bw + i / BLOCK] |= b;
                    }
                }
            }
        }
        g
    }


    /// The filled bands at a point (0 outside the grid).
    pub fn bands_at(&self, x: i64, y: i64) -> u32 {
        let (i, j) = ((x - self.x_mm).div_euclid(CELL_MM), (y - self.y_mm).div_euclid(CELL_MM));
        if i < 0 || j < 0 || i as usize >= self.w || j as usize >= self.h {
            return 0;
        }
        self.bands[j as usize * self.w + i as usize]
    }

    /// Whether nothing solid lies between two points. Samples every SAMPLE_MM along the line (skipping
    /// END_SKIP_MM at each end), sample k at a + d·k/n (truncated); empty 4 m blocks are crossed in one jump.
    pub fn clear(&self, a: Point, b: Point) -> bool {
        let (a, b) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) { (a, b) } else { (b, a) };
        let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        let len = isqrt(dx * dx + dy * dy);
        let n = (len / SAMPLE_MM).max(1);
        // Samples within END_SKIP_MM of either end are not tested: along = ⌊len·k/n⌋ must be ≥ END_SKIP_MM and
        // ≤ len − END_SKIP_MM.
        if len < 2 * END_SKIP_MM {
            return true;
        }
        let k_lo = (END_SKIP_MM * n + len - 1) / len;
        // ⌊len·k/n⌋ ≤ len − END ⇔ len·k < (len − END + 1)·n.
        let k_hi = ((len - END_SKIP_MM + 1) * n - 1) / len;
        if k_lo > k_hi {
            return true;
        }
        let block_mm = BLOCK as i64 * CELL_MM;
        // Position of sample k: a + sign·⌊|d|·k / n⌋, kept as quotient and remainder so stepping is division-free.
        struct Axis {
            base: i64,
            sign: i64,
            mag: i64,
            step_q: i64,
            step_r: i64,
            q: i64,
            r: i64,
        }
        let n_ = n;
        let axis = |base: i64, d: i64| {
            let mag = d.abs();
            Axis { base, sign: d.signum(), mag, step_q: mag / n_, step_r: mag % n_, q: 0, r: 0 }
        };
        let (mut ax, mut ay, mut az) = (axis(a.x, dx), axis(a.y, dy), axis(a.z, dz));
        let seek = |c: &mut Axis, k: i64| {
            let v = c.mag * k;
            c.q = v / n_;
            c.r = v % n_;
        };
        let advance = |c: &mut Axis| {
            c.q += c.step_q;
            c.r += c.step_r;
            if c.r >= n_ {
                c.r -= n_;
                c.q += 1;
            }
        };
        let at = |c: &Axis| c.base + c.sign * c.q;
        let mut k = k_lo;
        seek(&mut ax, k);
        seek(&mut ay, k);
        seek(&mut az, k);
        while k <= k_hi {
            let (x, y) = (at(&ax), at(&ay));
            let (i, j) = ((x - self.x_mm).div_euclid(CELL_MM), (y - self.y_mm).div_euclid(CELL_MM));
            let inside = i >= 0 && j >= 0 && (i as usize) < self.w && (j as usize) < self.h;
            let empty = !inside || self.coarse[(j as usize / BLOCK) * self.bw + i as usize / BLOCK] == 0;
            if empty {
                // Jump to the first sample past this block's edge (in the ray's direction).
                let leave = |p: i64, d: i64, origin: i64| -> i64 {
                    if d == 0 {
                        return i64::MAX;
                    }
                    let blk = (p - origin).div_euclid(block_mm);
                    let edge = if d > 0 { origin + (blk + 1) * block_mm } else { origin + blk * block_mm - 1 };
                    ((edge - p).abs() * n + d.abs() - 1) / d.abs()
                };
                let jump = leave(x, dx, self.x_mm).min(leave(y, dy, self.y_mm)).max(1);
                k = k.saturating_add(jump);
                if k <= k_hi {
                    seek(&mut ax, k);
                    seek(&mut ay, k);
                    seek(&mut az, k);
                }
                continue;
            }
            let z = at(&az);
            if z < 0 {
                return false; // into the ground
            }
            let band = z / BAND_MM;
            if band < 32 && self.bands[j as usize * self.w + i as usize] & (1u32 << band) != 0 {
                return false;
            }
            k += 1;
            advance(&mut ax);
            advance(&mut ay);
            advance(&mut az);
        }
        true
    }

    /// The coarse block of a point, if it is in the grid.
    #[allow(dead_code)]
    fn block_at(&self, x: i64, y: i64) -> Option<(i64, i64)> {
        let (i, j) = ((x - self.x_mm).div_euclid(CELL_MM), (y - self.y_mm).div_euclid(CELL_MM));
        if i < 0 || j < 0 || i as usize >= self.w || j as usize >= self.h {
            return None;
        }
        Some((i / BLOCK as i64, j / BLOCK as i64))
    }
}

/// Integer square root (floor).
pub fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = (v as f64).sqrt() as i64; // float-ok: only a first guess; the integer loops below make it exact
    while x * x > v {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= v {
        x += 1;
    }
    x
}
