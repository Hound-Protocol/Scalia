//! Compact per-soldier records, one contiguous slice per group.

use crate::units::Health;

pub const FLAG_PERSISTENT: u8 = 1;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct SoldierRec {
    pub xp: u16,
    pub kills: u16,
    pub health: Health,
    pub flags: u8,
    pub reserved: u16,
}

impl SoldierRec {
    pub const NEW: SoldierRec = SoldierRec {
        xp: 0,
        kills: 0,
        health: Health::Healthy,
        flags: 0,
        reserved: 0,
    };
}

#[derive(Default)]
pub struct Roster {
    recs: Vec<SoldierRec>,
}

impl Roster {
    pub fn new() -> Roster {
        Roster { recs: Vec::new() }
    }

    /// Appends `n` healthy records and returns the start index.
    pub fn alloc(&mut self, n: u32) -> u32 {
        let start = self.recs.len() as u32;
        self.recs
            .resize(self.recs.len() + n as usize, SoldierRec::NEW);
        start
    }

    pub fn rec(&self, start: u32, slot: u32) -> &SoldierRec {
        &self.recs[(start + slot) as usize]
    }

    pub fn rec_mut(&mut self, start: u32, slot: u32) -> &mut SoldierRec {
        &mut self.recs[(start + slot) as usize]
    }

    pub fn len(&self) -> usize {
        self.recs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.recs.is_empty()
    }
}
