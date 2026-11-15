# Titan System

**Status:** Architecture definition + AI-01 command registry foundation  
**Scope:** Unified AI architecture for Scalia  
**Current implementation:** `scalia-ai` crate

Titan System is the umbrella architecture for all simulation AI: strategic command, squad leadership, individual soldier behaviour, battlefield intelligence, navigation and deterministic execution.

Titan is a logical system boundary, not an instruction to split the current crate into multiple crates. Existing modules remain in place until a measured need justifies restructuring.

## Architecture layers

| Layer | Owns | Does not own |
|---|---|---|
| Titan Command | Player intent, command hierarchy, priorities, tasking subordinate formations | Individual movement or weapon execution |
| Titan Operations | Formation-level coordination, assigning objectives and support relationships | Soldier-level reactions |
| Titan Squad | Squad plans, formations, tactical execution, casualty response and local decisions | Company-wide priorities |
| Titan Soldier | Local perception, survival, engagement, reload and execution of assigned tasks | Rewriting strategic orders |
| Titan Intelligence | Contact reports, confidence, age, source, threat picture and information distribution | Issuing movement orders |
| Titan Navigation | Path requests, route validation, obstacle and stuck recovery | Tactical goal selection |
| Titan Core | Shared state contracts, deterministic scheduling, event flow and save compatibility | Tactical policy |

### Decision ownership

1. Higher command sets intent and constraints.
2. Subordinate command translates intent into achievable tasks.
3. Squad AI chooses and executes local tactics within those constraints.
4. Soldier AI may interrupt execution for immediate survival or local combat needs.
5. Interruptions are reported upward; they do not silently erase the higher-level intent.
6. Intelligence informs every layer through explicit, time-stamped observations.

## Current implementation audit (AI-01)

### Present

- `Battle::command(squad, Order)` is the public command entry point.
- `Order` currently supports `None`, `Capture`, `Hold` and `Move { to, mode }`.
- Each squad has a `Brain` containing its order, leader, plan and execution state.
- `Battle::lead()` updates squad brains on the leader schedule.
- Squad leader succession exists through `keep_leader` / `leader_of`.
- Squad execution includes movement, caution, confirmed-contact bounding, garrisoning, building assault, casualty care, regrouping and withdrawal.
- Individual soldiers have a separate GOAP-driven `mind` for survival, engagement, reload and order-following.
- Squad-level contacts and confidence are present.
- Navigation and deterministic battle stepping are implemented.

### Missing or not represented in the current command model

- No explicit company, platoon or multi-squad command entity.
- No persistent hierarchy above a squad brain.
- No subordinate assignment, delegation or command succession above squad level.
- No formal task lifecycle (assign, acknowledge, execute, report, cancel, reassign).
- No general formation-level coordination contract for support, reserve, fire support or mutual support.
- No explicit distinction between commander intent, squad task and soldier task in the public command model.
- No standard plan-failure / replan / escalate protocol shared across command levels.

### Questionable / technical debt

- `squad.rs` explicitly describes its action facts and costs as placeholders and an example domain. It must not be mistaken for the operational squad planner.
- `leader.rs` combines command representation, squad planning and substantial tactical execution. This is functional but creates a large ownership boundary.
- `Battle::command` replaces the squad `Brain` when a new order arrives. This preserves kit and active casualty care, but resets other plan state. Order replacement semantics need explicit tests and policy.
- The command model is squad-index based; it does not encode formation identity, parent command, role or reporting relationship.
- Soldier autonomy is separate from squad planning. Its interruption priorities are explicit, but the contract for resuming the squad's intent should be documented and tested.
- Squad contact lists are useful local intelligence, but they are not yet a formal cross-squad / formation intelligence service.
- The current code supports several strong tactical behaviours, but their existence does not establish coordinated multi-squad operations.

## AI-01 decision

**Do not refactor or rename existing modules yet.** First establish the command ownership model and compatibility rules. Keep `scalia-ai` as the implementation crate; expose Titan as the documented architecture and use Titan-prefixed module/API names only when new boundaries are introduced.

### Proposed command hierarchy

`Player / Commander → Operations Command → Squad Leader → Soldier`

Operations Command is the first formation-level layer. It can later represent platoon/company command through configurable hierarchy depth; do not hard-code military echelons before the game requires them.

A command node should minimally own:

- Stable ID and parent ID.
- Subordinate IDs.
- Role / echelon.
- Current intent and constraints.
- Assigned objective / task.
- Status and last report time.
- Succession policy.

A task should minimally carry:

- Stable task ID and issuer.
- Objective and destination / area where relevant.
- Priority and constraints.
- State: proposed, assigned, acknowledged, executing, blocked, completed, cancelled.
- Progress / failure reason and report timestamp.

Squad and soldier execution remain specialised. Do not force every layer through one generic GOAP action list.

### Required invariants

- One authoritative parent per command node.
- One active task owner at a time.
- Lower layers may report inability or request changes, but cannot silently mutate higher-level intent.
- Emergency soldier survival overrides execution temporarily, then resumes or requests a revised task.
- Stale intelligence is never presented as a live observation.
- All ordering, tie-breaking and state transitions remain deterministic.
- Save/load must preserve command identity, task state and succession state.

### AI-01 implementation acceptance criteria

- A documented map of every current command and decision owner.
- A tested command replacement policy that does not lose unrelated squad state.
- A minimal command-node and task contract designed without breaking existing save formats.
- Tests proving leader succession, invalid subordinate handling, task status transitions and deterministic replay.
- No change to combat outcomes merely from introducing the Titan naming / architecture layer.

## Next work

- **AI-02:** Define Titan Intelligence contracts and cross-squad information sharing.
- **AI-03:** Define task lifecycle, interruption and recovery; then integrate the squad GOAP domain with real facts and executable actions.
- **AI-04:** Add formation-level coordination only after command ownership and intelligence contracts are stable.


## AI-01 implementation

Implemented in `crates/scalia-ai/src/titan.rs`:

- Stable command-node IDs and Commander / Operations / Squad Leader roles.
- Validated hierarchy registration and squad command ownership.
- Existing `leader::Order` reused as the squad command payload.
- Deterministic succession to the nearest active ancestor authorised to own squads; assignments are cleared if no valid successor exists.
- Unit tests cover duplicate/missing parent rejection, authority validation and succession.

**Integration boundary:** this registry is currently an additive authority model. Existing `Battle::command`, squad `Brain`, save format and tactical execution are unchanged. Battle submission/persistence integration is deferred until its replay and save compatibility can be tested end-to-end.
