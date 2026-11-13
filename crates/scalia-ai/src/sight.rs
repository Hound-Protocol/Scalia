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
        let mut g = SightGrid { x_mm, y_mm, w, h, bands: vec![0; w * h] };
        let hc = CELL_MM / 2;
        for p in props {
            let b = bands_of(p);
            if b == 0 {
                continue;
            }
            for j in 0..h {
                for i in 0..w {
                    let (cx, cy) = (x_mm + i as i64 * CELL_MM + hc, y_mm + j as i64 * CELL_MM + hc);
                    // A cell is filled if the prop covers its centre or any corner (thin panels too).
                    let hit = [(0, 0), (-hc, -hc), (hc, -hc), (-hc, hc), (hc, hc)].iter().any(|&(dx, dy)| p.covers(cx + dx, cy + dy));
                    if hit {
                        g.bands[j * w + i] |= b;
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

    /// Whether nothing solid lies between two points.
    pub fn clear(&self, a: Point, b: Point) -> bool {
        let (a, b) = if (a.x, a.y, a.z) <= (b.x, b.y, b.z) { (a, b) } else { (b, a) };
        let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        let len = isqrt(dx * dx + dy * dy);
        let n = (len / SAMPLE_MM).max(1);
        for k in 0..=n {
            let along = len * k / n;
            if along < END_SKIP_MM || len - along < END_SKIP_MM {
                continue;
            }
            let (x, y, z) = (a.x + dx * k / n, a.y + dy * k / n, a.z + dz * k / n);
            if z < 0 {
                return false; // into the ground
            }
            let band = z / BAND_MM;
            if band < 32 && self.bands_at(x, y) & (1u32 << band) != 0 {
                return false;
            }
        }
        true
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
