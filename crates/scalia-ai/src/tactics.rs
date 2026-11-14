//! The tactical map: where a soldier can take cover (generated along every wall, panel, sandbag line, crate and
//! rock), how well a spot protects against a given threat and whether he can fire from it; and how to go into a
//! building (stack points at each outside door, the order to clear its rooms). Integer millimetres.

use crate::sight::{isqrt, Point, SightGrid};
use scalia_core::killhouse::Killhouse;
use scalia_core::nav::{NavGrid, Prop, PropKind};

/// Low cover hides a crouching man (fire over it); high cover hides a standing man (lean out of a corner, or
/// shoot through a window in it).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Height {
    Low,
    High,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    pub pos: (i64, i64),
    /// Toward the cover (unit vector x1000): the way he faces to use it.
    pub toward: (i64, i64),
    pub height: Height,
    /// At the end of high cover: where he leans out to (a step past the end of the wall).
    pub corner: Option<(i64, i64)>,
    /// The prop it belongs to.
    pub prop: usize,
}

const SPACING_MM: i64 = 1_000;
const STANDOFF_MM: i64 = 550;
const CORNER_MM: i64 = 800;
const THREAT_EYE_MM: i64 = 1_600;
/// Eye height firing over low cover from a crouched aim.
const OVER_LOW_EYE_MM: i64 = 1_350;

/// Height of the prop (mm) as sight and cover see it (0: gives none).
fn top_of(p: &Prop) -> i64 {
    match p.kind {
        PropKind::Panel { .. } => 2_400,
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 650 => 2_750,
        PropKind::Wall { thickness_mm, .. } if thickness_mm <= 800 => 1_000,
        PropKind::Wall { .. } => 2_600,
        PropKind::Crate { size_mm } if size_mm >= 2_500 => 0,
        PropKind::Crate { size_mm } => size_mm,
        PropKind::Rock { radius_mm } => radius_mm * 9 / 10,
    }
}

/// Length along x and y halves of a rectangular prop.
fn halves(p: &Prop) -> Option<(i64, i64)> {
    let (l, t) = match p.kind {
        PropKind::Wall { length_mm, thickness_mm } => (length_mm, thickness_mm),
        PropKind::Panel { length_mm, .. } => (length_mm, PropKind::PANEL_MM),
        PropKind::Crate { size_mm } => (size_mm, size_mm),
        PropKind::Rock { .. } => return None,
    };
    Some(if p.quarter_turns % 2 == 0 { (l / 2, t / 2) } else { (t / 2, l / 2) })
}

pub struct CoverMap {
    pub spots: Vec<Cover>,
}

impl CoverMap {
    /// Appends the spots of a prop placed during a battle (index `k` in the map's props). Spots already handed
    /// out keep their indices.
    pub fn add_prop(&mut self, k: usize, p: &Prop, nav: &NavGrid) {
        let more = CoverMap::build(std::slice::from_ref(p), nav);
        self.spots.extend(more.spots.into_iter().map(|c| Cover { prop: k, ..c }));
    }

    pub fn build(props: &[Prop], nav: &NavGrid) -> CoverMap {
        let mut spots = Vec::new();
        for (k, p) in props.iter().enumerate() {
            let top = top_of(p);
            if top < 900 {
                continue;
            }
            let height = if top <= 1_300 { Height::Low } else { Height::High };
            let mut add = |pos: (i64, i64), toward: (i64, i64), corner: Option<(i64, i64)>| {
                if nav.walkable_at(pos.0, pos.1, 0) {
                    spots.push(Cover { pos, toward, height, corner: if height == Height::High { corner } else { None }, prop: k });
                }
            };
            match halves(p) {
                None => {
                    // A rock: a ring of spots around it.
                    let r = match p.kind { PropKind::Rock { radius_mm } => radius_mm, _ => 0 } + STANDOFF_MM;
                    for (ux, uy) in [(1_000, 0), (500, 866), (-500, 866), (-1_000, 0), (-500, -866), (500, -866)] {
                        add((p.x_mm + ux * r / 1_000, p.y_mm + uy * r / 1_000), (-ux, -uy), None);
                    }
                }
                Some((hx, hy)) => {
                    // Along both long faces (both faces of a square crate's sides too).
                    let faces: Vec<(bool, i64, i64)> = if hx >= hy { vec![(true, hx, hy)] } else { vec![(false, hy, hx)] };
                    for (along_x, half_len, half_thick) in faces {
                        let n = ((2 * half_len - SPACING_MM) / SPACING_MM).max(0) + 1;
                        for s in [-1i64, 1] {
                            for i in 0..n {
                                let a = -half_len + SPACING_MM / 2 + i * (2 * half_len - SPACING_MM) / (n - 1).max(1);
                                let a = if n == 1 { 0 } else { a };
                                let off = half_thick + STANDOFF_MM;
                                let (pos, toward) = if along_x { ((p.x_mm + a, p.y_mm + s * off), (0, -s * 1_000)) } else { ((p.x_mm + s * off, p.y_mm + a), (-s * 1_000, 0)) };
                                let end = if a > 0 { 1 } else { -1 };
                                let corner = if half_len - a.abs() <= CORNER_MM {
                                    let past = half_len + 500;
                                    Some(if along_x { (p.x_mm + end * past, pos.1) } else { (pos.0, p.y_mm + end * past) })
                                } else {
                                    None
                                };
                                add(pos, toward, corner);
                            }
                        }
                    }
                }
            }
        }
        CoverMap { spots }
    }

    /// Body points of the man in the spot (crouched behind low cover, standing behind high).
    fn body(c: &Cover) -> [i64; 3] {
        match c.height {
            Height::Low => [400, 750, 1_000],
            Height::High => [500, 1_150, 1_600],
        }
    }

    /// How many of his three body points the threat cannot see (0 exposed .. 3 fully hidden).
    pub fn protection(&self, c: &Cover, threat: (i64, i64), sight: &SightGrid) -> i64 {
        let eye = Point { x: threat.0, y: threat.1, z: THREAT_EYE_MM };
        Self::body(c).iter().filter(|&&z| !sight.clear(eye, Point { x: c.pos.0, y: c.pos.1, z })).count() as i64
    }

    /// Whether he can engage the threat from the spot: rising over low cover, leaning out of a corner, or
    /// through a window in it.
    pub fn can_fire(&self, c: &Cover, threat: (i64, i64), sight: &SightGrid) -> bool {
        let chest = Point { x: threat.0, y: threat.1, z: 1_150 };
        let from = |p: (i64, i64), z: i64| sight.clear(Point { x: p.0, y: p.1, z }, chest);
        match c.height {
            Height::Low => from(c.pos, OVER_LOW_EYE_MM),
            Height::High => from(c.pos, 1_500) || c.corner.is_some_and(|p| from(p, 1_500)),
        }
    }

    /// The best cover within `max_mm` of `me` against all `threats` (not one of `taken`): it must hide at
    /// least two of three body points from every threat; then the better protected, the one he can fire from,
    /// and the nearer win.
    pub fn best(&self, me: (i64, i64), threats: &[(i64, i64)], max_mm: i64, sight: &SightGrid, taken: &[usize]) -> Option<usize> {
        let mut best: Option<(i64, usize)> = None;
        for (i, c) in self.spots.iter().enumerate() {
            if taken.contains(&i) {
                continue;
            }
            let (dx, dy) = (c.pos.0 - me.0, c.pos.1 - me.1);
            let d = isqrt(dx * dx + dy * dy);
            if d > max_mm {
                continue;
            }
            let prot = threats.iter().map(|&t| self.protection(c, t, sight)).min().unwrap_or(3);
            if prot < 2 {
                continue;
            }
            let fire = threats.iter().any(|&t| self.can_fire(c, t, sight));
            let score = prot * 1_000 + if fire { 400 } else { 0 } - d / 10;
            if best.map_or(true, |(b, _)| score > b) {
                best = Some((score, i));
            }
        }
        best.map(|b| b.1)
    }
}

/// A way into a building: the outside door and where four men stack beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub door: usize,
    pub stack: Vec<(i64, i64)>,
}

/// How to take a building: its entries and how its rooms connect.
#[derive(Clone, Debug)]
pub struct Building {
    pub entries: Vec<Entry>,
    /// Inner doors as room pairs (index into the killhouse doors, rooms).
    links: Vec<(usize, usize)>,
    first: Vec<(usize, usize)>,
}

/// Between men in a door stack: just over the spacing two men keep, so each gets his own point.
pub const STACK_GAP_MM: i64 = 950;

impl Building {
    pub fn of(k: &Killhouse) -> Building {
        let mut entries = Vec::new();
        let mut links = Vec::new();
        let mut first = Vec::new();
        for (i, d) in k.doors.iter().enumerate() {
            match d.rooms {
                (Some(a), Some(b)) => links.push((a, b)),
                (inside, outside) => {
                    let room = inside.or(outside).expect("a door joins at least one room");
                    first.push((i, room));
                    // Outside is the side with no room; stack along the wall beside the door, a step off it.
                    let out = if inside.is_none() { -1 } else { 1 };
                    let (along, across) = if d.along_x { ((1, 0), (0, out)) } else { ((0, 1), (out, 0)) };
                    let stack = (0..4).map(|n| {
                        // Hugging the wall: out of sight of anyone inside looking out through the doorway.
                        let a = d.width_mm / 2 + 400 + n * STACK_GAP_MM;
                        (d.x_mm - along.0 * a + across.0 * 650, d.y_mm - along.1 * a + across.1 * 650)
                    }).collect();
                    entries.push(Entry { door: i, stack });
                }
            }
        }
        Building { entries, links, first }
    }

    /// The rooms in the order to clear them going in through outside door `door`: the room behind it first,
    /// then outward through the inner doors.
    pub fn clear_order(&self, door: usize) -> Vec<usize> {
        let Some(&(_, start)) = self.first.iter().find(|f| f.0 == door) else { return Vec::new() };
        let mut order = vec![start];
        let mut at = 0;
        while at < order.len() {
            let r = order[at];
            for &(a, b) in &self.links {
                let next = if a == r { b } else if b == r { a } else { continue };
                if !order.contains(&next) {
                    order.push(next);
                }
            }
            at += 1;
        }
        order
    }
}
