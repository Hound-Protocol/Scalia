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

#[derive(Copy, Clone, PartialEq, Eq)]
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

pub struct Killhouse {
    /// South-west corner (absolute millimetres).
    pub origin: (i64, i64),
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub windows: Vec<Opening>,
}

fn mm(m: f64) -> i64 {
    (m * 1000.0).round() as i64
}

impl Killhouse {
    pub fn new(x_mm: i64, y_mm: i64) -> Killhouse {
        let o = (x_mm, y_mm);
        let rooms: Vec<Room> = ROOMS.iter().map(|&(x0, y0, x1, y1)| Room { min: (o.0 + mm(x0), o.1 + mm(y0)), max: (o.0 + mm(x1), o.1 + mm(y1)) }).collect();
        let mut doors = Vec::new();
        let mut windows = Vec::new();
        for l in &LINES {
            for &(a, b, kind) in l.gaps {
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
        Killhouse { origin: o, rooms, doors, windows }
    }

    /// The panels to build it from: solid runs between openings, window panels across windows, gaps at doors.
    pub fn props(&self) -> Vec<Prop> {
        let o = self.origin;
        let mut out = Vec::new();
        for l in &LINES {
            let mut gaps: Vec<(f64, f64, Gap)> = l.gaps.to_vec();
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
        LINES.iter().any(|l| {
            let (fixed, along) = if l.along_x { (y - o.1, x - o.0) } else { (x - o.0, y - o.1) };
            (fixed - mm(l.at)).abs() <= 1 && along >= mm(l.from) && along <= mm(l.to)
        })
    }

    /// Which room a point is in.
    pub fn room_at(&self, x: i64, y: i64) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(x, y))
    }
}
