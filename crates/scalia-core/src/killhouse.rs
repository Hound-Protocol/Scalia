//! The training house ("killhouse"): a roofless plywood house of four rooms on the testing ground, where the
//! enemy squad is stationed. One layout feeds the renderer, the navigation grid, the structure scan and the AI
//! (rooms, doors between them, windows). Integer millimetres.

use crate::nav::{Prop, PropKind};

/// A rectangle of floor, south-west corner to north-east corner (absolute millimetres).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub min: (i64, i64),
    pub max: (i64, i64),
}

impl Room {
    pub fn centre(&self) -> (i64, i64) {
        ((self.min.0 + self.max.0) / 2, (self.min.1 + self.max.1) / 2)
    }
    pub fn contains(&self, x: i64, y: i64) -> bool {
        x > self.min.0 && x < self.max.0 && y > self.min.1 && y < self.max.1
    }
}

/// A doorway: its centre, whether the wall it is in runs along x, and the two rooms it joins (None: outside).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Door {
    pub x_mm: i64,
    pub y_mm: i64,
    pub along_x: bool,
    pub width_mm: i64,
    pub rooms: (Option<usize>, Option<usize>),
}

/// A window (or any opening): its centre and whether its wall runs along x.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    pub x_mm: i64,
    pub y_mm: i64,
    pub along_x: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Gap {
    Door,
    Window,
}

/// One wall line (local metres): fixed coordinate, from, to, along x; and its openings (from, to, kind).
struct Line {
    at: f64,
    from: f64,
    to: f64,
    along_x: bool,
    gaps: &'static [(f64, f64, Gap)],
}

/// Footprint (metres) and wall heights.
pub const SIZE_M: (f64, f64) = (20.0, 14.0);
pub const WALL_TOP_M: f32 = 2.4;
pub const WINDOW_SILL_M: f32 = 1.0;
pub const WINDOW_TOP_M: f32 = 2.0;

/// Rooms (local metres): A south-west, B south-east, C north-west, D north-east.
const ROOMS: [(f64, f64, f64, f64); 4] = [(0.0, 0.0, 8.0, 7.0), (8.0, 0.0, 20.0, 7.0), (0.0, 7.0, 10.0, 14.0), (10.0, 7.0, 20.0, 14.0)];

const LINES: [Line; 7] = [
    // Outer walls: south (main door into B), north, west (door into C), east (door into D).
    Line { at: 0.0, from: 0.0, to: 20.0, along_x: true, gaps: &[(3.0, 4.5, Gap::Window), (12.0, 13.4, Gap::Door)] },
    Line { at: 14.0, from: 0.0, to: 20.0, along_x: true, gaps: &[(4.5, 6.0, Gap::Window), (14.0, 15.5, Gap::Window)] },
    Line { at: 0.0, from: 0.0, to: 14.0, along_x: false, gaps: &[(2.5, 4.0, Gap::Window), (10.0, 11.4, Gap::Door)] },
    Line { at: 20.0, from: 0.0, to: 14.0, along_x: false, gaps: &[(3.0, 4.5, Gap::Window), (9.5, 10.9, Gap::Door)] },
    // Inside: A|B, the east-west wall between south and north rooms (A-C, B-D), C|D.
    Line { at: 8.0, from: 0.0, to: 7.0, along_x: false, gaps: &[(3.0, 4.2, Gap::Door)] },
    Line { at: 7.0, from: 0.0, to: 20.0, along_x: true, gaps: &[(3.5, 4.7, Gap::Door), (14.0, 15.2, Gap::Door)] },
    Line { at: 10.0, from: 7.0, to: 14.0, along_x: false, gaps: &[(10.0, 11.2, Gap::Door)] },
];

/// One wall line of a house (local metres), with its openings (from, to, kind).
#[derive(Clone, Debug, PartialEq)]
struct Wall {
    at: f64,
    from: f64,
    to: f64,
    along_x: bool,
    gaps: Vec<(f64, f64, Gap)>,
}

#[derive(Clone, Debug)]
pub struct Killhouse {
    /// South-west corner (absolute millimetres).
    pub origin: (i64, i64),
    /// Footprint (metres).
    pub size_m: (f64, f64),
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub windows: Vec<Opening>,
    walls: Vec<Wall>,
}

fn mm(m: f64) -> i64 {
    (m * 1000.0).round() as i64
}

impl Killhouse {
    /// The testing ground's fixed house (four rooms).
    pub fn new(x_mm: i64, y_mm: i64) -> Killhouse {
        let walls = LINES.iter().map(|l| Wall { at: l.at, from: l.from, to: l.to, along_x: l.along_x, gaps: l.gaps.to_vec() }).collect();
        Killhouse::build((x_mm, y_mm), SIZE_M, &ROOMS, walls)
    }

    fn build(o: (i64, i64), size_m: (f64, f64), local_rooms: &[(f64, f64, f64, f64)], walls: Vec<Wall>) -> Killhouse {
        let rooms: Vec<Room> = local_rooms.iter().map(|&(x0, y0, x1, y1)| Room { min: (o.0 + mm(x0), o.1 + mm(y0)), max: (o.0 + mm(x1), o.1 + mm(y1)) }).collect();
        let mut doors = Vec::new();
        let mut windows = Vec::new();
        for l in &walls {
            for &(a, b, kind) in &l.gaps {
                let mid = (a + b) / 2.0;
                let (x, y) = if l.along_x { (o.0 + mm(mid), o.1 + mm(l.at)) } else { (o.0 + mm(l.at), o.1 + mm(mid)) };
                match kind {
                    Gap::Window => windows.push(Opening { x_mm: x, y_mm: y, along_x: l.along_x }),
                    Gap::Door => {
                        // The rooms half a metre either side of the doorway.
                        let (dx, dy) = if l.along_x { (0, 500) } else { (500, 0) };
                        let side = |x: i64, y: i64| rooms.iter().position(|r| r.contains(x, y));
                        doors.push(Door { x_mm: x, y_mm: y, along_x: l.along_x, width_mm: mm(b - a), rooms: (side(x - dx, y - dy), side(x + dx, y + dy)) });
                    }
                }
            }
        }
        Killhouse { origin: o, size_m, rooms, doors, windows, walls }
    }

    /// A house laid out afresh from `seed`, for training: 24 to 32 m by 16 to 22 m, split into rooms of at least
    /// 4 m (5 to 9 of them), every room reachable, two to four ways in from outside, windows along the outer walls.
    pub fn generate(x_mm: i64, y_mm: i64, seed: u64) -> Killhouse {
        Killhouse::generate_within(x_mm, y_mm, seed, (32, 22))
    }

    /// As `generate`, the footprint capped at `max_m` (whole metres; at least 18 by 14).
    pub fn generate_within(x_mm: i64, y_mm: i64, seed: u64, max_m: (u64, u64)) -> Killhouse {
        let mut r = Rand(seed ^ 0x6b17_4f05);
        let (mw, mh) = (max_m.0.max(18), max_m.1.max(14));
        let (w, h) = ((mw - 8).max(18) as f64 + r.below(9.min(mw - (mw - 8).max(18) + 1)) as f64, (mh - 6).max(14) as f64 + r.below(7.min(mh - (mh - 6).max(14) + 1)) as f64);
        // Split the footprint (binary space partition) into rooms; each split is an inner wall.
        let mut stack = vec![(0.0, 0.0, w, h)];
        let (mut rooms, mut walls) = (Vec::new(), Vec::new());
        while let Some((x0, y0, x1, y1)) = stack.pop() {
            let (dw, dh): (f64, f64) = (x1 - x0, y1 - y0);
            let (cut_x, cut_y) = (dw >= 9.0, dh >= 8.0);
            // Stop at 9 rooms; small rooms stop by chance, so sizes vary.
            let full = rooms.len() + stack.len() + 1 >= 9;
            if (!cut_x && !cut_y) || full || dw * dh < 40.0 || (dw * dh < 90.0 && r.below(5) < 3) {
                rooms.push((x0, y0, x1, y1));
                continue;
            }
            if cut_x && (dw >= dh || !cut_y) {
                let at = x0 + 4.0 + r.below((dw - 8.0) as u64 + 1) as f64;
                walls.push(Wall { at, from: y0, to: y1, along_x: false, gaps: Vec::new() });
                stack.push((x0, y0, at, y1));
                stack.push((at, y0, x1, y1));
            } else {
                let at = y0 + 4.0 + r.below((dh - 8.0) as u64 + 1) as f64;
                walls.push(Wall { at, from: x0, to: x1, along_x: true, gaps: Vec::new() });
                stack.push((x0, y0, x1, at));
                stack.push((x0, at, x1, y1));
            }
        }
        let inner = walls.len();
        walls.push(Wall { at: 0.0, from: 0.0, to: w, along_x: true, gaps: Vec::new() });
        walls.push(Wall { at: h, from: 0.0, to: w, along_x: true, gaps: Vec::new() });
        walls.push(Wall { at: 0.0, from: 0.0, to: h, along_x: false, gaps: Vec::new() });
        walls.push(Wall { at: w, from: 0.0, to: h, along_x: false, gaps: Vec::new() });
        // Where other walls meet each line: openings keep 0.7 m clear of those joints.
        let joints: Vec<Vec<f64>> = (0..walls.len())
            .map(|i| {
                let l = &walls[i];
                let mut j = vec![l.from, l.to];
                for m in walls.iter().filter(|m| m.along_x != l.along_x) {
                    if m.at > l.from && m.at < l.to && l.at >= m.from - 1e-6 && l.at <= m.to + 1e-6 {
                        j.push(m.at);
                    }
                }
                j.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                j
            })
            .collect();
        // Puts an opening of `width` somewhere free on line `i`; false if there is no room for it.
        let place = |walls: &mut Vec<Wall>, r: &mut Rand, i: usize, width: f64, kind: Gap| -> bool {
            let segs: Vec<(f64, f64)> = joints[i].windows(2).map(|p| (p[0] + 0.7, p[1] - 0.7)).filter(|(a, b)| b - a >= width).collect();
            for _ in 0..8 {
                if segs.is_empty() {
                    return false;
                }
                let (a, b) = segs[r.below(segs.len() as u64) as usize];
                let from = a + r.below(((b - a - width) * 10.0) as u64 + 1) as f64 / 10.0;
                let clash = walls[i].gaps.iter().any(|&(g0, g1, _)| from < g1 + 0.6 && from + width > g0 - 0.6);
                if !clash {
                    walls[i].gaps.push((from, from + width, kind));
                    return true;
                }
            }
            false
        };
        // Every inner wall has a door (so every room can be reached); long ones sometimes a second.
        for i in 0..inner {
            place(&mut walls, &mut r, i, 1.2, Gap::Door);
            if walls[i].to - walls[i].from > 9.0 && r.below(3) == 0 {
                place(&mut walls, &mut r, i, 1.2, Gap::Door);
            }
        }
        // Two to four ways in, on any side.
        let entries = 2 + r.below(3);
        let mut made = 0;
        for _ in 0..24 {
            if made == entries {
                break;
            }
            let side = inner + r.below(4) as usize;
            made += place(&mut walls, &mut r, side, 1.4, Gap::Door) as u64;
        }
        // Windows along the outer walls.
        for side in inner..inner + 4 {
            let n = ((walls[side].to - walls[side].from) / 5.0) as u64;
            for _ in 0..n {
                if r.below(3) != 0 {
                    place(&mut walls, &mut r, side, 1.5, Gap::Window);
                }
            }
        }
        Killhouse::build((x_mm, y_mm), (w, h), &rooms, walls)
    }

    /// Cover and furniture inside the rooms (from `seed`): crates, tables, lockers and benches, kept clear of the
    /// doorways so the house can still be moved through.
    pub fn furnish(&self, seed: u64) -> Vec<Prop> {
        let mut r = Rand(seed ^ 0x51ce_77a1);
        let mut out: Vec<Prop> = Vec::new();
        for room in &self.rooms {
            let n = 1 + r.below(3);
            for _ in 0..n {
                let (kind, half) = match r.below(5) {
                    0 | 1 => {
                        let size = [900, 1_200][r.below(2) as usize];
                        (PropKind::Crate { size_mm: size }, (size / 2, size / 2))
                    }
                    2 => (PropKind::Furniture { item: crate::nav::Furnish::Table, width_mm: 1_600, depth_mm: 800 }, (800, 400)),
                    3 => (PropKind::Furniture { item: crate::nav::Furnish::Locker, width_mm: 1_000, depth_mm: 500 }, (500, 250)),
                    _ => (PropKind::Furniture { item: crate::nav::Furnish::Bench, width_mm: 1_600, depth_mm: 450 }, (800, 225)),
                };
                let turn = r.below(2) as u8;
                let (hx, hy) = if turn == 1 { (half.1, half.0) } else { half };
                for _ in 0..12 {
                    let (lx, ly) = (room.min.0 + hx + 600, room.min.1 + hy + 600);
                    let (ux, uy) = (room.max.0 - hx - 600, room.max.1 - hy - 600);
                    if ux <= lx || uy <= ly {
                        break;
                    }
                    let x = lx + r.below((ux - lx) as u64) as i64;
                    let y = ly + r.below((uy - ly) as u64) as i64;
                    let near_door = self.doors.iter().any(|d| (d.x_mm - x).abs() < 1_800 + hx && (d.y_mm - y).abs() < 1_800 + hy);
                    let crowded = out.iter().any(|p| (p.x_mm - x).abs() < 1_700 && (p.y_mm - y).abs() < 1_700);
                    if !near_door && !crowded {
                        out.push(Prop { kind, x_mm: x, y_mm: y, quarter_turns: turn });
                        break;
                    }
                }
            }
        }
        out
    }

    /// The panels to build it from: solid runs between openings, window panels across windows, gaps at doors.
    pub fn props(&self) -> Vec<Prop> {
        let o = self.origin;
        let mut out = Vec::new();
        for l in &self.walls {
            let mut gaps: Vec<(f64, f64, Gap)> = l.gaps.clone();
            gaps.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite"));
            let mut runs: Vec<(f64, f64, bool)> = Vec::new();
            let mut at = l.from;
            for &(a, b, kind) in &gaps {
                if a > at {
                    runs.push((at, a, false));
                }
                if kind == Gap::Window {
                    runs.push((a, b, true));
                }
                at = b;
            }
            if l.to > at {
                runs.push((at, l.to, false));
            }
            for (a, b, window) in runs {
                let mid = (a + b) / 2.0;
                let (x, y) = if l.along_x { (o.0 + mm(mid), o.1 + mm(l.at)) } else { (o.0 + mm(l.at), o.1 + mm(mid)) };
                out.push(Prop { kind: PropKind::Panel { length_mm: mm(b - a), window }, x_mm: x, y_mm: y, quarter_turns: if l.along_x { 0 } else { 1 } });
            }
        }
        out
    }

    /// Whether a point lies on one of the wall lines (within 1 mm).
    pub fn on_wall(&self, x: i64, y: i64) -> bool {
        let o = self.origin;
        self.walls.iter().any(|l| {
            let (fixed, along) = if l.along_x { (y - o.1, x - o.0) } else { (x - o.0, y - o.1) };
            (fixed - mm(l.at)).abs() <= 1 && along >= mm(l.from) && along <= mm(l.to)
        })
    }

    /// Which room a point is in.
    pub fn room_at(&self, x: i64, y: i64) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(x, y))
    }
}

/// A small seeded random source (splitmix64), so a layout is the same for the same seed everywhere.
struct Rand(u64);

impl Rand {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// 0..n (0 when n is 0).
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_house_is_unchanged() {
        let k = Killhouse::new(0, 0);
        assert_eq!((k.rooms.len(), k.doors.len(), k.windows.len()), (4, 7, 5));
        assert_eq!(k.size_m, SIZE_M);
    }

    #[test]
    fn generated_houses_are_sound() {
        for seed in 0..200 {
            let k = Killhouse::generate(10_000, 20_000, seed);
            let n = k.rooms.len();
            assert!((2..=12).contains(&n), "seed {seed}: {n} rooms");
            assert!(k.rooms.iter().all(|r| r.max.0 - r.min.0 >= 4_000 && r.max.1 - r.min.1 >= 4_000), "seed {seed}: a room under 4 m");
            let entries = k.doors.iter().filter(|d| d.rooms.0.is_none() || d.rooms.1.is_none()).count();
            assert!((2..=4).contains(&entries), "seed {seed}: {entries} ways in");
            // Every room reachable from outside through the doors.
            let mut seen = vec![false; n];
            let mut grow = true;
            while grow {
                grow = false;
                for d in &k.doors {
                    let reach = |r: Option<usize>, seen: &Vec<bool>| r.map_or(true, |i| seen[i]);
                    for (a, b) in [(d.rooms.0, d.rooms.1), (d.rooms.1, d.rooms.0)] {
                        if reach(a, &seen) {
                            if let Some(j) = b {
                                if !seen[j] {
                                    seen[j] = true;
                                    grow = true;
                                }
                            }
                        }
                    }
                }
            }
            assert!(seen.iter().all(|s| *s), "seed {seed}: a room cannot be reached");
            // Doors join two different places (not a doorway into the same room on both sides).
            assert!(k.doors.iter().all(|d| d.rooms.0 != d.rooms.1), "seed {seed}");
        }
    }

    #[test]
    fn different_seeds_give_different_houses_and_furniture_keeps_doorways_clear() {
        let (a, b) = (Killhouse::generate(0, 0, 1), Killhouse::generate(0, 0, 2));
        assert!(a.rooms != b.rooms || a.doors != b.doors);
        for seed in 0..50 {
            let k = Killhouse::generate(0, 0, seed);
            let f = k.furnish(seed);
            assert!(f.len() * 2 >= k.rooms.len(), "seed {seed}: furniture in most rooms");
            for p in &f {
                assert!(k.room_at(p.x_mm, p.y_mm).is_some(), "seed {seed}: furniture outside the rooms");
                assert!(k.doors.iter().all(|d| (d.x_mm - p.x_mm).abs() >= 1_800 || (d.y_mm - p.y_mm).abs() >= 1_800), "seed {seed}: furniture in a doorway");
            }
        }
    }
}
