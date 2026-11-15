//! Titan Command: deterministic ownership and succession for the command hierarchy.
//!
//! This is the command-authority layer. Tactical execution remains in `leader::Brain`;
//! Titan records who is authorised to issue a squad's existing `Order`.

use crate::leader::Order;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CommandNodeId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRole {
    Commander,
    Operations,
    SquadLeader,
}

impl CommandRole {
    fn may_own_squad(self) -> bool {
        matches!(self, Self::Operations | Self::SquadLeader)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandNode {
    pub id: CommandNodeId,
    pub parent: Option<CommandNodeId>,
    pub role: CommandRole,
    pub active: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadCommand {
    pub owner: CommandNodeId,
    pub order: Order,
}

/// Authority registry. BTreeMap ordering keeps succession deterministic across platforms.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitanCommand {
    nodes: BTreeMap<CommandNodeId, CommandNode>,
    squads: BTreeMap<u16, SquadCommand>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    DuplicateNode,
    MissingNode,
    MissingParent,
    InactiveNode,
    InvalidOwnerRole,
    SelfParent,
}

impl TitanCommand {
    pub fn register(&mut self, node: CommandNode) -> Result<(), CommandError> {
        if self.nodes.contains_key(&node.id) {
            return Err(CommandError::DuplicateNode);
        }
        if node.parent == Some(node.id) {
            return Err(CommandError::SelfParent);
        }
        if let Some(parent) = node.parent {
            if !self.nodes.contains_key(&parent) {
                return Err(CommandError::MissingParent);
            }
        }
        self.nodes.insert(node.id, node);
        Ok(())
    }

    pub fn node(&self, id: CommandNodeId) -> Option<&CommandNode> {
        self.nodes.get(&id)
    }

    pub fn squad_command(&self, squad: u16) -> Option<SquadCommand> {
        self.squads.get(&squad).copied()
    }

    pub fn assign_squad(
        &mut self,
        squad: u16,
        owner: CommandNodeId,
        order: Order,
    ) -> Result<(), CommandError> {
        let node = self.nodes.get(&owner).ok_or(CommandError::MissingNode)?;
        if !node.active {
            return Err(CommandError::InactiveNode);
        }
        if !node.role.may_own_squad() {
            return Err(CommandError::InvalidOwnerRole);
        }
        self.squads.insert(squad, SquadCommand { owner, order });
        Ok(())
    }

    /// Transfers all squads owned by an inactive node to its nearest active ancestor
    /// with squad authority. If none exists, ownership is cleared rather than guessed.
    pub fn deactivate_and_succeed(&mut self, id: CommandNodeId) -> Result<(), CommandError> {
        let node = self.nodes.get_mut(&id).ok_or(CommandError::MissingNode)?;
        node.active = false;

        let mut ancestor = node.parent;
        let successor = loop {
            let Some(candidate) = ancestor else { break None };
            let Some(parent) = self.nodes.get(&candidate) else { break None };
            if parent.active && parent.role.may_own_squad() {
                break Some(candidate);
            }
            ancestor = parent.parent;
        };

        self.squads.retain(|_, command| {
            if command.owner != id {
                return true;
            }
            if let Some(owner) = successor {
                command.owner = owner;
                true
            } else {
                false
            }
        });
        Ok(())
    }

    pub fn transfer_squad(
        &mut self,
        squad: u16,
        new_owner: CommandNodeId,
    ) -> Result<(), CommandError> {
        let node = self.nodes.get(&new_owner).ok_or(CommandError::MissingNode)?;
        if !node.active {
            return Err(CommandError::InactiveNode);
        }
        if !node.role.may_own_squad() {
            return Err(CommandError::InvalidOwnerRole);
        }
        let command = self.squads.get_mut(&squad).ok_or(CommandError::MissingNode)?;
        command.owner = new_owner;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, parent: Option<u32>, role: CommandRole) -> CommandNode {
        CommandNode {
            id: CommandNodeId(id),
            parent: parent.map(CommandNodeId),
            role,
            active: true,
        }
    }

    #[test]
    fn rejects_missing_parent_and_duplicate_ids() {
        let mut titan = TitanCommand::default();
        assert_eq!(
            titan.register(node(2, Some(1), CommandRole::SquadLeader)),
            Err(CommandError::MissingParent)
        );
        titan.register(node(1, None, CommandRole::Operations)).unwrap();
        assert_eq!(
            titan.register(node(1, None, CommandRole::Operations)),
            Err(CommandError::DuplicateNode)
        );
    }

    #[test]
    fn only_active_authorised_nodes_can_own_squads() {
        let mut titan = TitanCommand::default();
        titan.register(node(1, None, CommandRole::Commander)).unwrap();
        assert_eq!(
            titan.assign_squad(4, CommandNodeId(1), Order::Hold),
            Err(CommandError::InvalidOwnerRole)
        );
        titan.register(node(2, Some(1), CommandRole::SquadLeader)).unwrap();
        titan.assign_squad(4, CommandNodeId(2), Order::Hold).unwrap();
        titan.deactivate_and_succeed(CommandNodeId(2)).unwrap();
        assert_eq!(titan.squad_command(4), None);
    }

    #[test]
    fn succession_uses_nearest_active_authorised_ancestor() {
        let mut titan = TitanCommand::default();
        titan.register(node(1, None, CommandRole::Operations)).unwrap();
        titan.register(node(2, Some(1), CommandRole::SquadLeader)).unwrap();
        titan.register(node(3, Some(2), CommandRole::SquadLeader)).unwrap();
        titan.assign_squad(7, CommandNodeId(3), Order::Capture).unwrap();

        titan.deactivate_and_succeed(CommandNodeId(3)).unwrap();
        assert_eq!(
            titan.squad_command(7),
            Some(SquadCommand { owner: CommandNodeId(2), order: Order::Capture })
        );
        titan.deactivate_and_succeed(CommandNodeId(2)).unwrap();
        assert_eq!(
            titan.squad_command(7),
            Some(SquadCommand { owner: CommandNodeId(1), order: Order::Capture })
        );
    }
}
