# PACK Engine Roadmap

**Status:** Active
**Date:** 17 November 2026
**Scope:** Reliability, observability, and game-facing capability of PACK as used by Scalia.

PACK is a focused engine for Scalia, not a general-purpose engine. Work should close real integration and reliability gaps before adding broad editor, physics, scripting, or networking systems. Keep the deterministic simulation authoritative, preserve save/replay compatibility, and make optional platform features conditional on product requirements.

## Current strengths

- Integer-based world simulation, fixed-step battles, deterministic hashes, and versioned battle saves.
- Rust subsystems for simulation, tactical AI, rendering, audio acoustics, world data, and validated blueprints, connected to a WebAssembly browser client.
- GPU-instanced rendering, procedural animation, native/WASM checks, focused tests, and battle performance budgets.
- Browser-side error reporting, a diagnostics panel, model/debug views, and data-driven map and vehicle definitions.

## Prioritized work

### P0 — Runtime failure handling

**Goal:** A transient surface problem skips or recovers without hiding fatal GPU failures.

**Current slice:** Surface outcomes are now classified, device loss/out-of-memory is retained as a fatal reason, and render errors propagate to the browser crash reporter. Full verification remains pending.

- Classify WebGPU surface outcomes: timeout/occlusion skip the frame; outdated/lost surfaces are reconfigured; validation errors are reported.
- Route device loss and GPU out-of-memory errors to the existing fatal error report with the original reason.
- Apply the same behavior to the tablet's secondary surface.
- Keep recoverable errors recoverable; never retry in a busy loop.

**Acceptance:** Each status has a deterministic decision-path test; recoverable cases do not crash the game; fatal cases reach the crash report; resize and secondary-surface failures are covered.

### P1 — Complete diagnostics and performance budgets

**Goal:** Make slow frames and unavailable measurements understandable rather than guessed.

- Retain the existing panel's FPS, frame-time summary, GPU pass timings, JS heap, soldier count, and copied diagnostic snapshot.
- Add sourced measurements for CPU simulation/battle, animation, mesh preparation, uploads, draw calls, triangles, effects, and lights where the runtime can provide them.
- Label unsupported metrics as unavailable and record measurement cadence and overhead.
- Extend benchmark budgets only where repeatable measurements support them; preserve separate aggregate and contact-heavy battle cases.

**Acceptance:** The snapshot includes units, sources, timing, and unavailable fields; no stale or fabricated values; deliberate benchmark regressions fail with a useful message.

### P1 — Integrate tactical AI systems

**Goal:** Make the planned Titan command, intelligence, soldier, and squad behaviors part of the live battle contract.

- Continue the staged AI roadmap: tactical options and order-aware individual decisions, then squad coordination, reporting, and recovery.
- Integrate task lifecycle and command state with battle submission, replay, and save/load only with an explicit compatibility/version plan.
- Keep knowledge source-aware and imperfect; preserve deterministic tie-breaking and return from survival interrupts to valid intent.
- Validate decisions in deterministic seeded scenarios and the Standoff/reference-squad labs.

**Acceptance:** Each stage has behavior tests, reproducible lab evidence, replay/save coverage, and unchanged hashes unless combat behavior is intentionally changed.

### P2 — Share world geometry across systems

**Goal:** Keep sight and acoustics on one prop-occupancy contract, then preserve material and object state as world data flows into runtime systems.

- The initial slice moves prop height bands and raster occupancy into `scalia-core`; tactical sight wraps that grid and Auralis queries it directly.
- Keep dynamic battle doors separate from static prop occupancy and pass their current state explicitly to acoustic queries.
- Follow up by carrying material and stable object identity through battle-map construction and persistence before adding material-specific sound or rendering behavior.

**Acceptance:** Sight and acoustic occupancy agree for windows, walls, furniture, and placed props; door state remains dynamic; battle hashes and save compatibility stay unchanged unless an explicit versioned migration is approved.

### P2 — Improve content authoring

**Goal:** Reduce iteration friction without committing prematurely to a full editor.

- Establish stable asset/content identifiers and a validated manifest for maps, blueprints, and browser assets.
- Add authoring-time checks for missing references, invalid dimensions, unsupported states, and budgets.
- Extend existing model/debug views toward map and blueprint preview/edit workflows; add hot reload only where it has a clear invalidation model.
- Keep shipped data compatible with the existing RON formats or provide explicit migrations.

**Acceptance:** Invalid content fails with actionable field/location errors; valid content loads consistently in tests and the browser; content changes do not silently alter deterministic simulation state.

### Conditional — Platform foundations

These are not assumed P0 gaps. Revisit when product scope requires them.

- **Networking/replication:** Build only if co-op means multiple live clients. First define authoritative state, command transport, reconnect behavior, versioning, and deterministic resynchronization.
- **General physics:** Extend the current grid/navigation model only when vehicles, collision, or projectile gameplay needs exceed it. Keep one authoritative model for prediction and resolution.
- **General editor, scripting, input abstraction:** Add only when the content or platform matrix justifies the maintenance cost.

## Delivery order

1. Harden renderer error classification and propagation.
2. Complete measurements in the diagnostics panel and use them to choose optimizations.
3. Integrate the active AI roadmap in small, replay-safe stages.
4. Improve content validation and authoring based on observed iteration bottlenecks.
5. Reassess conditional platform foundations against confirmed product requirements.

## Guardrails

- Do not rewrite functioning systems or split crates without measured need.
- Preserve deterministic ordering and battle hashes unless a gameplay change is intentional.
- Keep data format changes versioned and save/replay compatibility explicit.
- Require focused tests for behavior changes and `bash scripts/check.sh` before merging.
- For visual, audio, or browser-runtime changes, include the appropriate real-browser validation in addition to automated checks.
