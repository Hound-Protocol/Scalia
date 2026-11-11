# Scalia M2a: Orbits and First Flight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (the user chose inline execution and asked for autonomy) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Planets on circular orbits and an eased, target-leading flight between them, as pure functions of time in the integer core, exported to WebAssembly.

**Architecture:** `trig.rs` (integer sine table), `system.rs` (star and bodies, `body_pos`), `flight.rs` (smootherstep easing, `plan_flight`, `flight_pos`) in `scalia-core`; a new `ProcessKind::Flight` wired into `World`, derive and the state hash; four wasm exports.

**Tech Stack:** Rust 1.97 stable, `Fix` Q32.32, wasm-bindgen 0.2.130 (pinned), Node 22.

**Spec:** `docs/superpowers/specs/2026-11-11-m2-renderer-and-flight-design.md` (decisions 3 and 4)

## Global Constraints

- Integer or `Fix` arithmetic only in `scalia-core`; no floats. Floor rounding, saturating arithmetic. Large distances (millimetres up to about 1e12) are scaled with `i128`, never through `Fix::mul` (a `Fix` holds only 31 integer bits).
- Positions are `Pos { region, x, y, z }` in millimetres. The system region is `RegionId(100)`.
- Periods and durations are real milliseconds (`WorldMs`), like marches.
- The golden state hash `0xcc763f567496bd24` and the scenario hash `11470447963499381888` must not change.
- No teleporting: a flight is continuous from its start to its end.
- Commit messages end with the two attribution lines supplied by the session. Do not merge or push anything except `main` and the working branch.

## Review Focus

- A body with period 0 must not divide by zero; it stays at its starting phase.
- Times near `u64::MAX` milliseconds must not overflow or panic in `body_pos` or `plan_flight`.
- Negative and above-one turn values must wrap in the sine function (floor modulo).
- A flight whose start equals its end has duration 0 and must return the end point at once.
- A tick landing exactly on the end time completes the flight; one millisecond earlier does not.

---

### Task 1: Integer sine and cosine

**Files:**
- Create: `crates/scalia-core/src/trig.rs`, `crates/scalia-core/src/trig_table.rs` (generated once, committed)
- Modify: `crates/scalia-core/src/lib.rs` (`pub mod trig;`)
- Test: `crates/scalia-core/tests/trig.rs`

**Interfaces:**
- Produces: `pub fn sin_cos_turns(turns: Fix) -> (Fix, Fix)` (one turn = 2π; any `Fix`, wraps by floor modulo 1).

- [ ] **Step 1: Write failing tests** in `tests/trig.rs`: `exact_at_quarter_turns` (0 → sin 0, cos ONE; 0.25 → sin ONE, cos 0; 0.5 → sin 0, cos -ONE; 0.75 → sin -ONE, cos 0; allow 1 raw unit); `eighth_turn_is_root_half` (sin and cos of 0.125 within 2e-6 of 0.70710678, i.e. 8,600 raw units); `identity_holds` (sin² + cos² within 4e-6 for 5,000 turns values spread by `mix64`, including negatives and values above 1); `periodic` (f(t) equals f(t + ONE) and f(t - ONE) exactly for 1,000 values); `matches_reference_values` (a table of 8 `(turns, sin, cos)` raw triples printed from the generating script, e.g. 0.1, 0.3, 0.6, 0.9, to within 2e-6).
- [ ] **Step 2: Run** `cargo test -p scalia-core --test trig`. Expected: compile failure (module missing).
- [ ] **Step 3: Generate `trig_table.rs`** with a one-off Python script: `pub const QUARTER_SIN: [i64; 1025]` where entry k is `round(sin(k * pi / 2 / 1024) * 2**32)`. Implement `sin_cos_turns` in `trig.rs`: take the low 32 bits of the raw value as the fraction of a turn (floor modulo), split into quadrant (top 2 bits) and a 30-bit position x; define `sinq(x)` as a linear interpolation of adjacent table entries (index x >> 20, remainder x & 0xFFFFF); the four quadrants use sinq(x) and sinq(2^30 - x) with the usual signs.
- [ ] **Step 4: Run** the test file. Expected: 5 passed.
- [ ] **Step 5: Commit** `feat(core): integer sine and cosine`.

### Task 2: The system and its bodies

**Files:**
- Create: `crates/scalia-core/src/system.rs`; Modify: `lib.rs` (`pub mod system;`)
- Test: `crates/scalia-core/tests/system.rs`

**Interfaces:**
- Consumes: `trig::sin_cos_turns`, `Fix`, `Pos`, `RegionId`, `WorldMs`.
- Produces: `pub const SYSTEM_REGION: RegionId = RegionId(100)`; `pub struct BodyId(pub u8)` (Copy, Eq, Debug); `pub struct Body { pub name: &'static str, pub orbit_radius_mm: i64, pub period_ms: u64, pub phase: Fix }` (phase in turns, 0 up to 1); `pub struct System { pub star: Pos, pub bodies: Vec<Body> }`; `System::demo() -> System` (star at the origin; body 0 "Aster": radius 40,000,000,000 mm, period 600,000 ms, phase 0; body 1 "Brine": radius 90,000,000,000 mm, period 1,800,000 ms, phase 0.25 turn); `System::body_pos(&self, id: BodyId, t: WorldMs) -> Option<Pos>` (None for an unknown id; z equals the star's z).
- Produces: `pub const PARK_OFFSET: (i64, i64, i64) = (0, 50_000_000, 0)` (where a ship parks relative to a body).

- [ ] **Step 1: Write failing tests**: `start_position_is_on_the_phase_axis` (Aster at t=0 is (R, 0, 0); Brine at t=0 is within 4 mm of (0, R, 0)); `distance_from_star_equals_radius` (1,000 times for both bodies, within 3 mm per 1e10 of radius, i.e. relative error 3e-10 plus 2 mm); `returns_after_one_period` (position at t and t + period within 4 mm, 200 times); `quarter_period_is_quarter_turn` (Aster at period/4 is within 4 mm of (0, R, 0)); `unknown_body_is_none`; `period_zero_stays_at_phase` (a body with period 0 does not panic and equals its t=0 position at any time); `huge_time_does_not_panic` (t = `WorldMs(u64::MAX)` and `WorldMs(u64::MAX - 1)` both return Some); `golden_system_hash` (hash of both bodies at 1,000 times 977 ms apart; print once, paste).
- [ ] **Step 2: Run** `cargo test -p scalia-core --test system`. Expected: compile failure.
- [ ] **Step 3: Implement** `body_pos`: fraction = (t mod period) / period as `Fix::from_ratio`; turns = phase + fraction; `(sin, cos) = sin_cos_turns(turns)`; x = star.x + (radius * cos) >> 32 and y = star.y + (radius * sin) >> 32, computed in `i128` and floor-shifted; period 0 gives fraction 0.
- [ ] **Step 4: Run** the test file. Expected: 8 passed.
- [ ] **Step 5: Commit** `feat(core): star system with circular orbits`.

### Task 3: Eased flights that lead a moving target

**Files:**
- Create: `crates/scalia-core/src/flight.rs`; Modify: `lib.rs` (`pub mod flight;`), `process.rs` (`ProcessError::UnknownBody` variant only)
- Test: `crates/scalia-core/tests/flight.rs`

**Interfaces:**
- Consumes: `System`, `BodyId`, `SYSTEM_REGION`, `Pos::lerp`, `dist_mm`, `Fix`, `ProcessError`.
- Produces: `pub struct FlightPlan { pub from: Pos, pub to: Pos, pub start: WorldMs, pub duration_ms: u64 }` (Copy, Eq, Debug); `pub fn ease(num: u64, den: u64) -> Fix` (smootherstep 6t⁵ - 15t⁴ + 10t³ with t = num/den clamped to [0, 1]; `den` 0 gives ONE); `pub fn plan_flight(sys: &System, from: Pos, dest: BodyId, offset: (i64, i64, i64), peak_speed_mm_s: u32, start: WorldMs) -> Result<FlightPlan, ProcessError>` (`ZeroSpeed`; `UnknownBody`; `RegionMismatch` when `from.region != SYSTEM_REGION`); `pub fn flight_pos_raw(from: Pos, to: Pos, start: WorldMs, duration_ms: u64, t: WorldMs) -> Pos` (`from` before the start, `to` at or after the end, eased in between); `pub fn flight_pos(plan: &FlightPlan, t: WorldMs) -> Pos`; `pub const DEMO_PEAK_SPEED_MM_S: u32 = 2_000_000_000`.
- Duration rule: eased average speed is peak / 1.875, so `duration_ms = ceil(distance_mm * 1875 / peak_speed_mm_s)`. The planner starts from the destination at `start`, then repeats 8 times: target = `body_pos(dest, start + duration) + offset`; duration from the distance to that target. The plan's `to` is the target at the final duration, so the ship arrives exactly where the body is.

- [ ] **Step 1: Write failing tests**: `ease_endpoints_and_midpoint` (0 → 0, den → ONE, half → 0.5 within 1 raw unit, above den → ONE, den 0 → ONE); `ease_is_monotone` (1,000 steps); `arrival_is_where_the_body_will_be` (Aster to Brine, `to == body_pos(Brine, start + duration) + offset` exactly, for 5 start times); `peak_speed_is_close_to_requested` (distance × 1875 / duration within 1% of the requested peak); `starts_at_from_and_ends_at_to`; `speed_is_zero_at_both_ends` (the point 1 ms after the start is within 1e-5 of the whole distance from `from`); `progress_is_monotone` (distance from `from` never decreases over 200 samples); `demo_flight_takes_a_sensible_time` (Aster to Brine at the demo peak speed is between 10,000 and 600,000 ms); `zero_speed_rejected`; `unknown_body_rejected`; `wrong_region_rejected` (`from.region = RegionId(1)`); `zero_distance_has_zero_duration` (destination body at the same point and offset (0,0,0), from set to it); `huge_start_does_not_panic` (`WorldMs(u64::MAX - 5)`); `golden_flight_plan` (the demo plan's `to`, `duration_ms` pinned after one printed run).
- [ ] **Step 2: Run** `cargo test -p scalia-core --test flight`. Expected: compile failure.
- [ ] **Step 3: Implement** the module; add `UnknownBody` to `ProcessError` (check that no existing `match` on `ProcessError` becomes non-exhaustive). Use `u128` for distance × 1875 and saturating time arithmetic.
- [ ] **Step 4: Run** the test file and `cargo test -p scalia-core`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(core): eased target-leading flight planning`.

### Task 4: Flight as a process in the world

**Files:**
- Modify: `crates/scalia-core/src/process.rs`, `world.rs`, `derive.rs`, `state_hash.rs`
- Test: `crates/scalia-core/tests/flight_world.rs`

**Interfaces:**
- Consumes: `FlightPlan`, `plan_flight`, `flight_pos_raw`, `World`, `GroupId`.
- Produces: `ProcessKind::Flight { from: Pos, to: Pos }`; `Process::flight(plan: &FlightPlan) -> Process`; `pub fn process_pos(p: &Process, t: WorldMs) -> Option<Pos>` (marches and flights); `World::start_flight(&mut self, id: GroupId, sys: &System, dest: BodyId, offset: (i64, i64, i64), peak_speed_mm_s: u32, start: WorldMs) -> Result<(), ProcessError>` (`Busy` when a process is still running at `start`; group unchanged on every error); `World::tick` completes a flight at or after its end (group position becomes `to`, process idle); `derive_visible` skips groups in a flight (they are aboard); `state_hash` folds `Flight` with its own tag.

- [ ] **Step 1: Write failing tests** in `tests/flight_world.rs`: `flight_moves_the_group_and_lands_it` (group in the system region at Aster + `PARK_OFFSET`; start a flight to Brine; tick to 1 ms before the end: `process_pos` is not `to` and the group's stored position still equals the start; tick exactly to the end: position equals `plan.to` and the process is idle); `second_flight_while_flying_is_busy`; `flight_from_the_wrong_region_errors_and_changes_nothing`; `unknown_body_errors_and_changes_nothing`; `soldiers_aboard_a_flying_group_are_not_derived` (a view box around the group's start in the system region returns 0 individuals during the flight and more than 0 for the same group when idle); `state_hash_differs_in_flight_and_is_repeatable` (two identical worlds hash equal; starting a flight changes the hash); `existing_golden_state_hash_unchanged` (call `build_scenario` and assert the known golden `0xcc763f567496bd24` the same way the existing golden test does).
- [ ] **Step 2: Run** `cargo test -p scalia-core --test flight_world`. Expected: compile failure.
- [ ] **Step 3: Implement** the variant and wiring. Add the new arms to every `match` on `ProcessKind` (derive.rs three places, state_hash.rs one, process.rs `march_pos`). `Process::flight` copies `from`, `to`, `start`, `duration_ms` from the plan.
- [ ] **Step 4: Run** `bash scripts/verify.sh`. Expected: `ALL CHECKS PASSED`; the golden hashes unchanged.
- [ ] **Step 5: Commit** `feat(core): flight as a process`.

### Task 5: Wasm exports and check

**Files:**
- Modify: `crates/scalia-wasm/src/lib.rs`, `crates/scalia-wasm/scripts/check_wasm.mjs`

**Interfaces:**
- Produces (wasm-bindgen): `system_body_count() -> u32`; `system_body_name(i: u32) -> Option<String>`; `system_body_pos(i: u32, t_ms: f64) -> Vec<f64>` (`[x_mm, y_mm]` in the system region, empty for an unknown body); `#[wasm_bindgen] pub struct Flight` with `new(from_body: u32, to_body: u32, depart_ms: f64, peak_mm_s: f64) -> Result<Flight, JsError>` (the ship starts at `body_pos(from_body, depart) + PARK_OFFSET` and flies to `body_pos(to_body, arrival) + PARK_OFFSET`), `duration_ms() -> f64`, `pos(t_ms: f64) -> Vec<f64>` (`[x_mm, y_mm]`), `from() -> Vec<f64>`, `to() -> Vec<f64>`.

- [ ] **Step 1: Extend `check_wasm.mjs` first** with failing assertions: body count is 2 and names are `Aster` and `Brine`; `system_body_pos(0, 0)` is `[40000000000, 0]`; `system_body_pos(0, 123456)` equals the native value (print it from a native test or small bin, paste it); `system_body_pos(5, 0)` has length 0; a `Flight(0, 1, 1000, 2e9)` has `duration_ms()` between 10,000 and 600,000, `pos(1000)` equals `from()`, `pos(1000 + duration)` equals `to()`, and `to()` equals `system_body_pos(1, 1000 + duration)` shifted by the park offset; `new Flight(0, 9, 0, 2e9)` throws a readable error; `new Flight(0, 1, 0, 0)` throws.
- [ ] **Step 2: Run** the wasm build and `node crates/scalia-wasm/scripts/check_wasm.mjs`. Expected: the new assertions fail (functions missing).
- [ ] **Step 3: Implement** the exports (non-finite or negative JS numbers become 0 as the existing helpers do).
- [ ] **Step 4: Run** `bash scripts/verify.sh`. Expected: `ALL CHECKS PASSED`.
- [ ] **Step 5: Commit** `feat(wasm): orbits and flight exports`.
