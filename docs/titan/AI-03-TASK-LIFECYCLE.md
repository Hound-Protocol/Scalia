# AI-03 — Titan Task Lifecycle

## Purpose

Introduce a deterministic task contract above squad execution. A task is an issued `leader::Order` with a stable identity, issuer, current assignee, priority, progress and explicit lifecycle state.

## Lifecycle

`Assigned → Acknowledged → Executing → Completed`

Execution may move through:

- `Interrupted`: temporarily displaced by an urgent response; `resume` returns it to execution.
- `Blocked`: execution cannot proceed; `resume` returns it to execution.
- `Failed`: terminal outcome with a reason.
- `Cancelled`: terminal outcome.

Any non-terminal task may be reassigned. Reassignment returns it to `Assigned`, requires a fresh acknowledgement, and preserves the objective and reported progress. Completed, failed and cancelled tasks cannot be mutated.

Progress is integer permille (0–1000) and cannot move backwards. Events are append-only, have monotonically increasing sequence numbers, and require nondecreasing simulation timestamps. Ordered maps make task iteration deterministic.

## Implementation

Implemented in `crates/scalia-ai/src/titan_tasks.rs` as `TitanTaskBoard`.

- Stable monotonically allocated `TaskId`.
- Issuer and assignee use the AI-01 `CommandNodeId`.
- Task payload reuses `leader::Order`; no duplicate order vocabulary.
- Explicit acknowledgement, start, progress, interrupt, block, resume, complete, fail, cancel and reassign operations.
- Invalid transitions, terminal mutation, invalid progress and time reversal are rejected.
- Append-only transition events retain interruption, blockage and failure reasons.
- Five unit tests cover the normal lifecycle, interruption/resumption, reassignment, invalid transitions and terminal outcomes.

## Integration boundary

This work order deliberately does **not** change `Battle::submit`, `Battle::apply_commands`, `Battle::history`, `Battle::save/load`, or squad `Brain` execution. The existing battle queue already records applied commands for replay, while the save `State` serializes its queue and history. Adding task records there without a save-format migration and replay contract would risk compatibility.

The task board is therefore a tested, serializable lifecycle model, not yet the live authority for battle orders. The next integration step must define how task events enter battle history, how task IDs survive save/load, and how squad Brain completion/failure is reported back without duplicating tactical state.