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
