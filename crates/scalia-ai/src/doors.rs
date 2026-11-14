//! Doors in a battle: where they hang, their state, and what that state does. A shut door (closed or locked)
//! blocks sight up to its height; a locked one blocks walking (a closed one is opened by whoever walks into it);
//! open and breached doors are nothing in the way. Integer millimetres.

use crate::sight::Point;
use scalia_core::nav::{Prop, PropKind};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorState {
    Open,
    Closed,
    /// Closed and cannot be opened, only breached.
    Locked,
    /// Forced open for good.
    Breached,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Door {
    /// The middle of the doorway.
    pub at: (i64, i64),
    /// Whether the wall it hangs in runs along x.
    pub along_x: bool,
    pub width_mm: i64,
    pub height_mm: i64,
    pub state: DoorState,
    /// Whether it can be locked.
    pub lockable: bool,
    /// When its state last changed (for the swing, in presentation).
    pub changed_ms: i64,
    /// Who last opened or closed it (a soldier), if anyone.
    pub by: Option<u32>,
}

/// How long it takes a man to open a door (the latch, the push).
pub const OPEN_MS: i64 = 600;
/// How long forcing a locked door takes (a kick, a shoulder, a ram).
pub const BREACH_MS: i64 = 1_800;

impl Door {
    pub fn new(at: (i64, i64), along_x: bool, width_mm: i64, height_mm: i64, state: DoorState) -> Door {
        Door { at, along_x, width_mm, height_mm, state, lockable: true, changed_ms: 0, by: None }
    }

    /// Whether it bars the view (and, unless opened, the way).
    pub fn shut(&self) -> bool {
        matches!(self.state, DoorState::Closed | DoorState::Locked)
    }

    /// The doorway's line on the ground plan (end to end).
    pub fn ends(&self) -> ((i64, i64), (i64, i64)) {
        let h = self.width_mm / 2;
        if self.along_x {
            ((self.at.0 - h, self.at.1), (self.at.0 + h, self.at.1))
        } else {
            ((self.at.0, self.at.1 - h), (self.at.0, self.at.1 + h))
        }
    }

    /// Where the line from `a` to `b` crosses the doorway (as the fraction t/den along a→b), if it does.
    fn crossing(&self, a: (i64, i64), b: (i64, i64)) -> Option<(i128, i128)> {
        let (p, q) = self.ends();
        let r = ((b.0 - a.0) as i128, (b.1 - a.1) as i128);
        let s = ((q.0 - p.0) as i128, (q.1 - p.1) as i128);
        let den = r.0 * s.1 - r.1 * s.0;
        if den == 0 {
            return None;
        }
        let w = ((p.0 - a.0) as i128, (p.1 - a.1) as i128);
        let (t, u) = (w.0 * s.1 - w.1 * s.0, w.0 * r.1 - w.1 * r.0);
        let (t, u, den) = if den < 0 { (-t, -u, -den) } else { (t, u, den) };
        ((0..=den).contains(&t) && (0..=den).contains(&u)).then_some((t, den))
    }

    /// Whether a man walking from `a` to `b` goes through the doorway.
    pub fn crossed_by(&self, a: (i64, i64), b: (i64, i64)) -> bool {
        self.crossing(a, b).is_some()
    }

    /// Whether it blocks the line of sight from `a` to `b` (shut, and the line crosses it below its top).
    pub fn blocks(&self, a: Point, b: Point) -> bool {
        if !self.shut() {
            return false;
        }
        let Some((t, den)) = self.crossing((a.x, a.y), (b.x, b.y)) else { return false };
        let z = a.z as i128 + (b.z - a.z) as i128 * t / den;
        z < self.height_mm as i128
    }

    /// A locked door as an obstacle for the walking grid (a wall across the doorway).
    pub fn as_prop(&self) -> Prop {
        Prop { kind: PropKind::Wall { length_mm: self.width_mm, thickness_mm: 100 }, x_mm: self.at.0, y_mm: self.at.1, quarter_turns: if self.along_x { 0 } else { 1 } }
    }
}
