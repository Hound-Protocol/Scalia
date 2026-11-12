//! Soldiers as individual agents: each walks his own way to his own slot at the destination. They share one flow
//! field per order, walk straight to their slot once they can see it, keep their distance from each other and slide
//! along walls rather than stopping. Integer millimetres throughout, updated in a fixed order: deterministic.

use crate::nav::{clear_line, FlowField, NavGrid};
use std::collections::HashMap;

/// Spacing between slots at the destination, and how close two soldiers may come before they push apart.
const SLOT_MM: i64 = 900;
const PERSONAL_MM: i64 = 700;
/// Within this of his slot a soldier has arrived.
const ARRIVE_MM: i64 = 150;

pub struct AgentGroup {
    field: FlowField,
    pos: Vec<(i64, i64)>,
    slot: Vec<(i64, i64)>,
    arrived: Vec<bool>,
    sees_slot: Vec<bool>,
    speed_mm_s: i64,
    radius_mm: i64,
    ticks: u32,
}

impl AgentGroup {
    /// Orders soldiers standing at `starts` to `dest`: slots in a square block centred there (open slots only).
    /// `None` when the destination cannot be reached.
    pub fn new(grid: &NavGrid, starts: Vec<(i64, i64)>, dest: (i64, i64), speed_mm_s: i64, radius_mm: i64) -> Option<AgentGroup> {
        let field = FlowField::new(grid, dest, radius_mm)?;
        let n = starts.len();
        let cols = (n as f64).sqrt().ceil() as i64;
        // Candidate slots spiral outwards ring by ring until there are enough open, reachable ones.
        let mut slots = Vec::with_capacity(n);
        let mut ring = 0i64;
        while slots.len() < n && ring < 200 {
            let half = cols / 2 + ring;
            for j in -half..=half {
                for i in -half..=half {
                    if (i.abs().max(j.abs())) != half && ring > 0 {
                        continue;
                    }
                    let p = (dest.0 + i * SLOT_MM, dest.1 + j * SLOT_MM);
                    if slots.len() < n && grid.walkable_at(p.0, p.1, radius_mm) && field.cost_at(p.0, p.1).is_some() && !slots.contains(&p) {
                        slots.push(p);
                    }
                }
            }
            ring += 1;
        }
        if slots.len() < n {
            return None;
        }
        // Pair soldiers and slots in the same order (both by row, then column) so the block keeps its shape.
        let order = |v: &[(i64, i64)]| {
            let mut idx: Vec<usize> = (0..v.len()).collect();
            idx.sort_by_key(|&k| (v[k].1 / SLOT_MM, v[k].0, k));
            idx
        };
        let (so, sl) = (order(&starts), order(&slots));
        let mut slot = vec![(0, 0); n];
        for (a, b) in so.iter().zip(&sl) {
            slot[*a] = slots[*b];
        }
        Some(AgentGroup { field, pos: starts, slot, arrived: vec![false; n], sees_slot: vec![false; n], speed_mm_s, radius_mm, ticks: 0 })
    }

    pub fn positions(&self) -> &[(i64, i64)] {
        &self.pos
    }

    pub fn arrived(&self) -> &[bool] {
        &self.arrived
    }

    pub fn all_arrived(&self) -> bool {
        self.arrived.iter().all(|&a| a)
    }

    /// Advances everyone by `dt_ms`. All moves are computed from the old positions, then applied.
    pub fn step(&mut self, grid: &NavGrid, dt_ms: i64) {
        // Line of sight to the slot is checked twice a second (it is the expensive part).
        if self.ticks % 10 == 0 {
            for i in 0..self.pos.len() {
                self.sees_slot[i] = clear_line(grid, self.pos[i], self.slot[i], self.radius_mm);
            }
        }
        self.ticks += 1;
        let mut buckets: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, p) in self.pos.iter().enumerate() {
            buckets.entry((p.0.div_euclid(1000), p.1.div_euclid(1000))).or_default().push(i);
        }
        let step_len = self.speed_mm_s * dt_ms / 1000;
        let mut next = self.pos.clone();
        for i in 0..self.pos.len() {
            let p = self.pos[i];
            // Where he wants to go.
            let mut want = (0i64, 0i64);
            if !self.arrived[i] {
                let aim = if self.sees_slot[i] {
                    self.slot[i]
                } else {
                    match self.field.direction_at(p.0, p.1) {
                        Some((dx, dy)) => {
                            let c = self.field.cell_centre(p.0, p.1);
                            (c.0 + dx * 500, c.1 + dy * 500)
                        }
                        None => self.slot[i],
                    }
                };
                let (dx, dy) = (aim.0 - p.0, aim.1 - p.1);
                let d = isqrt(dx * dx + dy * dy);
                if d > 0 {
                    let len = step_len.min(d);
                    want = (dx * len / d, dy * len / d);
                }
            }
            // Personal space: push away from anyone closer than PERSONAL_MM.
            let mut push = (0i64, 0i64);
            let b = (p.0.div_euclid(1000), p.1.div_euclid(1000));
            for by in b.1 - 1..=b.1 + 1 {
                for bx in b.0 - 1..=b.0 + 1 {
                    for &j in buckets.get(&(bx, by)).map(|v| v.as_slice()).unwrap_or(&[]) {
                        if j == i {
                            continue;
                        }
                        let (dx, dy) = (p.0 - self.pos[j].0, p.1 - self.pos[j].1);
                        let d = isqrt(dx * dx + dy * dy);
                        if d < PERSONAL_MM {
                            if d == 0 {
                                push.0 += if i < j { -40 } else { 40 };
                            } else {
                                push.0 += dx * (PERSONAL_MM - d) / d / 4;
                                push.1 += dy * (PERSONAL_MM - d) / d / 4;
                            }
                        }
                    }
                }
            }
            if self.arrived[i] {
                push = (0, 0); // arrived soldiers hold their slot; the others step round them
            }
            let lim = step_len * 3 / 2;
            push = (push.0.clamp(-lim, lim), push.1.clamp(-lim, lim));
            let (mx, my) = (want.0 + push.0, want.1 + push.1);
            // Slide along walls: try the full move, then each axis alone.
            for cand in [(p.0 + mx, p.1 + my), (p.0 + mx, p.1), (p.0, p.1 + my)] {
                if grid.walkable_at(cand.0, cand.1, self.radius_mm) {
                    next[i] = cand;
                    break;
                }
            }
        }
        self.pos = next;
        for i in 0..self.pos.len() {
            let (dx, dy) = (self.slot[i].0 - self.pos[i].0, self.slot[i].1 - self.pos[i].1);
            if dx * dx + dy * dy <= ARRIVE_MM * ARRIVE_MM {
                self.arrived[i] = true;
            }
        }
    }
}

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
