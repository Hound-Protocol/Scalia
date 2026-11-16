# Titan AI Behaviour Roadmap

**Status:** Active  
**Owner:** SCALIA AI workstream  
**Scope:** NPC decision-making and behaviour only. No rendering, animation, audio, UI, or unrelated simulation work.

## Objective

Build NPCs that behave like cautious, natural, tactically competent soldiers. They must act on incomplete information, preserve mission intent, coordinate with teammates, manage risk, and adapt when plans fail. They must never have omniscient battlefield knowledge.

## Design principles

1. **Survival before obedience:** immediate danger may interrupt an order; it does not erase the order.
2. **Caution before certainty:** suspicion changes posture and decisions without becoming a confirmed contact.
3. **No omniscience:** knowledge comes only from personal perception or communicated reports.
4. **Persistent intent:** temporary reactions return to the original task when safe.
5. **Coordination over duplication:** squads assign complementary roles rather than every soldier selecting the same action.
6. **Adaptation:** failed routes, lost contacts, casualties and changing force balance trigger reassessment.
7. **Disciplined aggression:** advancing or firing is a decision, not the default response to contact.
8. **Determinism:** all decision ordering, scoring, tie-breaking and state transitions remain deterministic and replay-safe.

## Target decision pipeline

1. **Perception** — sight, sound, movement, incoming fire, casualties and environment.
2. **Interpretation** — classify observations, estimate threat/location, track confidence, source and freshness.
3. **Assessment** — evaluate exposure, suppression, ammunition, injury, morale, cover, support and local force balance.
4. **Intent selection** — choose an objective such as survive, observe, protect, advance, suppress, withdraw, rescue or regroup.
5. **Planning** — compare feasible options by risk, expected benefit, resources and likely outcome.
6. **Coordination** — assign squad roles, synchronize movement, communicate and prevent conflicting actions.
7. **Execution** — perform the selected action through existing simulation systems.
8. **Evaluation** — measure outcome, update beliefs and task state, then replan when justified.

## Ownership boundaries

- **Titan Command:** command authority, player intent and task assignment.
- **Titan Operations:** coordination and priorities across squads.
- **Titan Squad:** squad-level tactical planning, role allocation and execution.
- **Titan Soldier:** local perception, survival and execution decisions.
- **Titan Intelligence:** shared, imperfect, source-aware battlefield knowledge.
- **Titan Navigation:** route selection and movement feasibility, not tactical intent.
- **Titan Core:** deterministic scheduling, state, events and persistence.

Keep the current `scalia-ai` crate and evolve these logical boundaries incrementally. Do not split crates or replace systems without evidence from the audit.

## Decision behaviour targets

| Situation | Expected behaviour |
|---|---|
| Distant gunfire | Pause/listen, seek information, increase caution; do not blindly charge |
| Suspicious movement | Observe, identify and communicate |
| Uncertain contact report | Increase caution; retain uncertainty |
| Under fire | Seek suitable cover, identify threat direction and communicate |
| Teammate pinned | Evaluate suppression, support and alternate routes |
| Casualty nearby | Assess immediate threat before rescue |
| Lost enemy contact | Search cautiously around last known position |
| Low ammunition | Conserve, reload when safe and request resupply where supported |
| Outnumbered | Improve position, seek support or withdraw |
| Objective secured | Establish security, observe approaches and recover |

## Planning model

Use **tactical scoring for intent/option selection** and **GOAP for feasible action sequences**. GOAP must not be the sole representation of tactical judgement. Scores, costs and tie-breaks must be deterministic and bounded.

Personality/traits may influence risk tolerance and reactions later, but must never override basic survival, information integrity or command constraints.

## Work packages and progress

- [ ] **AI-00 — Behaviour audit (IN PROGRESS):** map the live decision path from perception through execution; inventory all decision owners, priorities, facts, timers, task transitions and interrupts; identify duplicated/conflicting logic, dead paths and missing feedback.
- [ ] **AI-01 — Perception and belief:** source-aware observations, confidence, freshness, uncertainty and personal-vs-shared knowledge.
- [ ] **AI-02 — Soldier decision core:** persistent intent, tactical risk assessment, priorities, interruptions and safe resumption.
- [ ] **AI-03 — Squad tactical brain:** role assignment, support, movement, engagement, casualty response and withdrawal.
- [ ] **AI-04 — Coordination:** command/task integration and inter-squad support without omniscient shared state.
- [ ] **AI-05 — Adaptation and evaluation:** outcome feedback, failed-plan recovery, contact loss and task reassessment.
- [ ] **AI-06 — Personality and variation:** controlled individual differences after core tactical behaviour is reliable.

## AI-00 audit deliverables

1. Current end-to-end decision pipeline with concrete source paths and entry points.
2. Decision ownership matrix: who decides movement, stance, target, fire permission, reload, cover, rescue, retreat and task changes.
3. Priority/interrupt table, including ordering and replan cadence.
4. Data-flow map for personal perception, squad reports and Titan Intelligence.
5. Integration map for Titan Command, Operations, Squad, Soldier, Navigation and task lifecycle.
6. Defect list: contradictions, unreachable actions, stale facts, hidden omniscience, duplicated decisions and determinism risks.
7. Recommended target design and a staged, minimal migration plan.

## Constraints and acceptance criteria

- Audit first; no behaviour rewrite until the current live path is understood.
- Preserve deterministic simulation and save/replay compatibility.
- Prefer additive, isolated changes with regression tests.
- Every decision must be explainable from the NPC's available information and current state.
- Uncertain information must never silently become certainty.
- Temporary survival reactions must preserve and later resume valid mission intent.
- Squad plans must account for available personnel and avoid assigning incompatible simultaneous tasks.
- Tests must cover decision outcomes, interrupts, re-planning, stale/lost contacts and deterministic repeatability.
- No implementation is complete on unit tests alone where in-game behaviour is observable; gameplay validation is required.

## AI-00 — Initial live-code findings

**Audit is underway; these are verified observations, not final design conclusions.**

### Runtime cadence and orchestration

- `crates/scalia-ai/src/battle.rs` owns the live `Battle::step` loop.
- Simulation time advances in fixed caller-supplied steps; perception is staggered per soldier and scheduled around `PERCEIVE_MS = 250` ms.
- Squad leaders are advanced on `LEAD_MS = 250` ms.
- Movement and combat execution continue between these decision passes.
- Battle save state includes soldier minds, squad brains, contacts, visible lists, clocks, command queue and replay history. Any redesign must preserve or deliberately version these semantics.

### Soldier decision path

- `crates/scalia-ai/src/mind.rs` scores six wants: Survive, Engage, Reload, Order, Aid and Idle.
- The current soldier GOAP library has six actions: TakeCover, Reload, ReloadExposed, ReturnFire, Suppress and FollowOrder.
- Facts are predominantly boolean; threat confidence, source, freshness, exposure and tactical alternatives are not represented in the GOAP state.
- `Battle::think` builds these facts and calls `mind::think`; the result controls `may_fire`, `may_suppress`, cover selection and reload initiation.
- Combat target selection and firing are separately executed in `Battle::step`. Each soldier selects the nearest currently visible living enemy; this target selection is not itself a GOAP action.
- This creates two decision authorities: the mind gates some combat behaviour, while the simulation loop independently acquires targets and executes engagement. Audit exact interactions and edge cases before proposing consolidation.

### Perception and shared contacts

- `Battle::perceive` performs line-of-sight checks and reports sightings into a squad-indexed contact list.
- Contact records currently contain enemy ID, last known position, timestamp and confidence, but no explicit source or report provenance.
- Sight, incoming fire and heard gunfire are represented with different confidence constants in battle logic, but the source distinction is not retained in the contact record.
- Contact confidence decays to a nonzero floor; the last known contact can therefore persist indefinitely during a battle.
- `Battle::report` prevents a weaker report from replacing a stronger contact, but freshness/source arbitration needs deeper review.
- A squad contact list is shared by its soldiers. The audit must verify exactly which observations each soldier can act on and whether any stale contact is treated as current.

### Squad and Titan foundations

- `crates/scalia-ai/src/leader.rs` contains substantial live squad/building planning and task execution. Leader decisions run at 250 ms.
- `crates/scalia-ai/src/squad.rs` explicitly describes its GOAP facts/actions as placeholders; do not mistake it for the live squad planner.
- `crates/scalia-ai/src/titan.rs`, `intelligence.rs` and `titan_tasks.rs` provide command, intelligence and task-lifecycle foundations. Integration into `Battle` remains to be verified; task lifecycle documentation previously states the existing command queue and `Brain` remain execution authority.
- `crates/scalia-ai/src/goap.rs` is a deterministic bounded forward planner with integer costs and stable insertion-order tie-breaking.

### Immediate audit questions

1. Trace the full `Battle::think` and `Battle::lead` paths, including every write to `Task`, `may_fire`, `may_suppress`, `target`, `stance` and `cover`.
2. Verify whether a soldier's target selection can bypass intended mind restrictions, and under which conditions.
3. Determine whether contact data is genuinely per-soldier, squad-shared or globally visible at each decision point.
4. Trace casualty aid, drag, retreat/rally and order interruption/resumption paths.
5. Verify Titan task/command/intelligence runtime integration and save/replay implications.

These findings are preliminary. Complete the source-linked ownership and conflict map before proposing code changes.


### AI-00 — Verified decision ownership and conflict findings (2026-11-16)

#### Soldier decisions (battle.rs, mind.rs)

- Battle::think runs at the perception cadence and rebuilds boolean facts before calling mind::think.
- Want priority is currently: Aid (980), empty-magazine Reload (960), close-target Engage (950), Survive (900 + suppression/10), ordinary visible-target Engage (700), known-contact suppression Engage (300), then Order (200). This is a fixed priority ordering, not a unified risk/benefit assessment.
- GOAP actions are TakeCover, Reload, ReloadExposed, ReturnFire, Suppress and FollowOrder. FollowOrder only marks an abstract ORDER_DONE fact; actual squad task execution remains in leader::Brain.
- Battle::think executes only cover selection, reload initiation and fire/suppress permission from the first planned action. Other planned actions are not a complete execution contract.
- Battle::step independently selects the nearest visible living enemy and executes combat. A soldier can continue firing while moving on a non-sprint task if aimed and not otherwise stopped; movement and engagement are not coordinated by a single action executor.
- may_fire defaults to true. TakeCover normally gates fire, but close-range contact overrides that gate in the combat loop. may_suppress is enabled only for Act::Suppress.
- known used for low-stance holding is broader than the validated contact fact: it accepts any living, non-down enemy in the squad contact list without confidence or age checks.
- Cover candidate threat selection filters confidence but not age. This can make old reports influence cover choice.
- Battle::think computes safe partly as !cover_near; absence of a reachable cover option is therefore treated as safe even while under fire. This is a deliberate fallback in current GOAP semantics, but should be explicitly represented as “no better option” rather than conflated with safety in a future design.

#### Squad decisions and interrupts (leader.rs)

- Battle::lead runs every LEAD_MS and executes keep_leader, casualty care, loss assessment, withdrawal, then the current order (Hold, Move, Capture).
- leader::Brain is the live squad execution authority. Its plan and done state drive building-capture methods; replan recomputes the plan when a phase is finished or situation changes through explicit calls.
- Order::Move loss handling counts able personnel. A reduction below half of initial strength sets a rally point and withdraw redirects available members. Lesser losses reset the movement line for regrouping.
- Casualty care assigns the nearest fit squadmate only when casualty_threatened is false. Care can be abandoned if the route becomes threatened; a helper is replaced if incapacitated. Dragging and treatment are explicit phases.
- Care is a separate leader-level process, not a persistent task object. The helper's previous squad task is overwritten during aid and is not explicitly saved as a resumable task. This is a concrete mission-intent preservation gap to address in a later work package.
- fall_back inside buildings counts all live contact records in rooms without confidence or freshness filtering. Stale reports can therefore cause a defender to abandon a room.
- casualty_threatened uses freshness for the direct line-of-sight test (4 s), but its room_hot branch does not apply confidence or age filtering.
- Building clear logic uses a 30 s contact-age bound for some grenade/room decisions, while other room assessments use different or no freshness bounds. Contact semantics are inconsistent across consumers.
- keep_leader succession is deterministic by soldier index among living, non-down squad members.

#### Information and Titan integration

- Contact in battle.rs stores enemy ID, last-known position, timestamp and confidence only. The source distinction (Visual, IncomingFire, Audio) exists in the separate TitanIntelligence type but is not retained by the live Battle contact record.
- report replaces an existing contact only when the new confidence is at least as strong; a weaker but newer observation cannot refresh the position/time. This can preserve an older location despite a newer lower-confidence report.
- Contact confidence decays to CONF_FLOOR = 100; the record is not automatically expired. Consumers must apply their own freshness/confidence policy.
- TitanIntelligence, TitanCommand and TitanTaskBoard are additive foundations, not integrated live decision authorities. Battle continues to use contacts, Brain, its command queue and replay history. No live call sites were found in the inspected Battle/leader decision path.
- titan_tasks.rs models interrupt/resume and progress, but current casualty care and squad task execution do not use it.

#### Determinism and persistence observations

- Battle state serializes soldier minds, squad brains, contacts, visible lists, clocks, command queue, command history and RNG roll count. Any change to these structures requires explicit save compatibility review.
- Mind::plan_for uses a thread-local HashMap cache keyed by facts and want. The cache is an optimization only; deterministic output depends on the planner and stable action ordering, not hash iteration.
- GOAP action order is fixed and planner tie-breaking is stable. Leader and soldier loops process indexed collections in deterministic order.
- Existing combat randomness is derived from the battle seed, simulation time, soldier index and roll counter. Do not introduce nondeterministic random sources into decision logic.

#### Defects and migration implications (audit findings, not implementation authorization)

1. Split execution authority: mind, leader and combat loop can independently influence the same soldier's engagement/movement state. Define explicit decision ownership before expanding behaviour.
2. Inconsistent contact freshness: stale reports influence low stance, cover selection, room fallback and casualty threat differently. Centralize contact validity policy while preserving uncertainty and provenance.
3. Source provenance loss: live contacts cannot distinguish visual confirmation from sound/incoming fire after publication.
4. Weak-report refresh limitation: a lower-confidence newer report cannot refresh a stale location. Future arbitration should separate confidence from observation recency.
5. Task resumption gap: casualty aid overwrites a helper's prior task without a durable resume record.
6. Safety semantics overloaded: SAFE can mean “no better cover exists,” not necessarily low danger.
7. Titan runtime gap: command/intelligence/task contracts are not wired into the live Battle path; do not integrate them wholesale until save/replay and authority boundaries are designed.

#### Recommended minimal migration order

1. Complete remaining write-site and test inventory; establish a baseline with existing save/replay tests.
2. Define one source-aware contact validity API and migrate consumers without changing tactical outcomes initially.
3. Add explicit soldier task/intent interruption snapshots so casualty and survival work can resume valid orders.
4. Separate tactical intent selection from GOAP feasibility and action execution; keep current Battle executor as the adapter during transition.
5. Integrate Titan command/task/intelligence incrementally behind compatibility adapters, with serialization/replay tests at each step.
6. Only then add richer squad coordination and personality variation.


### AI-00 — Write-site inventory and regression coverage (2026-11-16)

#### Authoritative write sites

| State | Primary write sites | Notes |
|---|---|---|
| Squad task / movement goal | `leader.rs`: `lead_move` (470+), `withdraw` (762+), `care` / `give_aid` (821+ / 1072+), `lead_hold` (1219+), `fall_back` (1281+), `ensure_open` (1337+), `lead_capture` (1498+) | Numerous direct `Task::Move` / `Task::Hold` assignments; `Battle::order` (battle.rs:685) is the external/test entry point. |
| Position / path / movement execution | `battle.rs`: `destination` (690+), path/goal assignment (around 799–865), `move_soldiers` (799+) | Leader writes intent; Battle resolves destinations, pathing, spacing, movement and gait. |
| Stance | `battle.rs`: `wounds` (736+), `move_soldiers` (799+), player placement (around 1670+) | Leader sets `posture`; movement execution derives actual stance from gait, cover and posture. |
| Target | `battle.rs`: target clear/acquire (around 1424–1453); casualty drag clears target (`leader.rs`:1114) | Target is independently reacquired by Battle from current visible enemies. |
| Fire permission | `battle.rs`: `think` (972–1081); combat fire gate (around 1470+) | `may_fire` and `may_suppress` are mind outputs, but close-range fire has a separate override. |
| Cover | `battle.rs`: `think` (1054–1079); `leader.rs` tactical crossing/cover (around 1019+) and `fall_back` (1325) | Soldier mind and squad leader both write cover ownership. |
| Aid / drag / busy state | `leader.rs`: `care` / `give_aid` / `start_aid` (821–1157); `battle.rs`: `wounds` / `drag_along` | Aid state lives outside the generic task lifecycle; helper task is overwritten. |
| Reload / weapon action | `battle.rs`: `think` calls `reload` (1067), reload timer in combat step (around 1291+) | GOAP initiates reload; Battle owns timer and completion. |

#### Execution ordering and interrupt behaviour

- `Battle::step` owns the fixed-step simulation. It runs wound processing, movement, perception/think scheduling, leader scheduling and combat resolution in a fixed order; the exact order must remain unchanged unless replay tests prove compatibility.
- `Battle::lead` ordering is explicit: leader succession, casualty care, loss counting, broken-squad withdrawal, current order execution, sandbag completion and grenade resolution.
- A soldier going down forcibly becomes prone and loses task, target, cover, action, posture, path, aid and drag state (`battle.rs:736–755`). This is a hard incapacitation transition, not a resumable interrupt.
- Dead soldiers are forced to `Task::Idle` during movement execution (`battle.rs:807–815`).
- A casualty helper may abandon aid when the route becomes threatened and receives a stationary Hold task (`leader.rs:1081–1087`); this does not restore the task previously held.
- `Battle::think` runs on the staggered perception cadence; leader planning runs on `LEAD_MS`. Movement and combat continue between those decision passes.

#### Regression coverage observed (static source inspection only)

- `mind.rs` has five tests covering survival/cover/reload/fire priorities, close-range engagement, no-cover fallback and suppression.
- `intelligence.rs` has four tests covering report recency/confidence arbitration, decay/future reports, team isolation and expiry.
- `titan.rs` has three command hierarchy/ownership/succession tests.
- `titan_tasks.rs` has five task lifecycle, interrupt/resume, reassignment, invalid transition and terminal-state tests.
- `battle.rs` has one 200,000-case deterministic view-cone equivalence test.
- `leader.rs`, `goap.rs` and `save.rs` contain no inline `#[test]` blocks in the inspected files. No direct regression test currently covers task restoration after casualty response, contact-source propagation into live Battle contacts, inconsistent contact freshness across consumers, target/fire-gate interaction, or Battle save/replay determinism under AI transitions.
- The GitHub connector permits source inspection but does not provide a shell/Cargo test runner or workflow-dispatch action in this session. Therefore no test execution result is claimed. A real CI run is required before any runtime patch is considered validated.

#### AI-00 exit status

Decision ownership, primary write sites, major interrupt paths, contact data flow, Titan integration gap and visible regression-test coverage are now mapped. The audit is **substantially complete**, but its final gate remains a reproducible CI/test baseline and a final cross-check against all current branch changes. Do not mark AI-00 complete until those are available.


### Next implementation work order

- **AI-01 issue:** https://github.com/Hound-Protocol/Scalia/issues/49 — source-aware live contacts, recency/confidence arbitration, centralized freshness policies and save/load regression tests.
- AI-01 is intentionally blocked on the AI-00 CI/test baseline. The GitHub connector cannot run Cargo tests in this session; do not claim validation until a Rust CI run completes.


---

## Handoff checkpoint — 2026-11-16 (authoritative continuation notes)

This section is the current handoff point for the next AI contributor. Read it before changing code. It consolidates the audit state, unresolved gates, implementation boundaries and exact next actions.

### Repository and work tracking

- Repository: `Hound-Protocol/Scalia`.
- Documentation branch: `work/AI-behaviour-roadmap-2026-11-16`.
- Roadmap PR: [#48 — docs(ai): establish Titan behaviour roadmap and audit](https://github.com/Hound-Protocol/Scalia/pull/48). It is a documentation-only draft PR; it has not been merged.
- Audit issue: [#47 — AI-00: Audit live NPC behaviour and decision pipeline](https://github.com/Hound-Protocol/Scalia/issues/47).
- Next implementation issue: [#49 — AI-01: Add source-aware contact intelligence and consistent freshness](https://github.com/Hound-Protocol/Scalia/issues/49).
- Latest default-branch commit observed during this checkpoint: `0c701462e1dd7db199ecc30aba80749e90750cc7` (2026-11-16, merge of PR #46). Re-check the default branch and PR #48 base before starting; this observation is a checkpoint, not a permanent base guarantee.
- Keep the roadmap branch documentation-only. Start implementation work on a fresh branch from the then-current default branch.

### Current completion state

**AI-00 is substantially audited, not closed.** The live decision path, ownership/write-site inventory, key interrupts, contact data flow, Titan integration gap and static test inventory are documented above. Two gates remain:

1. Obtain a reproducible Rust test/CI baseline. No Cargo tests have been executed by the GitHub connector in this work session; do not report tests as passing.
2. Re-check the audit against the latest default branch and relevant open/merged PRs. In particular, verify whether any newer branch changed the inspected call sites or save format.

Do not close AI-00 or mark the audit complete until both gates have evidence recorded in this document and issue #47.

### Exact AI-01 starting point

Before editing, inspect these live paths on the fresh implementation branch:

1. `crates/scalia-ai/src/battle.rs`
   - Find the live `Contact` definition and its derives/serde attributes.
   - Find every `report` call, not only the currently known ones:
     - `report_pub` → `report` (around 1113–1118).
     - Visual sighting from `perceive` (around 1213; `CONF_SEEN`).
     - Incoming-fire/miss/shot-at paths (around 1338 and 1627; `CONF_SHOT_AT`).
     - Heard gunfire (around 1643; `CONF_HEARD`).
   - Confirm whether `report_pub` or any other public/manual report path needs a distinct source. Add an external/manual source only if an actual caller requires it.
   - Inventory every consumer of the live contact vector and record its current age/confidence rule before centralizing policy.
   - Re-check `Battle::step` ordering and the combat target/fire path. AI-01 must not change target acquisition, permission overrides or firing behaviour.
2. `crates/scalia-ai/src/intelligence.rs`
   - Compare the existing Titan source/provenance and recency/confidence arbitration types. Reuse concepts where suitable, but do not wholesale wire TitanIntelligence into Battle.
3. `crates/scalia-ai/src/save.rs` and all Battle serialization/version definitions
   - Trace the actual save envelope/version and migration/refusal behaviour before changing serialized Contact fields.
   - Do not assume adding a serde field is backward-compatible. Decide explicitly whether a default/migration is safe or a save-version increment is required.
4. Search the entire repository for `Contact`, `report(`, contact serialization and save/load/hash tests. The call-site list above is an audit lead, not a substitute for a fresh exhaustive search.

### AI-01 implementation contract

- Retain provenance on each live contact: visual, incoming fire and audio/heard; add another source only when a real call site justifies it.
- Keep personal visibility distinct from squad-shared contact reports.
- Store observation recency independently from confidence. A newer weaker report may refresh last-known position/time, but must not inherit or inflate the previous report's certainty.
- An older report must never rewind a newer observation.
- Define deterministic equal-time arbitration, including a stable source/identity tie-break.
- Centralize named freshness/confidence policies for tactical consumers. Select policy names and thresholds only after inventorying current call sites; document intended semantics. Never let a stale report become a confirmed target.
- Preserve the current target acquisition, fire gating, close-range override, simulation step order and combat execution semantics.
- Keep the patch scoped to `scalia-ai` unless a proven save-format dependency requires a narrowly justified change.
- Do not integrate Titan Command, TitanTaskBoard or TitanIntelligence wholesale as part of AI-01.
- No unrelated rendering, animation, audio, UI, weapon-balancing or general simulation changes.

### Required AI-01 regression tests

Add focused tests for:

- Newer low-confidence report refreshes position/time while retaining lower certainty.
- Older report cannot rewind a newer observation.
- Equal-time reports resolve deterministically, independent of incidental iteration/hash order.
- Visual, incoming-fire and audio provenance remains correct through squad sharing and save/load.
- Fresh, stale, expired and future-dated observations follow each named policy consistently.
- Personal visibility is not accidentally promoted to squad/global certainty.
- Repeat-run determinism and Battle save/load/hash behaviour across contact updates.
- Existing behaviour around target acquisition and firing remains unchanged (regression test or before/after invariant where practical).

### Test and validation gate

Run from the repository root using the project's supported toolchain:

```sh
cargo test --workspace
```

Also run the relevant CI checks used by the repository and any focused `scalia-ai` tests. Record the exact commands, commit SHA, pass/fail result and any environment limitations in the implementation PR and update this roadmap. If workspace tests cannot run locally, use the repository's actual CI workflow and record its run URL; do not substitute static inspection for execution.

Before merge:
- Review save compatibility and replay determinism explicitly.
- Inspect the full diff for scope creep.
- Confirm no target/fire behaviour change slipped into AI-01.
- Require CI and gameplay validation where observable behaviour changes.
- Update issue #49 and this document with implementation/test results.

### Handoff sequence

1. Revalidate default branch, PR #48 and open AI work items.
2. Complete AI-00 CI/test baseline and cross-branch comparison; update issue #47 and this document with evidence.
3. Create a fresh AI-01 branch from the current default branch.
4. Perform the exact source/save/call-site inspection above before coding.
5. Implement provenance + arbitration + named freshness policies as one isolated change set.
6. Add the required regression tests, run workspace/CI checks, review save/replay compatibility.
7. Update this roadmap and issue #49 with commit, test evidence and any follow-on defects.
8. Only after AI-01 is validated, proceed to AI-02 persistent soldier intent and safe interruption/resumption.

### Known follow-on defects — do not silently fold into AI-01

- Casualty aid overwrites a helper's prior task without a durable resume record.
- Soldier mind, squad leader and Battle combat loop have overlapping decision authority.
- Cover ownership is written by both soldier mind and squad leader.
- `SAFE` currently conflates “no better cover option exists” with genuinely low danger in part of the GOAP facts.
- Close-range combat can override the normal `may_fire` gate.
- Titan command/task/intelligence foundations are not live Battle execution authorities.

Track these for AI-02/AI-03/AI-04 or separate narrowly scoped issues. Do not expand AI-01 to absorb them.
