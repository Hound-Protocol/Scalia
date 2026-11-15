//! Deterministic task lifecycle for Titan command.
//!
//! This is an additive contract. Battle's existing command queue and squad Brain remain
//! the execution authority until a later integration work order can preserve replay/save semantics.

use crate::leader::Order;
use crate::titan::CommandNodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Assigned,
    Acknowledged,
    Executing,
    Interrupted,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitanTask {
    pub id: TaskId,
    pub issuer: CommandNodeId,
    pub assignee: CommandNodeId,
    pub order: Order,
    pub priority: i16,
    pub state: TaskState,
    /// 0..=1000. Progress is preserved when a task is interrupted or reassigned.
    pub progress_permille: u16,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskEventKind {
    Assigned { assignee: CommandNodeId },
    Acknowledged,
    Started,
    Progress { permille: u16 },
    Interrupted { reason: String },
    Blocked { reason: String },
    Resumed,
    Completed,
    Failed { reason: String },
    Cancelled,
    Reassigned { from: CommandNodeId, to: CommandNodeId },
}

/// Append-only, ordered transition record. Sequence is stable for replay and diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub sequence: u64,
    pub at_ms: i64,
    pub task_id: TaskId,
    pub kind: TaskEventKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskError {
    MissingTask,
    InvalidTransition,
    TerminalTask,
    TimeWentBackwards,
    InvalidProgress,
    IdExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitanTaskBoard {
    next_id: u64,
    next_sequence: u64,
    last_event_ms: Option<i64>,
    tasks: BTreeMap<TaskId, TitanTask>,
    events: Vec<TaskEvent>,
}

impl Default for TitanTaskBoard {
    fn default() -> Self {
        Self {
            next_id: 1,
            next_sequence: 0,
            last_event_ms: None,
            tasks: BTreeMap::new(),
            events: Vec::new(),
        }
    }
}

impl TitanTaskBoard {
    pub fn issue(
        &mut self,
        issuer: CommandNodeId,
        assignee: CommandNodeId,
        order: Order,
        priority: i16,
        at_ms: i64,
    ) -> Result<TaskId, TaskError> {
        self.check_time(at_ms)?;
        let id = TaskId(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or(TaskError::IdExhausted)?;
        self.tasks.insert(id, TitanTask {
            id, issuer, assignee, order, priority, state: TaskState::Assigned,
            progress_permille: 0, updated_at_ms: at_ms,
        });
        self.push_event(at_ms, id, TaskEventKind::Assigned { assignee })?;
        Ok(id)
    }

    pub fn get(&self, id: TaskId) -> Option<&TitanTask> {
        self.tasks.get(&id)
    }

    pub fn events(&self) -> &[TaskEvent] {
        &self.events
    }

    pub fn iter(&self) -> impl Iterator<Item = &TitanTask> {
        self.tasks.values()
    }

    pub fn acknowledge(&mut self, id: TaskId, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Assigned], TaskState::Acknowledged, TaskEventKind::Acknowledged)
    }

    pub fn start(&mut self, id: TaskId, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Acknowledged], TaskState::Executing, TaskEventKind::Started)
    }

    pub fn report_progress(&mut self, id: TaskId, permille: u16, at_ms: i64) -> Result<(), TaskError> {
        self.check_time(at_ms)?;
        if permille > 1_000 {
            return Err(TaskError::InvalidProgress);
        }
        let task = self.tasks.get_mut(&id).ok_or(TaskError::MissingTask)?;
        if task.state != TaskState::Executing || permille < task.progress_permille {
            return Err(TaskError::InvalidTransition);
        }
        task.progress_permille = permille;
        task.updated_at_ms = at_ms;
        self.push_event(at_ms, id, TaskEventKind::Progress { permille })
    }

    pub fn interrupt(&mut self, id: TaskId, reason: impl Into<String>, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Executing], TaskState::Interrupted,
            TaskEventKind::Interrupted { reason: reason.into() })
    }

    pub fn block(&mut self, id: TaskId, reason: impl Into<String>, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Executing], TaskState::Blocked,
            TaskEventKind::Blocked { reason: reason.into() })
    }

    pub fn resume(&mut self, id: TaskId, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Interrupted, TaskState::Blocked], TaskState::Executing, TaskEventKind::Resumed)
    }

    pub fn complete(&mut self, id: TaskId, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms, &[TaskState::Executing], TaskState::Completed, TaskEventKind::Completed)
    }

    pub fn fail(&mut self, id: TaskId, reason: impl Into<String>, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms,
            &[TaskState::Assigned, TaskState::Acknowledged, TaskState::Executing, TaskState::Interrupted, TaskState::Blocked],
            TaskState::Failed, TaskEventKind::Failed { reason: reason.into() })
    }

    pub fn cancel(&mut self, id: TaskId, at_ms: i64) -> Result<(), TaskError> {
        self.transition(id, at_ms,
            &[TaskState::Assigned, TaskState::Acknowledged, TaskState::Executing, TaskState::Interrupted, TaskState::Blocked],
            TaskState::Cancelled, TaskEventKind::Cancelled)
    }

    /// Reassigns any non-terminal task. Its objective and progress are retained; acknowledgement is required again.
    pub fn reassign(&mut self, id: TaskId, assignee: CommandNodeId, at_ms: i64) -> Result<(), TaskError> {
        self.check_time(at_ms)?;
        let task = self.tasks.get_mut(&id).ok_or(TaskError::MissingTask)?;
        if task.state.is_terminal() {
            return Err(TaskError::TerminalTask);
        }
        let from = task.assignee;
        task.assignee = assignee;
        task.state = TaskState::Assigned;
        task.updated_at_ms = at_ms;
        self.push_event(at_ms, id, TaskEventKind::Reassigned { from, to: assignee })
    }

    fn transition(
        &mut self,
        id: TaskId,
        at_ms: i64,
        allowed: &[TaskState],
        next: TaskState,
        event: TaskEventKind,
    ) -> Result<(), TaskError> {
        self.check_time(at_ms)?;
        let task = self.tasks.get_mut(&id).ok_or(TaskError::MissingTask)?;
        if task.state.is_terminal() {
            return Err(TaskError::TerminalTask);
        }
        if !allowed.contains(&task.state) {
            return Err(TaskError::InvalidTransition);
        }
        task.state = next;
        task.updated_at_ms = at_ms;
        self.push_event(at_ms, id, event)
    }

    fn check_time(&self, at_ms: i64) -> Result<(), TaskError> {
        if at_ms < 0 || self.last_event_ms.is_some_and(|last| at_ms < last) {
            Err(TaskError::TimeWentBackwards)
        } else {
            Ok(())
        }
    }

    fn push_event(&mut self, at_ms: i64, task_id: TaskId, kind: TaskEventKind) -> Result<(), TaskError> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.checked_add(1).ok_or(TaskError::IdExhausted)?;
        self.last_event_ms = Some(at_ms);
        self.events.push(TaskEvent { sequence, at_ms, task_id, kind });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: CommandNodeId = CommandNodeId(1);
    const SQUAD_A: CommandNodeId = CommandNodeId(2);
    const SQUAD_B: CommandNodeId = CommandNodeId(3);

    fn issue(board: &mut TitanTaskBoard) -> TaskId {
        board.issue(ISSUER, SQUAD_A, Order::Hold, 5, 100).unwrap()
    }

    #[test]
    fn lifecycle_records_assignment_acknowledgement_execution_and_completion() {
        let mut board = TitanTaskBoard::default();
        let id = issue(&mut board);
        board.acknowledge(id, 150).unwrap();
        board.start(id, 200).unwrap();
        board.report_progress(id, 600, 250).unwrap();
        board.complete(id, 300).unwrap();

        let task = board.get(id).unwrap();
        assert_eq!(task.state, TaskState::Completed);
        assert_eq!(task.progress_permille, 600);
        assert_eq!(board.events().iter().map(|e| e.sequence).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn interruption_and_blockage_resume_without_losing_progress() {
        let mut board = TitanTaskBoard::default();
        let id = issue(&mut board);
        board.acknowledge(id, 110).unwrap();
        board.start(id, 120).unwrap();
        board.report_progress(id, 350, 130).unwrap();
        board.interrupt(id, "casualty response", 140).unwrap();
        assert_eq!(board.get(id).unwrap().state, TaskState::Interrupted);
        board.resume(id, 150).unwrap();
        board.block(id, "route obstructed", 160).unwrap();
        board.resume(id, 170).unwrap();
        assert_eq!(board.get(id).unwrap().progress_permille, 350);
        assert_eq!(board.get(id).unwrap().state, TaskState::Executing);
    }

    #[test]
    fn reassignment_requires_fresh_acknowledgement_and_preserves_progress() {
        let mut board = TitanTaskBoard::default();
        let id = issue(&mut board);
        board.acknowledge(id, 110).unwrap();
        board.start(id, 120).unwrap();
        board.report_progress(id, 400, 130).unwrap();
        board.reassign(id, SQUAD_B, 140).unwrap();

        let task = board.get(id).unwrap();
        assert_eq!(task.assignee, SQUAD_B);
        assert_eq!(task.state, TaskState::Assigned);
        assert_eq!(task.progress_permille, 400);
        assert_eq!(board.events().last().unwrap().kind,
            TaskEventKind::Reassigned { from: SQUAD_A, to: SQUAD_B });
    }

    #[test]
    fn invalid_transitions_terminal_mutation_and_time_reversal_are_rejected() {
        let mut board = TitanTaskBoard::default();
        let id = issue(&mut board);
        assert_eq!(board.start(id, 110), Err(TaskError::InvalidTransition));
        board.acknowledge(id, 120).unwrap();
        board.start(id, 130).unwrap();
        board.complete(id, 140).unwrap();
        assert_eq!(board.cancel(id, 150), Err(TaskError::TerminalTask));
        assert_eq!(board.report_progress(id, 1_001, 150), Err(TaskError::InvalidProgress));
        assert_eq!(board.issue(ISSUER, SQUAD_A, Order::Hold, 1, 139), Err(TaskError::TimeWentBackwards));
    }

    #[test]
    fn failure_and_cancellation_are_terminal() {
        let mut board = TitanTaskBoard::default();
        let failed = issue(&mut board);
        board.fail(failed, "no viable route", 110).unwrap();
        assert_eq!(board.get(failed).unwrap().state, TaskState::Failed);

        let cancelled = board.issue(ISSUER, SQUAD_A, Order::Hold, 1, 120).unwrap();
        board.cancel(cancelled, 130).unwrap();
        assert_eq!(board.get(cancelled).unwrap().state, TaskState::Cancelled);
    }
}
