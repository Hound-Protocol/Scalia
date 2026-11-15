# SCALIA Performance Work Orders
Date: 2026-11-15
Baseline: `main` at audit time
Owner: Code Freelancer #1
Status: Audit complete; implementation not started by this work order

## Audit result

The previous performance plan is partly complete. Do not repeat completed work.

| Previous item | Current status | Evidence |
|---|---|---|
| Cover-search O(N²) fix | **DONE** | `crates/scalia-ai/src/battle.rs`: `think()` builds `taken` once, updates it as soldiers claim/release cover. |
| World drain frame-rate determinism | **DONE** | `crates/scalia-core/src/world.rs`: drain uses absolute `from_ms` / `to_ms` elapsed time; integer floors telescope across arbitrary tick intervals. |
| Battle benchmark | **IMPLEMENTED** | `crates/scalia-bench/src/bin/battle_bench.rs`: reports p50/p99/worst for idle, marching and contact. `scripts/verify.sh` runs `--check 1000`. |
| Performance panel v2 | **NOT IMPLEMENTED** | No evidence of the requested per-system live panel in the audited code. |
| Shared scene preparation | **NOT IMPLEMENTED / NEEDS MEASUREMENT** | Main and secondary view paths still need profiling before changing preparation ownership. |
| Allocation reduction | **NOT IMPLEMENTED / NEEDS PROFILING** | GPU buffers remain keyed by `HashMap<String, Gpubuf>`; temporary per-frame allocations need measurement before refactoring. |
| Navigation scratch reuse | **NOT IMPLEMENTED / NEEDS PROFILING** | No verified reusable A* scratch implementation in this audit. |
| GPU surface error handling | **PARTIAL** | `scalia-render/src/gpu.rs::frame` groups all non-success surface results into configure + skip; it does not distinguish timeout, outdated/lost, OOM or device loss. |

### Important correction

The old proposal to introduce a fixed-step world tick is **not currently justified** for needs drain. The drain calculation is already based on absolute elapsed game time and is independent of how the interval is divided. Keep this design unless future world systems introduce per-step state transitions that genuinely require fixed stepping. Do not add a redundant accumulator.

The cover-search optimization is already implemented with a vector built once per decision pass, then updated incrementally. Preserve deterministic soldier iteration and cover selection order; do not replace it with an unordered set without evidence that ordering remains identical.

## Execution order

### PERF-01 — Performance instrumentation panel
**Priority:** P0 — prerequisite for safe optimization

**Scope**
- Add a toggleable diagnostic panel, full-height and readable at 16:9 1080p.
- Show frame time and available breakdowns: CPU simulation, battle step, animation, mesh build, uploads, GPU passes.
- Show draw calls, instances, triangles, soldiers, effects, lights and JS heap where available.
- Retain rolling timings; flag long frames with the slowest known subsystem.
- Surface runtime errors/warnings and provide copy-snapshot action.
- Clearly mark unavailable metrics; never display fabricated or stale values as live measurements.

**Acceptance**
- Panel can be enabled/disabled without affecting simulation.
- Metrics are labelled by source and update cadence.
- Snapshot copies a timestamped diagnostic record.
- Measurable overhead is documented and bounded.
- Existing HUD and game controls remain unaffected.

### PERF-02 — Benchmark coverage and regression budgets
**Priority:** P0

**Current baseline**
- Benchmark exists and is wired into `scripts/verify.sh`.
- CI currently enforces only the 1,000-soldier contact case at p99 <= 35 ms.

**Work**
- Keep aggregate simulation and full-contact combat benchmarks separate.
- Extend CI checks to representative 200 / 500 / 1,000 / 2,000 soldier cases where runtime permits.
- Report p50, p99 and worst; document hardware/runtime context.
- Add allocation counts only if collected reliably (do not estimate them).

**Acceptance**
- `bash scripts/verify.sh` enforces documented budgets.
- Benchmarks are deterministic in setup and do not make flaky timing claims.
- A deliberate budget breach fails CI with a useful message.

### PERF-03 — Shared scene preparation
**Priority:** P1 — after PERF-01

**Scope**
- Use the panel to measure main view and tablet/secondary view costs.
- Identify data rebuilt identically per frame.
- Prepare shared instance/scene data once; keep camera-dependent culling, sorting and uniforms separate.
- Avoid caching mutable state across frames without explicit invalidation.

**Acceptance**
- Before/after timings recorded on the same hardware and scenario.
- Main and secondary views render identical content and preserve independent cameras.
- No stale transforms, visibility or animation state.
- No regression in draw counts or frame time.

### PERF-04 — Allocation and buffer ownership
**Priority:** P1 — profiling-led

**Scope**
- Profile temporary Vec/HashMap creation, clones and string-keyed GPU buffer operations.
- Introduce persistent scratch storage and typed buffer handles only where allocation or lookup costs are material.
- Reuse spatial buckets where ownership and invalidation are clear.
- Avoid global pools or premature pooling.

**Acceptance**
- Allocation count/bytes and CPU time measured before and after.
- Resource lifetime is explicit; no stale GPU buffers or unbounded retained capacity.
- Rendering output and deterministic simulation state remain unchanged.

### PERF-05 — Navigation search scratch
**Priority:** P2

**Scope**
- Audit A* call frequency, path lengths and allocation cost.
- Reuse per-searcher/per-thread arrays and queues with safe reset semantics.
- Prefer existing squad flow fields where they satisfy the query.
- Cache paths only with explicit invalidation for changed obstacles, goals and world state.

**Acceptance**
- Path correctness and determinism tests pass.
- Concurrent searches cannot alias mutable scratch.
- Benchmarks demonstrate a meaningful improvement.

### PERF-06 — GPU surface/device failure handling
**Priority:** P1 — reliability

**Scope**
- Replace the catch-all surface result handling in `scalia-render/src/gpu.rs::frame`.
- Timeout: skip this frame without unnecessary reconfiguration.
- Outdated/Lost: reconfigure and retry on a later frame.
- Out-of-memory/device-lost: report a fatal renderer error with the reason through the existing crash/error path.
- Handle secondary-surface failures consistently.
- Avoid panic, infinite retry loops or silently swallowing fatal failures.

**Acceptance**
- Each recoverable/fatal path has a testable decision path.
- Error reason reaches diagnostics/crash reporting.
- Renderer recovers from resize/surface loss where supported.
- No busy retry loop.

## Cross-cutting rules

- One focused branch and PR per work order.
- Inspect existing implementation before editing; no broad rewrites.
- Preserve deterministic simulation hashes unless the change explicitly alters simulation behaviour.
- Run focused tests and `bash scripts/verify.sh`.
- Include before/after measurements for performance work.
- Do not merge on automated checks alone when visual/gameplay behaviour changes.

## Ballistics — deferred roadmap item

Ballistics is desired, but is **not part of this performance work order**. Revisit once the current work-order queue is clear. Before implementation, audit existing weapon/projectile/recoil/reticle code and define one authoritative projectile model shared by prediction and hit resolution. Avoid implementing a reticle-only approximation as the actual simulation.
