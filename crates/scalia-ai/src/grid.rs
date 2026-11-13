//! Who is where: soldiers bucketed into 16 m cells so neighbours, listeners and sight candidates are found
//! without looking at everyone. Rebuilt from positions in index order, so queries are deterministic.
use crate::sight::isqrt;

pub const CELL_MM: i64 = 16_000;

#[derive(Clone, Debug, Default)]
pub struct SoldierGrid {
    x0: i64,
    y0: i64,
    w: i64,
    h: i64,
    cells: Vec<Vec<u32>>,
}

impl SoldierGrid {
    /// A grid over the area (mm); positions outside it are clamped into the border cells.
    pub fn new(x0: i64, y0: i64, w_mm: i64, h_mm: i64) -> SoldierGrid {
        let (w, h) = (((w_mm + CELL_MM - 1) / CELL_MM).max(1), ((h_mm + CELL_MM - 1) / CELL_MM).max(1));
        SoldierGrid { x0, y0, w, h, cells: vec![Vec::new(); (w * h) as usize] }
    }

    fn cell(&self, p: (i64, i64)) -> (i64, i64) {
        (((p.0 - self.x0) / CELL_MM).clamp(0, self.w - 1), ((p.1 - self.y0) / CELL_MM).clamp(0, self.h - 1))
    }

    /// Refills the buckets with soldier `i` at `pos[i]` for every `live[i]`.
    pub fn rebuild(&mut self, pos: impl Iterator<Item = ((i64, i64), bool)>) {
        for c in &mut self.cells {
            c.clear();
        }
        for (i, (p, live)) in pos.enumerate() {
            if live {
                let (cx, cy) = self.cell(p);
                self.cells[(cy * self.w + cx) as usize].push(i as u32);
            }
        }
    }

    /// Everyone in the cells touching the square of half-size `r_mm` around `p` (a superset of those within
    /// `r_mm`; callers check the distance). Cells in row order, each in index order.
    pub fn near(&self, p: (i64, i64), r_mm: i64, out: &mut Vec<u32>) {
        out.clear();
        let (a, b) = (self.cell((p.0 - r_mm, p.1 - r_mm)), self.cell((p.0 + r_mm, p.1 + r_mm)));
        for cy in a.1..=b.1 {
            for cx in a.0..=b.0 {
                out.extend_from_slice(&self.cells[(cy * self.w + cx) as usize]);
            }
        }
    }
}

/// Distance in millimetres.
pub fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    isqrt((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2))
}
