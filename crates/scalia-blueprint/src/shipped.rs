//! The blueprints shipped with the game, embedded at build time.

use crate::types::Blueprint;
use crate::{parse, BlueprintError};

/// File text of each shipped blueprint, in a fixed order.
pub const SHIPPED_RON: [&str; 6] = [
    include_str!("../../../blueprints/trooper_light.ron"),
    include_str!("../../../blueprints/trooper_medium.ron"),
    include_str!("../../../blueprints/trooper_heavy.ron"),
    include_str!("../../../blueprints/tank_bulwark.ron"),
    include_str!("../../../blueprints/truck_hauler.ron"),
    include_str!("../../../blueprints/transport_ox.ron"),
];

/// Parse and validate every shipped blueprint; ids must be unique.
pub fn shipped() -> Result<Vec<Blueprint>, BlueprintError> {
    let mut out: Vec<Blueprint> = Vec::with_capacity(SHIPPED_RON.len());
    for text in SHIPPED_RON {
        let b = parse(text)?;
        if out.iter().any(|o| o.id == b.id) {
            return Err(BlueprintError { field: "id".into(), message: format!("duplicate id {}", b.id) });
        }
        out.push(b);
    }
    Ok(out)
}
