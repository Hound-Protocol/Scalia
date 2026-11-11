//! The boarding layout a blueprint hands to `scalia-core`.

use crate::types::{Blueprint, Class};
use scalia_core::boarding::ShipLayout;

/// `Some` only for transports and ships; other vehicles carry no boarding layout.
pub fn ship_layout(b: &Blueprint) -> Option<ShipLayout> {
    match b.class {
        Class::Transport | Class::Ship => Some(ShipLayout {
            ramps: b.interior.ramps,
            doors: b.interior.doors,
            corridor_width_cm: b.interior.corridor_width_cm,
            seat_cols: b.interior.seat_cols,
        }),
        _ => None,
    }
}
