# Scalia Core (Milestone 0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the deterministic Rust core that advances company-level groups once per second and derives individual soldiers from group state, with benchmarks and a native-versus-WebAssembly equality check.

**Architecture:** A Cargo workspace. `scalia-core` is a pure, integer-only library (packed arrays, timestamped process records, counter-based hashing). `scalia-bench` holds native timing and hash binaries. `scalia-wasm` exposes group state and derived-individual buffers to the browser. Individuals are never stepped; they are computed from (group state, time, seed).

**Tech Stack:** Rust stable (1.97 installed), target `wasm32-unknown-unknown`, `wasm-bindgen` plus `wasm-bindgen-cli`, Node 22 for the equality check.

**Spec:** "Project Scalia: Game Design Document", sections 10 and 11 (https://claude.ai/artifact/3cw3zQ4SrEuSQweYLcL6d1).

## Global Constraints

- Integers (fixed-point) for all authoritative state; floats only when drawing (`f32` only in `Fix::to_f32` and the WebAssembly output buffers).
- Custom packed arrays, not an entity framework.
- Hierarchical position: a region id plus a local position in whole millimetres (64-bit).
- Group state advances once per second (`TICK_MS = 1000`).
- World time in real milliseconds; game time is derived at 24 times real.
- Companies and above are authoritative; squads and platoons are derived (Milestone 0 uses companies only).
- Randomness is counter-based hashing from (seed, group, soldier, stream); no stored generator state.
- Each process is a record: start time, duration, parameters.
- Single-threaded core; no threads in the browser build.
- Roster record is 8 bytes per soldier; sized for 10 million soldiers, with `u32` indices so growth to 200 million needs no rewrite.
- Per-soldier stats are available when zoomed in: every rostered soldier has a stored record (experience, kills, health) plus skills derived from a hash and experience (Task 7b). Persistent individuals are excluded from derived crowds; Milestone 0 implements only that exclusion flag, and the full-state table for players, officers and spies is deferred (decided 11 Nov 2026).
- Rounding rule everywhere: floor (toward negative infinity).
- Acceptance: `cargo test` passes with golden results; one tick of 1,000,000 soldiers (about 6,700 company groups) under 5 ms; deriving 20,000 individuals under 2 ms; identical output on two runs and identical native and WebAssembly result hash; 5,000-soldier boarding gives the same progress at any query time.
- Out of scope: drawing, procedural galaxy, economy, combat, networking, persistence, ship design data, vehicle derivation, small-fight projectile simulation.

## Review Focus

- A ship layout with zero ramps, doors or corridor width must return an error, never divide by zero (Task 5).
- A group whose soldiers are all dead or wounded must derive nobody and board in zero time (Tasks 5 and 7).
- Query times before a process starts, after it ends, or at `u64::MAX` must clamp, never underflow or overflow (Tasks 1, 4, 5).
- Negative coordinates must round by the floor rule in position blending and formation offsets (Tasks 1 and 7).
- Groups of zero or one soldier, and views in a different region from a group, must not panic (Tasks 6 and 7).

## File Structure

```
scalia/
  Cargo.toml, rust-toolchain.toml, .gitignore, STATUS.md
  crates/scalia-core/src/
    lib.rs        re-exports
    fixed.rs      Fix (Q32.32)
    time.rs       WorldMs, constants
    position.rs   RegionId, Pos, dist_mm, isqrt
    hash.rs       mix64, hash4, unit, range_i64, stream constants
    units.rs      UnitType, Composition, Needs
    roster.rs     SoldierRec, Health, Roster
    groups.rs     GroupId, GroupSpec, Groups
    process.rs    Process, ProcessKind, ProcessError, march
    boarding.rs   ShipLayout, board_times, seated_count
    world.rs      World, tick, start_march, start_boarding
    derive.rs     Phase, Derived, ViewBox, formation, derive_soldier, derive_visible
    scenario.rs   build_scenario
    state_hash.rs state_hash, derive_hash
  crates/scalia-core/tests/   one file per task
  crates/scalia-bench/src/bin/{sim_bench.rs, hash_scenario.rs}
  crates/scalia-wasm/src/lib.rs
  scripts/check_wasm.mjs
```

---

### Task 1: Workspace and fixed-point basics

**Files:**
- Create: `Cargo.toml` (workspace, members `crates/*`), `rust-toolchain.toml` (channel `stable`, target `wasm32-unknown-unknown`), `.gitignore` (`target/`, `pkg/`), `crates/scalia-core/{Cargo.toml, src/lib.rs, src/fixed.rs, src/time.rs, src/position.rs}`
- Test: `crates/scalia-core/tests/basics.rs`

**Interfaces:**
- Produces:
  - `pub struct Fix(pub i64)` (Q32.32), `Fix::ZERO`, `Fix::ONE`, `from_int(i32)`, `from_ratio(i64, i64) -> Option<Fix>`, `mul(self, Fix) -> Fix` (saturating), `div(self, Fix) -> Option<Fix>`, `floor_to_i64(self) -> i64`, `to_f32(self) -> f32`
  - `pub struct WorldMs(pub u64)`, `game_ms(self) -> u64` (saturating, times `GAME_SPEED = 24`), `since(self, earlier: WorldMs) -> u64` (saturating), `saturating_add(self, ms: u64) -> WorldMs`; `pub const TICK_MS: u64 = 1000`
  - `pub struct RegionId(pub u32)`, `pub struct Pos { region: RegionId, x: i64, y: i64, z: i64 }`, `offset(self, dx, dy, dz) -> Pos`, `lerp(self, to: Pos, num: u64, den: u64) -> Option<Pos>` (`None` if regions differ; floor rounding), `pub fn dist_mm(a: Pos, b: Pos) -> u64`, `pub fn isqrt(n: u128) -> u128`

- [ ] **Step 1:** `git init -b main` in `/home/claude/scalia`; create the workspace files and the empty `scalia-core` crate; run `cargo build` (expect success).
- [ ] **Step 2: Write failing tests in `tests/basics.rs`:**
  - `fix_floor_rounding`: `Fix::from_ratio(-1,3).unwrap().mul(Fix::from_int(3)).floor_to_i64() == -1` and `Fix::from_ratio(1,3).unwrap().mul(Fix::from_int(3)).floor_to_i64() == 0`
  - `fix_div_zero_is_none`: `Fix::ONE.div(Fix::ZERO) == None` and `Fix::from_ratio(1,0) == None`
  - `fix_mul_saturates`: `Fix::from_int(1_000_000_000).mul(Fix::from_int(2)).floor_to_i64() == 2_000_000_000` and `Fix::from_int(2_000_000_000).mul(Fix::from_int(2)) == Fix(i64::MAX)`
  - `time_game_ms`: `WorldMs(1000).game_ms() == 24_000`, `WorldMs(u64::MAX).game_ms() == u64::MAX`, `WorldMs(5).since(WorldMs(9)) == 0`
  - `pos_lerp_floors`: with region 1, `lerp` from x=-1 to x=0 by 1/3 gives x=-1; from x=0 to x=-1 by 1/3 gives x=-1; different regions give `None`
  - `pos_dist`: `dist_mm((0,0,0),(3000,4000,0)) == 5000`; `isqrt(0) == 0`; `isqrt(24) == 4`
- [ ] **Step 3:** Run `cargo test -p scalia-core --test basics`; expect compile failure or FAIL.
- [ ] **Step 4:** Implement the three modules per the signatures. `mul` and lerp use `i128` intermediates with arithmetic-shift/floor division; `isqrt` by integer Newton iteration.
- [ ] **Step 5:** Run `cargo test -p scalia-core --test basics`; expect PASS.
- [ ] **Step 6:** Commit: `git add -A && git commit -m "feat: workspace, fixed-point, time and position types"`.

---

### Task 2: Counter-based hashing

**Files:**
- Create: `crates/scalia-core/src/hash.rs`
- Test: `crates/scalia-core/tests/hash.rs`

**Interfaces:**
- Consumes: `Fix` (Task 1)
- Produces: `mix64(u64) -> u64`, `hash4(seed: u64, a: u64, b: u64, c: u64) -> u64`, `unit(h: u64) -> Fix` (value in [0,1)), `range_i64(h: u64, lo: i64, hi: i64) -> i64` (lo inclusive, hi exclusive; returns `lo` if `hi <= lo`), constants `STREAM_FORMATION = 1`, `STREAM_WALK = 2`, `STREAM_SEAT = 3`, `STREAM_FORMATION_Y = 4`

- [ ] **Step 1: Write failing tests** (golden values from the SplitMix64 finaliser; `mix64(x)` adds `0x9E3779B97F4A7C15` before mixing; `hash4` is `h=mix64(seed); h=mix64(h^a); h=mix64(h^b); mix64(h^c)`):
  - `mix64(0) == 0xe220a8397b1dcdaf`, `mix64(1) == 0x910a2dec89025cc1`
  - `hash4(1,2,3,4) == 0xd55ccd4aeb3ccafb`
  - `unit(hash4(1,2,3,4)) == Fix(3579628874)`, `unit(u64::MAX) == Fix(0xFFFF_FFFF)`
  - `range_i64(hash4(1,2,3,4), -5, 5) == 3`, `range_i64(mix64(0), 0, 100) == 88`, `range_i64(7, 5, 5) == 5`
- [ ] **Step 2:** Run `cargo test -p scalia-core --test hash`; expect FAIL.
- [ ] **Step 3:** Implement. `unit` is the top 32 bits as the fraction; `range_i64` is `lo + ((h as u128 * span as u128) >> 64) as i64`.
- [ ] **Step 4:** Run the tests; expect PASS.
- [ ] **Step 5:** Commit: `feat: counter-based hashing`.

---

### Task 3: Unit types, roster and groups

**Files:**
- Create: `crates/scalia-core/src/units.rs`, `roster.rs`, `groups.rs`
- Test: `crates/scalia-core/tests/groups.rs`

**Interfaces:**
- Consumes: `Fix`, `Pos` (Task 1). `Groups` has no process column yet; Task 4 adds it.
- Produces:
  - `enum UnitType { Infantry, Medic, Tank, Truck }`, `Composition(pub [u32; 4])` with `people(&self) -> u32` (infantry plus medics), `total(&self) -> u32`
  - `Needs { food, sleep, morale, upkeep: Fix }`, `Needs::full()` (all `Fix::ONE`)
  - `enum Health { Healthy = 0, Wounded = 1, Dead = 2 }`, `#[repr(C)] struct SoldierRec { xp: u16, kills: u16, health: Health, flags: u8, reserved: u16 }` (8 bytes), `const FLAG_PERSISTENT: u8 = 1`
  - `Roster::new()`, `alloc(&mut self, n: u32) -> u32` (returns start index), `rec(&self, start: u32, slot: u32) -> &SoldierRec`, `rec_mut(...)`
  - `GroupId(pub u32)`, `GroupSpec { pos: Pos, composition: Composition, needs: Needs }`, `Groups::new()`, `spawn(&mut self, spec: GroupSpec, roster: &mut Roster) -> GroupId`, `len()`, `pos(id) -> Pos`, `composition(id) -> &Composition`, `needs(id) -> &Needs`, `roster_range(id) -> (u32, u32)` (start, length), `people(id) -> u32`

- [ ] **Step 1: Write failing tests:**
  - `soldier_rec_is_8_bytes`: `size_of::<SoldierRec>() == 8`
  - `spawn_allocates_roster`: a group of 150 infantry and 5 medics gets `roster_range` length 155 and `people == 155`; every record is `Healthy` with `xp == 0`
  - `ranges_disjoint`: two groups' roster ranges do not overlap
  - `zero_soldier_group_allowed`: a group with only tanks has roster length 0 and spawns without error
- [ ] **Step 2:** Run `cargo test -p scalia-core --test groups`; expect FAIL.
- [ ] **Step 3:** Implement with one `Vec` per field in `Groups` (struct-of-arrays) and one `Vec<SoldierRec>` in `Roster`.
- [ ] **Step 4:** Run the tests; expect PASS.
- [ ] **Step 5:** Commit: `feat: unit types, roster and group arrays`.

---

### Task 4: Process records and marching

**Files:**
- Create: `crates/scalia-core/src/process.rs`
- Modify: `groups.rs` (add a `process: Vec<Process>` column; `Groups::process(id) -> &Process`, `set_process(id, Process)`; new groups start `Process::idle()`)
- Test: `crates/scalia-core/tests/process.rs`

**Interfaces:**
- Consumes: `Pos`, `WorldMs`, `dist_mm`, `ShipLayout` (declared in Task 5; add `ProcessKind::Board` in Task 5)
- Produces: `enum ProcessError { ZeroSpeed, ZeroThroughput, ZeroSeatCols }`, `struct Process { kind: ProcessKind, start: WorldMs, duration_ms: u64 }`, `enum ProcessKind { Idle, March { from: Pos, to: Pos, speed_mm_s: u32 } }`, `Process::idle() -> Process`, `Process::march(from: Pos, to: Pos, speed_mm_s: u32, start: WorldMs) -> Result<Process, ProcessError>`, `pub fn march_pos(p: &Process, t: WorldMs) -> Pos`

- [ ] **Step 1: Write failing tests:**
  - `march_duration`: from (0,0,0) to (3000,4000,0) at 1000 mm/s lasts 5000 ms; at 3000 mm/s lasts 1667 ms (ceiling)
  - `march_zero_speed_errors`: speed 0 returns `Err(ProcessError::ZeroSpeed)`
  - `march_pos_clamps`: before start returns `from`; at start plus half the duration is within 1 mm of the midpoint; at the end and at `WorldMs(u64::MAX)` returns `to`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement; `duration_ms = ceil(dist_mm * 1000 / speed)`; `march_pos` uses `Pos::lerp` with elapsed over duration, clamped.
- [ ] **Step 4:** Run; expect PASS.
- [ ] **Step 5:** Commit: `feat: process records and marching`.

---

### Task 5: Boarding

**Files:**
- Create: `crates/scalia-core/src/boarding.rs`
- Modify: `process.rs` (add `ProcessKind::Board { ship: Pos, layout: ShipLayout, interval_ms: u64 }` and `Process::board`)
- Test: `crates/scalia-core/tests/boarding.rs`

**Interfaces:**
- Consumes: `hash4`, `range_i64`, `STREAM_WALK`, `STREAM_SEAT`, `GroupId`, `Pos`, `WorldMs`
- Produces:
  - `struct ShipLayout { ramps: u16, doors: u16, corridor_width_cm: u16, seat_cols: u16 }`
  - Constants: `RAMP_MILLI_PER_S = 800`, `DOOR_MILLI_PER_S = 1200`, `CORRIDOR_MILLI_PER_CM = 15`, `WALK_BASE_MS = 12_000`, `RAMP_CLIMB_MS = 6_000`, `SEAT_BASE_MS = 8_000`, `JITTER_PERCENT = 20`
  - `ShipLayout::throughput_milli_per_s(&self) -> Result<u64, ProcessError>` (minimum of `ramps*800`, `doors*1200`, `corridor_width_cm*15`; `ZeroThroughput` if any is zero; `ZeroSeatCols` if `seat_cols` is zero)
  - `pub fn interval_ms(throughput_milli_per_s: u64) -> u64` (ceiling of `1_000_000 / throughput`)
  - `struct BoardTimes { start_walk_ms, reach_ramp_ms, top_ramp_ms, seated_ms: u64 }` and `pub fn board_times(seed: u64, gid: GroupId, slot: u32, interval_ms: u64) -> BoardTimes` (offsets from process start; `start_walk = slot * interval`; walk and seat durations are the base values scaled by `(100 - 20 + range_i64(hash, 0, 41)) / 100`; climb is exactly `RAMP_CLIMB_MS`)
  - `Process::board(start: WorldMs, ship: Pos, layout: ShipLayout, seed: u64, gid: GroupId, healthy_slots: impl Iterator<Item = u32>) -> Result<Process, ProcessError>` (duration is the largest `seated_ms` over the healthy slots; 0 if none)
  - `pub fn seated_count(seed: u64, gid: GroupId, healthy_slots: impl Iterator<Item = u32>, interval_ms: u64, start: WorldMs, t: WorldMs) -> u32`

- [ ] **Step 1: Write failing tests:**
  - `throughput_is_minimum`: layout (ramps 2, doors 3, corridor 200 cm, seat_cols 20) gives 1600 milli/s and `interval_ms(1600) == 625`
  - `zero_layout_errors`: zero ramps, zero doors or zero corridor width returns `Err(ZeroThroughput)`; zero `seat_cols` returns `Err(ZeroSeatCols)`
  - `board_times_ordered_and_bounded`: for slot 0, `walk <= reach <= top <= seated`; walk duration within [9_600, 14_400]; climb exactly 6_000; seat duration within [6_400, 9_600]; two calls with the same inputs are equal
  - `duration_is_max_seated`: for 150 healthy slots, `Process::board(...).duration_ms` equals the maximum `seated_ms` computed by an independent loop
  - `five_thousand_boarding_query_invariant`: for 5,000 healthy slots, `seated_count` at 200 times sampled in order and in shuffled order give identical values for identical `t`; the count is 0 at `start - 1` (use start 10_000), non-decreasing in `t`, and equals 5,000 at `start + duration`
  - `all_dead_group`: zero healthy slots gives duration 0 and `seated_count == 0`
  - `query_extremes`: `t = WorldMs(0)` and `t = WorldMs(u64::MAX)` do not panic; the latter gives the full count
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement per the signatures. Queue position is the slot index, so dead or wounded slots leave gaps in the queue.
- [ ] **Step 4:** Run; expect PASS.
- [ ] **Step 5:** Commit: `feat: layout-limited boarding`.

---

### Task 6: Needs, world and tick

**Files:**
- Create: `crates/scalia-core/src/world.rs`
- Modify: `units.rs` (needs drain constants)
- Test: `crates/scalia-core/tests/world.rs`

**Interfaces:**
- Consumes: `Groups`, `Roster`, `Process`, `march_pos`, `Needs`, `Fix`, `WorldMs`, `TICK_MS`
- Produces:
  - Constants per game hour: `FOOD_DRAIN = 4%`, `SLEEP_DRAIN = 6%`, `UPKEEP_DRAIN = 1%`; morale does not drain in Milestone 0
  - `struct World { pub seed: u64, pub now: WorldMs, pub groups: Groups, pub roster: Roster }`, `World::new(seed: u64) -> World`
  - `enum TickError { TimeWentBackwards }`, `World::tick(&mut self, now: WorldMs) -> Result<(), TickError>` (drains needs by `rate * game_elapsed_ms / 3_600_000`, floored, clamped at zero; a finished `March` sets the group position to `to` and the process to idle; a finished `Board` stays as the record)
  - `World::start_march(&mut self, id: GroupId, to: Pos, speed_mm_s: u32, start: WorldMs) -> Result<(), ProcessError>`
  - `World::start_boarding(&mut self, id: GroupId, ship: Pos, layout: ShipLayout, start: WorldMs) -> Result<(), ProcessError>` (collects the group's healthy slots; on error the group is unchanged)

- [ ] **Step 1: Write failing tests:**
  - `needs_drain_one_game_hour`: after 150 ticks from 0 to 150_000 ms, food is within 256 raw units of `ONE - 4%`, upkeep within 256 of `ONE - 1%`, sleep within 256 of `ONE - 6%`, morale equals `ONE`
  - `needs_never_negative`: after 10 game days of ticks, no meter is below zero
  - `tick_completes_march`: a 5000 ms march ends with the group at `to` after `tick(start + 5000)` and not before
  - `tick_backwards_errors`: `tick(WorldMs(5))` after `tick(WorldMs(10))` returns `Err(TimeWentBackwards)` and leaves the world unchanged
  - `empty_and_zero_soldier_world`: ticking an empty world, and a world with a tank-only group, does not panic
  - `start_boarding_error_leaves_group_unchanged`: a zero-ramp layout returns `Err(ZeroThroughput)` and the group's process is still idle
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement. Needs math in `i128`; one linear pass over the group arrays per tick.
- [ ] **Step 4:** Run; expect PASS.
- [ ] **Step 5:** Commit: `feat: world, needs drain and tick`.

---

### Task 7: Deriving individuals

**Files:**
- Create: `crates/scalia-core/src/derive.rs`
- Test: `crates/scalia-core/tests/derive.rs`

**Interfaces:**
- Consumes: `World`, `GroupId`, `Pos`, `Health`, `FLAG_PERSISTENT`, `march_pos`, `board_times`, `hash4`, `STREAM_FORMATION`, `range_i64`
- Produces:
  - `enum Phase { Standing = 0, Marching = 1, Queued = 2, Walking = 3, Climbing = 4, Seated = 5 }`
  - `struct Derived { pub slot: u32, pub group: GroupId, pub pos: Pos, pub phase: Phase }`
  - `struct ViewBox { region: RegionId, min_x: i64, max_x: i64, min_y: i64, max_y: i64 }`
  - `pub fn formation_offset(seed: u64, gid: GroupId, slot: u32, n_slots: u32) -> (i64, i64)`: `cols = ceil_sqrt(n_slots)` (at least 1), `rows = ceil(n_slots / cols)`; `dx = col*900 - (cols-1)*450 + jx` and `dy = row*900 - (rows-1)*450 + jy`, with `jx = range_i64(hash4(seed, gid, slot, STREAM_FORMATION), -150, 151)` and `jy = range_i64(hash4(seed, gid, slot, STREAM_FORMATION_Y), -150, 151)`
  - `pub fn derive_soldier(w: &World, gid: GroupId, slot: u32, t: WorldMs) -> Option<Derived>` (`None` unless `Healthy` and not persistent)
  - `pub fn derive_visible(w: &World, t: WorldMs, view: &ViewBox, cap: usize, out: &mut Vec<Derived>)` (clears `out`, skips groups in another region or outside the view expanded by 100,000 mm, stops at `cap`)
- Rules: standing and marching soldiers are at the group's position at `t` (marching uses `march_pos`) plus the formation offset. Boarding: before `start + start_walk` is `Queued` at the formation spot; then `Walking` along a floor-rounded `lerp` from the formation spot to the ramp base `ship.offset(-6000, 0, 0)`; then `Climbing` to the ramp top `ship.offset(0, 0, 2500)`; then `Seated` at `ship.offset(col*600, row*600, 2500)` where `col = slot % seat_cols`, `row = slot / seat_cols`. A time at or after the end is `Seated`.

- [ ] **Step 1: Write failing tests:**
  - `formation_unique_and_deterministic`: 150 soldiers at group position (0,0,0) have 150 distinct positions; two calls are equal
  - `formation_negative_coordinates`: a group at (-5000,-5000,0) gives positions equal to `(-5000 + dx, -5000 + dy)` from `formation_offset`
  - `dead_wounded_persistent_are_none`: a dead slot, a wounded slot and a slot with `FLAG_PERSISTENT` return `None`; the rest return `Some`
  - `all_dead_group_derives_nobody`: `derive_visible` on a group with every slot dead returns an empty list
  - `marching_phase`: a marching group reports `Marching` mid-march and `Standing` after the end, at the destination
  - `boarding_phases_and_seats`: before start every soldier is `Queued`; slot 0 is `Walking` shortly after start; after the process end every soldier is `Seated` at the seat position formula; the count of `Seated` at any `t` equals `boarding::seated_count`
  - `visible_respects_cap_and_region`: a view in another region returns nothing; a view covering many groups with `cap = 20_000` returns exactly 20,000
  - `tiny_groups`: groups of 0 and 1 soldiers do not panic; the single soldier sits at `group pos + formation_offset(.., 0, 1)`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement. Seat and ramp geometry are constants in `derive.rs`.
- [ ] **Step 4:** Run; expect PASS.
- [ ] **Step 5:** Commit: `feat: derive individuals from group state`.

---

### Task 7b: Per-soldier stats for the zoomed view

**Files:**
- Create: `crates/scalia-core/src/stats.rs`
- Modify: `hash.rs` (add `STREAM_ACCURACY = 5`, `STREAM_SPEED = 6`, `STREAM_COMPOSURE = 7`, `STREAM_NAME = 8`)
- Test: `crates/scalia-core/tests/stats.rs`

**Interfaces:**
- Consumes: `World`, `GroupId`, `Health`, `hash4`, `range_i64`
- Produces: `struct SoldierStats { slot: u32, group: GroupId, xp: u16, kills: u16, health: Health, accuracy: u8, speed: u8, composure: u8, name_seed: u64 }` and `pub fn soldier_stats(w: &World, gid: GroupId, slot: u32) -> Option<SoldierStats>` (`None` if the slot is out of range). Each skill is `min(100, 40 + range_i64(hash4(seed, gid, slot, STREAM_<SKILL>), 0, 41) + min(xp, 1000) / 50)`; `name_seed = hash4(seed, gid, slot, STREAM_NAME)`.

- [ ] **Step 1: Write failing tests:**
  - `stats_deterministic`: two calls for the same soldier are equal
  - `skills_in_range`: for 150 soldiers at xp 0, every skill is in [40, 80]
  - `xp_adds_exactly_twenty`: the same soldier with `xp = 1000` has each skill exactly 20 higher than with `xp = 0`
  - `out_of_range_is_none`: `slot == people` returns `None`
  - `dead_soldier_still_has_stats`: a `Dead` soldier returns `Some` with `health == Dead`
- [ ] **Step 2:** Run `cargo test -p scalia-core --test stats`; expect FAIL.
- [ ] **Step 3:** Implement `soldier_stats` and the constants.
- [ ] **Step 4:** Run; expect PASS.
- [ ] **Step 5:** Commit: `feat: per-soldier stats`.

---

### Task 8: Scenario, state hash and determinism

**Files:**
- Create: `crates/scalia-core/src/scenario.rs`, `state_hash.rs`
- Test: `crates/scalia-core/tests/determinism.rs`

**Interfaces:**
- Consumes: `World`, `derive_visible`, `mix64`
- Produces:
  - `pub fn build_scenario(seed: u64, groups: u32, soldiers_per_group: u32) -> World`: groups on a square grid with spacing 200_000 mm in `RegionId(1)`, each with `soldiers_per_group` infantry; groups where `g % 10 == 0` march 500_000 mm in +x at 1500 mm/s from start 0; groups where `g % 10 == 1` board a ship at their position offset by (30_000, 0, 0) with layout (ramps 2, doors 3, corridor 200, seat_cols 20) from start 0; the rest stand
  - `pub fn state_hash(w: &World) -> u64`: folds `seed`, `now`, then every group field and every roster record in a fixed order through `mix64`
  - `pub fn derive_hash(w: &World, t: WorldMs, view: &ViewBox, cap: usize) -> u64`: folds every derived individual in output order

- [ ] **Step 1: Write failing tests:**
  - `same_seed_same_hash`: two `build_scenario(42, 200, 150)` worlds ticked to 10_000 ms have equal `state_hash` and equal `derive_hash`
  - `different_seed_different_hash`: seeds 42 and 43 differ
  - `tick_schedule_independent_of_runs`: ticking 1 Hz to 10_000 ms twice gives equal hashes
  - `golden_scenario_hash`: `state_hash` of `build_scenario(42, 200, 150)` ticked to 10_000 ms equals a constant `GOLDEN_STATE_HASH`; first set it to `0`, run, copy the printed actual value into the constant, and re-run (the value must be identical under `cargo test` and `cargo test --release`)
- [ ] **Step 2:** Run `cargo test -p scalia-core --test determinism`; expect FAIL.
- [ ] **Step 3:** Implement the three functions.
- [ ] **Step 4:** Freeze the golden constant as described; run in debug and release; expect PASS in both.
- [ ] **Step 5:** Commit: `feat: scenario builder, state hash and determinism tests`.

---

### Task 9: Benchmarks

**Files:**
- Create: `crates/scalia-bench/Cargo.toml`, `src/bin/sim_bench.rs`, `src/bin/hash_scenario.rs`

**Interfaces:**
- Consumes: `build_scenario`, `World::tick`, `derive_visible`, `state_hash`, `derive_hash`
- Produces: `sim_bench` prints `tick_1m_soldiers_ms=<median> budget=5 PASS|FAIL` and `derive_20k_ms=<median> budget=2 PASS|FAIL`, exits 1 on any FAIL unless `--report-only` is passed; `hash_scenario <seed>` prints the decimal value of `state_hash XOR derive_hash` for `build_scenario(seed, 200, 150)` ticked to 10_000 ms with a view covering the grid and `cap = 5000`

- [ ] **Step 1:** Implement `sim_bench`: build `build_scenario(42, 6_667, 150)` (1,000,050 soldiers); run 3 warm-up ticks; take the median of 30 ticks and the median of 30 derivations of 20,000 individuals (`cap = 20_000`).
- [ ] **Step 2:** Implement `hash_scenario`.
- [ ] **Step 3:** Run `cargo run --release -p scalia-bench --bin sim_bench -- --report-only`; expect both lines printed with numbers.
- [ ] **Step 4:** Run without `--report-only`; expect PASS lines and exit 0. If a line is FAIL, profile and optimise; do not change the budgets, and record the numbers in `STATUS.md`.
- [ ] **Step 5:** Commit: `feat: benchmarks`.

---

### Task 10: WebAssembly bridge and native-versus-WebAssembly check

**Files:**
- Create: `crates/scalia-wasm/Cargo.toml` (`crate-type = ["cdylib"]`, `wasm-bindgen` pinned), `src/lib.rs`, `scripts/check_wasm.mjs`

**Interfaces:**
- Consumes: `build_scenario`, `World`, `derive_visible`, `state_hash`, `derive_hash`
- Produces (exported to JavaScript):
  - `scenario_hash(seed: u64) -> u64`: same computation as `hash_scenario`
  - `class Sim`: `constructor(seed: u64, groups: u32, per_group: u32)`, `tick(now_ms: f64)`, `derive(t_ms: f64, region: u32, min_x: f64, max_x: f64, min_y: f64, max_y: f64, cap: u32) -> u32` (fills the output buffers, positions as `f32` metres relative to the view centre), `xs() -> Float32Array`, `ys() -> Float32Array`, `zs() -> Float32Array`, `phases() -> Uint8Array`, plus `xs_ptr()`, `ys_ptr()`, `zs_ptr()`, `phases_ptr()` for zero-copy views by the viewer later

- [ ] **Step 1:** Run `cargo build -p scalia-wasm --target wasm32-unknown-unknown --release`; run `cargo install wasm-bindgen-cli --version <version in Cargo.lock> --locked` if the CLI is missing; run `wasm-bindgen --target nodejs --out-dir crates/scalia-wasm/pkg target/wasm32-unknown-unknown/release/scalia_wasm.wasm`.
- [ ] **Step 2: Write `scripts/check_wasm.mjs`:** it loads the package, then asserts (a) `scenario_hash(42n)` equals the output of `cargo run -q --release -p scalia-bench --bin hash_scenario -- 42` (compared as BigInt); (b) a `Sim(42n, 200, 150)` ticked to 10_000 ms returns a `derive` count between 1 and `cap` and `xs()` has exactly that length with all finite values; (c) a view in region 2 returns count 0.
- [ ] **Step 3:** Run `node scripts/check_wasm.mjs`; expect `OK` for all three checks and exit 0.
- [ ] **Step 4:** Commit: `feat: WebAssembly bridge and native-versus-WebAssembly check`.

---

### Task 11: Status record and full verification

**Files:**
- Create: `STATUS.md`

- [ ] **Step 1:** Run `cargo test --workspace` and `cargo test --workspace --release`; expect all PASS.
- [ ] **Step 2:** Run `cargo run --release -p scalia-bench --bin sim_bench` and `node scripts/check_wasm.mjs`; expect PASS and OK.
- [ ] **Step 3:** Write `STATUS.md` listing the five acceptance checks with their measured numbers and PASS or FAIL, the decisions made where the plan left a choice, and the open items: the persistent-individual table (only the flag exists), vehicle derivation, and anything that needs a real GPU.
- [ ] **Step 4:** Commit: `docs: Milestone 0 status`.
