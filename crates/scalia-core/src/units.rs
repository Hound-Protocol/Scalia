//! Unit kinds, group composition and per-group needs.

use crate::fixed::Fix;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UnitType {
    Infantry = 0,
    Medic = 1,
    Tank = 2,
    Truck = 3,
}

/// Counts per `UnitType`, indexed by `UnitType as usize`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Composition(pub [u32; 4]);

impl Composition {
    /// Soldiers who walk and sleep: infantry plus medics.
    pub fn people(&self) -> u32 {
        self.0[UnitType::Infantry as usize].saturating_add(self.0[UnitType::Medic as usize])
    }

    pub fn total(&self) -> u32 {
        self.0.iter().fold(0u32, |a, &n| a.saturating_add(n))
    }
}

/// Four separate meters, each in `[0, 1]`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Needs {
    pub food: Fix,
    pub sleep: Fix,
    pub morale: Fix,
    pub upkeep: Fix,
}

impl Needs {
    pub fn full() -> Needs {
        Needs { food: Fix::ONE, sleep: Fix::ONE, morale: Fix::ONE, upkeep: Fix::ONE }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Health {
    Healthy = 0,
    Wounded = 1,
    Dead = 2,
}
