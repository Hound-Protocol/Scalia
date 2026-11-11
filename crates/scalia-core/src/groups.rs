//! Struct-of-arrays store of groups (companies and above).

use crate::position::Pos;
use crate::process::Process;
use crate::roster::Roster;
use crate::units::{Composition, Needs};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct GroupId(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct GroupSpec {
    pub pos: Pos,
    pub composition: Composition,
    pub needs: Needs,
}

/// Largest group `try_spawn` accepts (80 MB of roster at 8 bytes a soldier).
pub const MAX_GROUP_PEOPLE: u32 = 10_000_000;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SpawnError {
    TooLarge,
}

#[derive(Default)]
pub struct Groups {
    pos: Vec<Pos>,
    composition: Vec<Composition>,
    needs: Vec<Needs>,
    roster_start: Vec<u32>,
    roster_len: Vec<u32>,
    process: Vec<Process>,
}

impl Groups {
    pub fn new() -> Groups {
        Groups::default()
    }

    /// Panics with a clear message when the group is larger than `MAX_GROUP_PEOPLE`.
    pub fn spawn(&mut self, spec: GroupSpec, roster: &mut Roster) -> GroupId {
        self.try_spawn(spec, roster).expect("group larger than MAX_GROUP_PEOPLE")
    }

    /// Like `spawn`, but refuses a group whose total count exceeds `MAX_GROUP_PEOPLE`
    /// and leaves both stores untouched.
    pub fn try_spawn(&mut self, spec: GroupSpec, roster: &mut Roster) -> Result<GroupId, SpawnError> {
        if spec.composition.total() > MAX_GROUP_PEOPLE {
            return Err(SpawnError::TooLarge);
        }
        let id = GroupId(self.pos.len() as u32);
        let people = spec.composition.people();
        let start = roster.alloc(people);
        self.pos.push(spec.pos);
        self.composition.push(spec.composition);
        self.needs.push(spec.needs);
        self.roster_start.push(start);
        self.roster_len.push(people);
        self.process.push(Process::idle());
        Ok(id)
    }

    pub fn len(&self) -> usize {
        self.pos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pos.is_empty()
    }

    pub fn pos(&self, id: GroupId) -> Pos {
        self.pos[id.0 as usize]
    }

    pub fn composition(&self, id: GroupId) -> &Composition {
        &self.composition[id.0 as usize]
    }

    pub fn needs(&self, id: GroupId) -> &Needs {
        &self.needs[id.0 as usize]
    }

    /// `(start, length)` of the group's slice in the roster.
    pub fn roster_range(&self, id: GroupId) -> (u32, u32) {
        (self.roster_start[id.0 as usize], self.roster_len[id.0 as usize])
    }

    pub fn set_pos(&mut self, id: GroupId, p: Pos) {
        self.pos[id.0 as usize] = p;
    }

    /// All needs, in group order, for one linear pass.
    pub fn needs_all_mut(&mut self) -> &mut [Needs] {
        &mut self.needs
    }

    pub fn process(&self, id: GroupId) -> &Process {
        &self.process[id.0 as usize]
    }

    pub fn set_process(&mut self, id: GroupId, p: Process) {
        self.process[id.0 as usize] = p;
    }

    pub fn people(&self, id: GroupId) -> u32 {
        self.roster_len[id.0 as usize]
    }
}
