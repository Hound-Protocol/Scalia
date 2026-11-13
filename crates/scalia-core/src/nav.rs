//! Navigation: static props (rocks, walls, crates) rasterised into a grid of 0.5 m cells, with each cell's
//! clearance (distance to the nearest obstacle) so one grid serves movers of every size: a soldier needs about
//! 0.3 m, a truck 1.5 m, a tank 2 m. Integer-only and deterministic.

/// Cell edge in millimetres.
pub const CELL_MM: i64 = 500;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropKind {
    /// A boulder: a disc on the ground.
    Rock { radius_mm: i64 },
    /// A straight wall, running east-west before turning.
    Wall { length_mm: i64, thickness_mm: i64 },
    /// A square crate.
    Crate { size_mm: i64 },
    /// A plywood training-house panel, `PANEL_MM` thick and 2.4 m high, running east-west before turning; with
    /// `window` it has an opening from 1.0 to 2.0 m (still blocks walking, not sight).
    Panel { length_mm: i64, window: bool },
}

impl PropKind {
    /// Thickness of a plywood panel.
    pub const PANEL_MM: i64 = 200;
}

/// A static obstacle at a position, turned by whole quarter turns.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Prop {
    pub kind: PropKind,
    pub x_mm: i64,
    pub y_mm: i64,
    pub quarter_turns: u8,
}

impl Prop {
    /// Whether the prop overlaps a cell whose centre is at (dx, dy) from the prop's centre. Overlap, not just
    /// the centre, so a wall thinner than a cell still blocks it.
    fn overlaps_cell(&self, dx: i64, dy: i64) -> bool {
        let h = CELL_MM / 2;
        match self.kind {
            PropKind::Rock { radius_mm } => {
                // Nearest point of the cell to the rock's centre.
                let nx = dx.signum() * (dx.abs() - h).max(0);
                let ny = dy.signum() * (dy.abs() - h).max(0);
                nx * nx + ny * ny < radius_mm * radius_mm
            }
            _ => {
                let (hx, hy) = self.half_extent();
                dx.abs() < hx + h && dy.abs() < hy + h
            }
        }
    }

    /// Half the width and height of the prop's bounding box.
    fn half_extent(&self) -> (i64, i64) {
        match self.kind {
            PropKind::Rock { radius_mm } => (radius_mm, radius_mm),
            PropKind::Wall { length_mm, thickness_mm } => {
                if self.quarter_turns % 2 == 0 { (length_mm / 2, thickness_mm / 2) } else { (thickness_mm / 2, length_mm / 2) }
            }
            PropKind::Crate { size_mm } => (size_mm / 2, size_mm / 2),
            PropKind::Panel { length_mm, .. } => {
                let t = PropKind::PANEL_MM;
                if self.quarter_turns % 2 == 0 { (length_mm / 2, t / 2) } else { (t / 2, length_mm / 2) }
            }
        }
    }
}

/// A rectangle of cells with its south-west corner at (x_mm, y_mm).
pub struct NavGrid {
    pub x_mm: i64,
    pub y_mm: i64,
    pub w: usize,
    pub h: usize,
    blocked: Vec<bool>,
    clearance_mm: Vec<i64>,
}

impl NavGrid {
    pub fn new(x_mm: i64, y_mm: i64, w: usize, h: usize) -> NavGrid {
        NavGrid { x_mm, y_mm, w, h, blocked: vec![false; w * h], clearance_mm: vec![0; w * h] }
    }

    /// The cell holding a point, if it is inside the grid.
    pub fn cell_of(&self, x_mm: i64, y_mm: i64) -> Option<(usize, usize)> {
        let (i, j) = ((x_mm - self.x_mm).div_euclid(CELL_MM), (y_mm - self.y_mm).div_euclid(CELL_MM));
        (i >= 0 && j >= 0 && (i as usize) < self.w && (j as usize) < self.h).then_some((i as usize, j as usize))
    }

    /// Centre of a cell in millimetres.
    pub fn centre(&self, i: usize, j: usize) -> (i64, i64) {
        (self.x_mm + i as i64 * CELL_MM + CELL_MM / 2, self.y_mm + j as i64 * CELL_MM + CELL_MM / 2)
    }

    pub fn blocked(&self, i: usize, j: usize) -> bool {
        self.blocked[j * self.w + i]
    }

    pub fn clearance(&self, i: usize, j: usize) -> i64 {
        self.clearance_mm[j * self.w + i]
    }

    /// Rasterises the props (a cell is blocked when a prop overlaps it) and recomputes clearance.
    pub fn rebuild(&mut self, props: &[Prop]) {
        self.blocked.iter_mut().for_each(|b| *b = false);
        for p in props {
            let (hx, hy) = p.half_extent();
            let lo = self.cell_range(p.x_mm - hx, p.y_mm - hy);
            let hi = self.cell_range(p.x_mm + hx, p.y_mm + hy);
            for j in lo.1..=hi.1 {
                for i in lo.0..=hi.0 {
                    let (cx, cy) = self.centre(i, j);
                    if p.overlaps_cell(cx - p.x_mm, cy - p.y_mm) {
                        self.blocked[j * self.w + i] = true;
                    }
                }
            }
        }
        self.compute_clearance();
    }

    /// Clamped cell coordinates of a point (for bounding boxes that may poke outside the grid).
    fn cell_range(&self, x_mm: i64, y_mm: i64) -> (usize, usize) {
        let i = (x_mm - self.x_mm).div_euclid(CELL_MM).clamp(0, self.w as i64 - 1);
        let j = (y_mm - self.y_mm).div_euclid(CELL_MM).clamp(0, self.h as i64 - 1);
        (i as usize, j as usize)
    }

    /// Exact Euclidean distance transform (Felzenszwalb and Huttenlocher) over the grid padded with one ring of
    /// blocked cells, so the grid's edge counts as an obstacle. Clearance is the distance from a cell's centre to
    /// the nearest blocked cell's centre, less half a cell; blocked cells have none.
    fn compute_clearance(&mut self) {
        let (pw, ph) = (self.w + 2, self.h + 2);
        const INF: i64 = i64::MAX / 4;
        let mut d = vec![INF; pw * ph];
        for j in 0..ph {
            for i in 0..pw {
                let edge = i == 0 || j == 0 || i == pw - 1 || j == ph - 1;
                if edge || self.blocked[(j - 1) * self.w + (i - 1)] {
                    d[j * pw + i] = 0;
                }
            }
        }
        // Columns, then rows: squared distances in cells.
        let mut line = Vec::new();
        for i in 0..pw {
            line.clear();
            line.extend((0..ph).map(|j| d[j * pw + i]));
            let out = edt_1d(&line);
            for j in 0..ph {
                d[j * pw + i] = out[j];
            }
        }
        for j in 0..ph {
            let out = edt_1d(&d[j * pw..(j + 1) * pw]);
            d[j * pw..(j + 1) * pw].copy_from_slice(&out);
        }
        for j in 0..self.h {
            for i in 0..self.w {
                let sq = d[(j + 1) * pw + (i + 1)];
                self.clearance_mm[j * self.w + i] = if sq == 0 { 0 } else { (isqrt(sq * CELL_MM * CELL_MM) - CELL_MM / 2).max(0) };
            }
        }
    }

    /// Clearance at a point (0 outside the grid).
    pub fn clearance_mm_at(&self, x_mm: i64, y_mm: i64) -> i64 {
        self.cell_of(x_mm, y_mm).map_or(0, |(i, j)| self.clearance(i, j))
    }

    /// Whether a mover of `radius_mm` can stand at a point.
    pub fn walkable_at(&self, x_mm: i64, y_mm: i64, radius_mm: i64) -> bool {
        self.cell_of(x_mm, y_mm).is_some_and(|(i, j)| !self.blocked(i, j) && self.clearance(i, j) >= radius_mm)
    }

    /// A hash of the grid's contents (for determinism checks).
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (b, c) in self.blocked.iter().zip(&self.clearance_mm) {
            for v in [*b as u64, *c as u64] {
                h ^= v;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }
}

/// One-dimensional squared distance transform of a sampled function (0 at obstacles, "infinite" elsewhere).
fn edt_1d(f: &[i64]) -> Vec<i64> {
    let n = f.len();
    let inf = i64::MAX / 4;
    let mut out = vec![inf; n];
    // Lower envelope of parabolas rooted at the finite samples; boundaries compared as exact fractions.
    let mut v: Vec<usize> = Vec::with_capacity(n);
    let mut z_num: Vec<i64> = Vec::with_capacity(n + 1);
    let mut z_den: Vec<i64> = Vec::with_capacity(n + 1);
    for q in 0..n {
        if f[q] >= inf {
            continue;
        }
        loop {
            match v.last() {
                None => {
                    v.push(q);
                    z_num.push(i64::MIN / 4);
                    z_den.push(1);
                    break;
                }
                Some(&p) => {
                    // Intersection of the parabolas at p and q: s = ((f[q] + q^2) - (f[p] + p^2)) / (2(q - p)).
                    let num = (f[q] + (q * q) as i64) - (f[p] + (p * p) as i64);
                    let den = 2 * (q - p) as i64;
                    let k = v.len() - 1;
                    // Drop p while s <= z[k] (compare num/den <= z_num[k]/z_den[k], both denominators positive).
                    if (num as i128) * (z_den[k] as i128) <= (z_num[k] as i128) * (den as i128) && k > 0 {
                        v.pop();
                        z_num.pop();
                        z_den.pop();
                        continue;
                    }
                    if (num as i128) * (z_den[k] as i128) <= (z_num[k] as i128) * (den as i128) {
                        // k == 0: q replaces the only parabola.
                        v[0] = q;
                        break;
                    }
                    v.push(q);
                    z_num.push(num);
                    z_den.push(den);
                    break;
                }
            }
        }
    }
    if v.is_empty() {
        return out;
    }
    let mut k = 0;
    for (q, o) in out.iter_mut().enumerate() {
        // Advance while the next boundary lies at or before q.
        while k + 1 < v.len() && (z_num[k + 1] as i128) <= (q as i128) * (z_den[k + 1] as i128) {
            k += 1;
        }
        let p = v[k];
        let d = q as i64 - p as i64;
        *o = d * d + f[p];
    }
    out
}

/// Integer square root (floor).
fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as i64;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// Finds a route for a mover of `radius_mm` from `from` to `to` (millimetres), as straight legs between
/// waypoints: A* over the cells it fits in (8 neighbours, no cutting corners), then smoothed by dropping every
/// waypoint the leg before it can see past. `None` when either end is blocked or no route exists.
/// Deterministic: ties are broken by cell index, so every machine finds the same route.
pub fn find_path(grid: &NavGrid, from: (i64, i64), to: (i64, i64), radius_mm: i64) -> Option<Vec<(i64, i64)>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let start = grid.cell_of(from.0, from.1)?;
    let goal = grid.cell_of(to.0, to.1)?;
    let fits = |i: usize, j: usize| !grid.blocked(i, j) && grid.clearance(i, j) >= radius_mm;
    if !fits(start.0, start.1) || !fits(goal.0, goal.1) {
        return None;
    }
    let (w, h) = (grid.w, grid.h);
    let idx = |i: usize, j: usize| j * w + i;
    let heuristic = |i: usize, j: usize| {
        let (dx, dy) = ((i as i64 - goal.0 as i64).abs(), (j as i64 - goal.1 as i64).abs());
        10 * (dx + dy) - 6 * dx.min(dy)
    };
    let mut cost = vec![i64::MAX; w * h];
    let mut came = vec![usize::MAX; w * h];
    let mut open = BinaryHeap::new();
    cost[idx(start.0, start.1)] = 0;
    open.push(Reverse((heuristic(start.0, start.1), 0i64, idx(start.0, start.1))));
    let target = idx(goal.0, goal.1);
    while let Some(Reverse((_, g, cur))) = open.pop() {
        if cur == target {
            break;
        }
        if g > cost[cur] {
            continue;
        }
        let (ci, cj) = ((cur % w) as i64, (cur / w) as i64);
        for (dx, dy, step) in [(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)] {
            let (ni, nj) = (ci + dx, cj + dy);
            if ni < 0 || nj < 0 || ni >= w as i64 || nj >= h as i64 || !fits(ni as usize, nj as usize) {
                continue;
            }
            // Diagonals only where both sides are open, so routes never clip a corner.
            if dx != 0 && dy != 0 && (!fits((ci + dx) as usize, cj as usize) || !fits(ci as usize, (cj + dy) as usize)) {
                continue;
            }
            let n = idx(ni as usize, nj as usize);
            let ng = g + step;
            if ng < cost[n] {
                cost[n] = ng;
                came[n] = cur;
                open.push(Reverse((ng + heuristic(ni as usize, nj as usize), ng, n)));
            }
        }
    }
    if cost[target] == i64::MAX {
        return None;
    }
    // Walk back to the start, then string-pull.
    let mut cells = vec![target];
    while *cells.last().unwrap() != idx(start.0, start.1) {
        cells.push(came[*cells.last().unwrap()]);
    }
    cells.reverse();
    let mut points: Vec<(i64, i64)> = vec![from];
    points.extend(cells[1..cells.len().saturating_sub(1)].iter().map(|&c| grid.centre(c % w, c / w)));
    points.push(to);
    let mut out = vec![points[0]];
    let mut k = 0;
    while k + 1 < points.len() {
        // The furthest point still visible from the current one.
        let mut next = k + 1;
        for m in (k + 2..points.len()).rev() {
            if clear_line(grid, points[k], points[m], radius_mm) {
                next = m;
                break;
            }
        }
        out.push(points[next]);
        k = next;
    }
    Some(out)
}

/// Whether a mover of `radius_mm` can walk the straight line between two points (sampled every quarter cell).
pub fn clear_line(grid: &NavGrid, a: (i64, i64), b: (i64, i64), radius_mm: i64) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = ((dx.abs().max(dy.abs())) / (CELL_MM / 4)).max(1);
    (0..=steps).all(|k| grid.walkable_at(a.0 + dx * k / steps, a.1 + dy * k / steps, radius_mm))
}

/// Cost-to-goal over the whole grid for movers of one size: every cell knows how far it is from the goal along
/// the open ground, so any number of soldiers sent to the same place share one field. Integer Dijkstra
/// (straight steps 10, diagonals 14, no corner cutting); ties by cell index, so it is deterministic.
pub struct FlowField {
    x_mm: i64,
    y_mm: i64,
    w: usize,
    h: usize,
    cost: Vec<u32>,
}

const NEIGHBOURS: [(i64, i64, u32); 8] = [(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)];

impl FlowField {
    pub fn new(grid: &NavGrid, goal: (i64, i64), radius_mm: i64) -> Option<FlowField> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let (w, h) = (grid.w, grid.h);
        let fits = |i: i64, j: i64| i >= 0 && j >= 0 && i < w as i64 && j < h as i64 && !grid.blocked(i as usize, j as usize) && grid.clearance(i as usize, j as usize) >= radius_mm;
        let (gi, gj) = grid.cell_of(goal.0, goal.1)?;
        if !fits(gi as i64, gj as i64) {
            return None;
        }
        let mut cost = vec![u32::MAX; w * h];
        let mut open = BinaryHeap::new();
        cost[gj * w + gi] = 0;
        open.push(Reverse((0u32, gj * w + gi)));
        while let Some(Reverse((c, cur))) = open.pop() {
            if c > cost[cur] {
                continue;
            }
            let (ci, cj) = ((cur % w) as i64, (cur / w) as i64);
            for (dx, dy, step) in NEIGHBOURS {
                let (ni, nj) = (ci + dx, cj + dy);
                if !fits(ni, nj) || (dx != 0 && dy != 0 && (!fits(ci + dx, cj) || !fits(ci, cj + dy))) {
                    continue;
                }
                let n = nj as usize * w + ni as usize;
                if c + step < cost[n] {
                    cost[n] = c + step;
                    open.push(Reverse((c + step, n)));
                }
            }
        }
        Some(FlowField { x_mm: grid.x_mm, y_mm: grid.y_mm, w, h, cost })
    }

    fn cell(&self, x: i64, y: i64) -> Option<(i64, i64)> {
        let (i, j) = ((x - self.x_mm).div_euclid(CELL_MM), (y - self.y_mm).div_euclid(CELL_MM));
        (i >= 0 && j >= 0 && i < self.w as i64 && j < self.h as i64).then_some((i, j))
    }

    /// Cost from a point to the goal (tenths of a cell), or `None` where the goal cannot be reached.
    pub fn cost_at(&self, x_mm: i64, y_mm: i64) -> Option<u32> {
        let (i, j) = self.cell(x_mm, y_mm)?;
        let c = self.cost[j as usize * self.w + i as usize];
        (c != u32::MAX).then_some(c)
    }

    /// The neighbouring cell (as a step of -1, 0 or 1 on each axis) that is closest to the goal; (0, 0) at it.
    pub fn direction_at(&self, x_mm: i64, y_mm: i64) -> Option<(i64, i64)> {
        let (i, j) = self.cell(x_mm, y_mm)?;
        let here = self.cost[j as usize * self.w + i as usize];
        if here == u32::MAX {
            return None;
        }
        let mut best = (here, (0, 0));
        for (dx, dy, _) in NEIGHBOURS {
            let (ni, nj) = (i + dx, j + dy);
            if ni < 0 || nj < 0 || ni >= self.w as i64 || nj >= self.h as i64 {
                continue;
            }
            let c = self.cost[nj as usize * self.w + ni as usize];
            if c < best.0 {
                best = (c, (dx, dy));
            }
        }
        Some(best.1)
    }

    /// Centre of a point's cell (millimetres).
    pub fn cell_centre(&self, x_mm: i64, y_mm: i64) -> (i64, i64) {
        let (i, j) = ((x_mm - self.x_mm).div_euclid(CELL_MM), (y_mm - self.y_mm).div_euclid(CELL_MM));
        (self.x_mm + i * CELL_MM + CELL_MM / 2, self.y_mm + j * CELL_MM + CELL_MM / 2)
    }
}
