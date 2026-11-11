# Scalia M1: Blueprints, Planet and Pages Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (the user chose inline execution) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Vehicles defined as RON blueprints (validated, meshed, drawn), a deterministic cube-sphere planet height function with chunks and spawn points, wasm exports for both, and a static Pages page that shows them.

**Architecture:** New crate `scalia-blueprint` (depends on `scalia-core` only for `ShipLayout`). Planet code is a new module `planet.rs` in `scalia-core`, integer and fixed-point only. `scalia-wasm` gains three exports. A static page in `web/` loads the wasm; a GitHub Actions workflow builds and deploys it.

**Tech Stack:** Rust 1.97 stable, `serde` + `ron` (blueprints), wasm-bindgen 0.2.130 (pinned), Node 22 for the wasm check script, GitHub Actions + Pages.

**Spec:** `docs/superpowers/specs/2026-11-11-blueprints-and-planet-design.md`

## Global Constraints

- Blueprint files are RON, one per vehicle, in `blueprints/`, all numbers integers (mm, kg, mm/s).
- Triangle budgets: 400 walker, 1,200 tank or truck (and transport), 4,000 ship.
- Files carry `id` and `version`; no owner field. Blueprints are findable items only; no player-variant code.
- Original designs only; style industrial and utilitarian. No names, shapes or marks copied from any game.
- Planet: cube-sphere (six faces); only face 0..=5 sampled as a flat patch; deterministic and identical native and wasm.
- Integer or `Fix` arithmetic only in `scalia-core`; no floats. Floor rounding, saturating arithmetic.
- Chunk budget: 1 ms per 64 by 64 chunk.
- Do not push to `main` and do not merge; push only the working branch and ask before anything else.
- Commit messages end with the two attribution lines supplied by the session.

## Review Focus

- A blueprint with a zero or overflowing dimension (hull 0, primitive size near `u32::MAX`) must return a field-named error, never panic or wrap.
- Two files with the same `id`, or an `id` that is empty or has characters outside `a-z0-9_`, must be rejected by the loader of the shipped set.
- Negative planet coordinates must use floor division, so the value just below zero is not mirrored; chunks at negative indices must still agree on shared edges.
- Malformed or truncated RON handed to the wasm export must return an error to JS, not trap.
- A blueprint with passengers above seats, or rooms outside the hull, must be rejected even if its triangle count is fine.

---

### Task 1: Blueprint types and RON parsing

**Files:**
- Create: `crates/scalia-blueprint/Cargo.toml`, `crates/scalia-blueprint/src/lib.rs`, `crates/scalia-blueprint/src/types.rs`
- Test: `crates/scalia-blueprint/tests/parse.rs`

**Interfaces:**
- Produces: `types::{Class{Walker,Tank,Truck,Transport,Ship}, Hull{length_mm,width_mm,height_mm:u32, mass_kg:u32}, Stats{speed_mm_s:u32, crew:u16, passengers:u16, armour:u16}, Purpose{Bridge,Cargo,Troops,Engine,Crew}, Room{name:String, purpose:Purpose, x_mm:i32, y_mm:i32, w_mm:u32, d_mm:u32}, Interior{ramps:u16, doors:u16, corridor_width_cm:u16, seat_cols:u16, seats:u32, rooms:Vec<Room>}, Prim{Box{x:i32,y:i32,z:i32,w:u32,d:u32,h:u32,color:u8}, Wedge{same fields}, Cylinder{x:i32,y:i32,z:i32,radius:u32,h:u32,color:u8}}, Blueprint{id:String, version:u32, name:String, class:Class, hull:Hull, stats:Stats, interior:Interior, model:Vec<Prim>}}`, all `serde::Deserialize + Clone + Debug + PartialEq`.
- Produces: `BlueprintError{ pub field: String, pub message: String }` (Display, Debug); `fn parse(text: &str) -> Result<Blueprint, BlueprintError>` (parse only in this task).

- [ ] **Step 1: Write failing tests** in `tests/parse.rs`: `parses_a_minimal_valid_file` (inline RON of a walker, asserts `id`, `class == Class::Walker`, one `Box` primitive); `bad_syntax_names_position` (truncated text, error `message` non-empty); `unknown_field_is_rejected` (extra `owner: "x"` field returns `Err` whose `field` or `message` mentions `owner`).
- [ ] **Step 2: Run** `cargo test -p scalia-blueprint --test parse`. Expected: compile failure, crate missing.
- [ ] **Step 3: Create the crate** (workspace picks it up through `crates/*`), add deps `serde = { version = "1", features = ["derive"] }`, `ron = "0.8"`, `scalia-core = { path = "../scalia-core" }`. Implement the types with `#[serde(deny_unknown_fields)]` and `parse` mapping `ron::error::SpannedError` into `BlueprintError` (field = `"<syntax>"` with line and column in `message`; for unknown-field errors set `field` to the named field when the message contains it).
- [ ] **Step 4: Run** the same command. Expected: 3 passed.
- [ ] **Step 5: Commit** `feat(blueprint): types and RON parsing`.

### Task 2: Validation

**Files:**
- Create: `crates/scalia-blueprint/src/validate.rs`; Modify: `lib.rs` (`parse` now validates)
- Test: `crates/scalia-blueprint/tests/validate.rs`

**Interfaces:**
- Consumes: Task 1 types.
- Produces: `pub fn validate(b: &Blueprint) -> Result<(), BlueprintError>`; `pub fn triangle_budget(c: Class) -> u32` (400, 1200, 1200, 1200, 4000 in the order Walker, Tank, Truck, Transport, Ship); `pub fn triangle_count(b: &Blueprint) -> u32` (Box 12, Wedge 8, Cylinder 28). `parse` calls `validate` last.

- [ ] **Step 1: Write failing tests**, one per rule, each asserting the exact `field`: `zero_hull_length_rejected` (`hull.length_mm`), `zero_hull_height_rejected` (`hull.height_mm`), `empty_id_rejected` (`id`), `bad_id_characters_rejected` (`id`, value `"Tank 1"`), `version_zero_rejected` (`version`), `room_outside_hull_rejected` (`interior.rooms[0]`), `passengers_above_seats_rejected` (`stats.passengers`), `over_budget_rejected` (`model`, 40 boxes on a walker = 480 > 400), `primitive_size_overflow_rejected` (`model[0]`, `w = u32::MAX` with `x = i32::MAX`; use `i64` arithmetic so nothing wraps), `primitive_outside_hull_rejected` (`model[0]`), `no_primitives_rejected` (`model`). Plus `valid_file_passes`.
- [ ] **Step 2: Run** `cargo test -p scalia-blueprint --test validate`. Expected: failures (validate missing).
- [ ] **Step 3: Implement** `validate.rs`: all geometry checks in `i64`; the model must sit inside the hull box centred on the origin in x and y with z from 0 to `height_mm` (state this in a doc comment); `seat_cols > 0` required when `class` is `Transport` or `Ship`.
- [ ] **Step 4: Run** the test file. Expected: 12 passed.
- [ ] **Step 5: Commit** `feat(blueprint): validation with field-named errors`.

### Task 3: Mesh generation

**Files:**
- Create: `crates/scalia-blueprint/src/mesh.rs`
- Test: `crates/scalia-blueprint/tests/mesh.rs`

**Interfaces:**
- Consumes: Task 1 types.
- Produces: `Mesh{ pub vertices: Vec<[i32;3]>, pub indices: Vec<u32>, pub colors: Vec<u8> }` (one colour entry per triangle); `pub fn mesh(b: &Blueprint) -> Mesh`; `pub fn mesh_hash(m: &Mesh) -> u64` (FNV-1a over little-endian bytes of vertices, indices, colours).

- [ ] **Step 1: Write failing tests**: `box_has_12_triangles_and_8_vertices`; `wedge_has_8_triangles`; `cylinder_has_28_triangles` (8 sides); `triangle_count_matches_validate_count` (build a mixed blueprint, `mesh.indices.len()/3 == triangle_count(&b)`); `mesh_is_deterministic` (twice equal, hash equal); `all_vertices_inside_hull` (positions within the hull box).
- [ ] **Step 2: Run** `cargo test -p scalia-blueprint --test mesh`. Expected: compile failure.
- [ ] **Step 3: Implement** with integer maths only; the cylinder is an 8-sided prism with its 8 base points on integer offsets from a fixed table (cos and sin of multiples of 45 degrees scaled by `radius` using the numerators 0, 707, 1000 over 1000, floor-divided); the wedge slopes its top from full height at the back (+y) to zero at the front.
- [ ] **Step 4: Run** the test file. Expected: 6 passed.
- [ ] **Step 5: Commit** `feat(blueprint): integer mesh generation`.

### Task 4: ShipLayout and SVG drawing

**Files:**
- Create: `crates/scalia-blueprint/src/layout.rs`, `crates/scalia-blueprint/src/draw.rs`
- Test: `crates/scalia-blueprint/tests/outputs.rs`

**Interfaces:**
- Consumes: Task 1 types; `scalia_core::boarding::ShipLayout{ramps,doors,corridor_width_cm,seat_cols}`.
- Produces: `pub fn ship_layout(b: &Blueprint) -> Option<ShipLayout>` (`Some` only for `Transport` and `Ship`); `pub fn svg(b: &Blueprint) -> String` (top-down: hull outline rectangle, one `<rect>` per room with `data-room="<name>"`, one `<rect>` per primitive footprint, integer coordinates in mm).

- [ ] **Step 1: Write failing tests**: `transport_yields_layout_matching_interior`; `walker_yields_no_layout`; `layout_throughput_is_nonzero_for_shipped_transport` (calls `throughput_milli_per_s`); `svg_contains_every_room` (count of `data-room=` equals rooms); `svg_is_deterministic`; `svg_escapes_names` (room name `a<b&"c"` appears escaped).
- [ ] **Step 2: Run** `cargo test -p scalia-blueprint --test outputs`. Expected: compile failure.
- [ ] **Step 3: Implement** both functions; XML-escape `& < > "`.
- [ ] **Step 4: Run** the test file. Expected: 6 passed.
- [ ] **Step 5: Commit** `feat(blueprint): ShipLayout and SVG outputs`.

### Task 5: Six original blueprints and the shipped-set loader

**Files:**
- Create: `blueprints/trooper_light.ron`, `trooper_medium.ron`, `trooper_heavy.ron`, `tank_bulwark.ron`, `truck_hauler.ron`, `transport_ox.ron`; `crates/scalia-blueprint/src/shipped.rs`
- Test: `crates/scalia-blueprint/tests/shipped.rs`

**Interfaces:**
- Consumes: `parse`, `mesh`, `mesh_hash`, `ship_layout`, `svg`.
- Produces: `pub fn shipped() -> Result<Vec<Blueprint>, BlueprintError>` embedding the six files with `include_str!`; rejects duplicate ids (`field = "id"`).

Designs, all original and utilitarian: three troopers (walker class; light about 1,750 mm tall, medium with a back pack box, heavy with a wide shield wedge), tank "Bulwark" (low hull, wedge glacis, cylinder barrel), truck "Hauler" (cab box, long cargo box, 6 wheel cylinders), transport "Ox" (boxy hull with rear ramp, 2 ramps, 4 doors, corridor 120 cm, 20 seat columns, 600 seats, rooms: bridge, troops, cargo, engine, crew).

- [ ] **Step 1: Write failing tests**: `six_blueprints_load_and_validate`; `ids_are_unique`; `each_blueprint_under_its_triangle_budget`; `golden_mesh_hashes` (a `[(&str, u64); 6]` table; first run prints the values, then paste them in, so the test is red until pasted); `transport_boards_a_group_through_core` (build a `World` via `scalia_core`, call `start_boarding` with `ship_layout(ox)`, tick past the interval, assert seated count equals the group size up to capacity).
- [ ] **Step 2: Run** `cargo test -p scalia-blueprint --test shipped`. Expected: failure (files and loader missing).
- [ ] **Step 3: Write the six RON files and `shipped.rs`.** Paste the golden hashes printed by the first run into the test.
- [ ] **Step 4: Run** the test file. Expected: 5 passed.
- [ ] **Step 5: Commit** `feat(blueprint): six original blueprints`.

### Task 6: Planet height function

**Files:**
- Create: `crates/scalia-core/src/planet.rs`; Modify: `lib.rs` (add `pub mod planet;`), `hash.rs` (add `pub const STREAM_TERRAIN: u64 = 9;`)
- Test: `crates/scalia-core/tests/planet.rs`

**Interfaces:**
- Consumes: `hash::{hash4, STREAM_TERRAIN}`, `fixed::Fix`.
- Produces: `pub const TERRAIN_MIN_MM: i32 = -2_000_000;` `pub const TERRAIN_MAX_MM: i32 = 8_000_000;` `pub const CELL_MM: i64 = 1_000;` `pub fn height_mm(seed: u64, face: u8, x_mm: i64, y_mm: i64) -> i32` (4 octaves of integer value noise on a lattice of 64,000 mm at the base octave, smoothstep blending in `Fix`, floor division for negative coordinates, `face` mixed into the hash so faces differ, result clamped to the two constants; `face > 5` is clamped to 5).

- [ ] **Step 1: Write failing tests**: `same_inputs_same_height`; `different_seed_differs_somewhere` (some point in 100 samples differs); `different_face_differs_somewhere`; `heights_within_range` (10,000 samples including coordinates beyond ±1e12); `continuous_across_zero` (height at x=-1 and x=0 differ by under 5,000 mm); `golden_height_hash` (hash over 4,096 samples on a grid; paste the printed value).
- [ ] **Step 2: Run** `cargo test -p scalia-core --test planet`. Expected: compile failure.
- [ ] **Step 3: Implement** `height_mm` with `i64::div_euclid` and `rem_euclid` for lattice indices and the within-cell fraction as a `Fix`.
- [ ] **Step 4: Run** the test file and `cargo test -p scalia-core`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(core): deterministic planet height function`.

### Task 7: Chunks, spawn points and the budget

**Files:**
- Modify: `crates/scalia-core/src/planet.rs`; `crates/scalia-bench/src/bin/sim_bench.rs`
- Test: `crates/scalia-core/tests/planet_chunks.rs`

**Interfaces:**
- Consumes: `height_mm`, `CELL_MM`.
- Produces: `pub struct Chunk{ pub cells: u32, pub heights: Vec<i32> }` (`(cells+1)^2` samples, row-major, y outer); `pub fn chunk(seed: u64, face: u8, cx: i64, cy: i64, cells: u32) -> Chunk`; `pub fn chunk_hash(c: &Chunk) -> u64`; `pub fn spawn_points(seed: u64, face: u8, cx: i64, cy: i64, cells: u32, count: usize) -> Vec<(i64, i64)>` (points on cells whose four-corner height spread is at most 500 mm, picked in hash order, `count` or fewer returned, never more).

- [ ] **Step 1: Write failing tests**: `same_chunk_same_hash`; `neighbours_agree_on_shared_edge` (east edge of (0,0) equals west edge of (1,0), and the same for (−1,0)/(0,0) and for (0,−1)/(0,0)); `chunk_has_expected_sample_count`; `spawn_points_are_flat_and_within_chunk`; `spawn_points_never_exceed_count`; `golden_chunk_hash`.
- [ ] **Step 2: Run** `cargo test -p scalia-core --test planet_chunks`. Expected: compile failure.
- [ ] **Step 3: Implement** the three functions; add `chunk_64_ms` to `sim_bench.rs` with budget 1 ms (same PASS or FAIL print style, same exit-1 rule).
- [ ] **Step 4: Run** the test file, then `cargo run --release -p scalia-bench --bin sim_bench`. Expected: tests pass; `chunk_64_ms=… budget=1 PASS`.
- [ ] **Step 5: Commit** `feat(core): planet chunks, spawn points and benchmark`.

### Task 8: Wasm exports

**Files:**
- Modify: `crates/scalia-wasm/src/lib.rs`, `crates/scalia-wasm/Cargo.toml` (dep on `scalia-blueprint`), `crates/scalia-wasm/scripts/check_wasm.mjs`

**Interfaces:**
- Consumes: `scalia_blueprint::{parse, svg, mesh, mesh_hash, shipped}`, `scalia_core::planet::{chunk, chunk_hash}`.
- Produces (wasm-bindgen): `blueprint_svg(text: &str) -> Result<String, JsError>`; `blueprint_info(text: &str) -> Result<String, JsError>` (compact JSON: id, name, class, triangles, mesh hash as decimal string); `planet_chunk(seed: u64, face: u8, cx: i32, cy: i32, cells: u32) -> Vec<i32>`; `shipped_ron(index: u32) -> Option<String>` returning the file text of shipped blueprint `index`.

- [ ] **Step 1: Extend `check_wasm.mjs` first** with failing assertions: `blueprint_svg("garbage")` throws (not traps); `blueprint_svg` of shipped 0 contains `<svg`; `planet_chunk(1,0,0,0,8).length === 81`; the wasm `chunk_hash` of that chunk equals the value the native test pins (print native with a small bin or test, paste it).
- [ ] **Step 2: Run** the build and check: `cargo build --release --target wasm32-unknown-unknown -p scalia-wasm && wasm-bindgen --target nodejs --out-dir crates/scalia-wasm/pkg target/wasm32-unknown-unknown/release/scalia_wasm.wasm && node crates/scalia-wasm/scripts/check_wasm.mjs`. Expected: the new assertions fail.
- [ ] **Step 3: Implement** the four exports.
- [ ] **Step 4: Run** the same command. Expected: all checks pass, native hash still equals wasm hash.
- [ ] **Step 5: Commit** `feat(wasm): blueprint and planet exports`.

### Task 9: Hardening of the deferred minors, and one verify script

**Files:**
- Modify: `crates/scalia-core/src/process.rs`, `world.rs`, `groups.rs`; Create: `scripts/verify.sh`
- Test: `crates/scalia-core/tests/hardening.rs`

**Interfaces:**
- Consumes: `World::start_march`, `World::start_boarding`, `Groups::spawn`.
- Produces: `ProcessError::Busy` (new variant); `start_march` and `start_boarding` return `Err(ProcessError::Busy)` when the group's process is not `Idle`, leaving the group unchanged; `Groups::try_spawn(...) -> Result<GroupId, SpawnError>` with `SpawnError::TooLarge` when `composition.total()` exceeds `MAX_GROUP_PEOPLE = 50_000_000`; `Groups::spawn` keeps its signature and panics only through `try_spawn`'s error with a clear message. `scripts/verify.sh` runs, in order and stopping at the first failure: `cargo test --workspace`, `cargo test --workspace --release`, the sim bench, the wasm build, and `check_wasm.mjs`.

- [ ] **Step 1: Write failing tests**: `second_march_while_marching_is_busy`; `boarding_while_marching_is_busy_and_group_unchanged`; `idle_group_can_start_again_after_march_completes`; `try_spawn_rejects_absurd_group`; `try_spawn_accepts_one_million`.
- [ ] **Step 2: Run** `cargo test -p scalia-core --test hardening`. Expected: compile failure.
- [ ] **Step 3: Implement** the variant, the guard and `try_spawn`. Existing tests that restart a running process must be updated to complete the first process first; record each in the ledger as a ruling.
- [ ] **Step 4: Run** `bash scripts/verify.sh`. Expected: every stage passes; the golden state hash `0xcc763f567496bd24` is unchanged.
- [ ] **Step 5: Commit** `fix(core): reject overlapping processes and absurd group sizes; add verify script`.

### Task 10: Pages page and workflow

**Files:**
- Create: `web/index.html`, `web/main.js`, `.github/workflows/pages.yml`

**Interfaces:**
- Consumes: the wasm exports of Task 8, built with `--target web` into `web/pkg/` (git-ignored).
- Produces: a static page that lists the six shipped blueprints with their SVG drawings and triangle counts, and draws one planet chunk (face 0, 64 cells) as a greyscale canvas with a seed box that redraws. No WebGPU; plain DOM and 2D canvas.

- [ ] **Step 1: Write the page and workflow.** The workflow runs on push to `main` and on manual dispatch, installs the pinned toolchain and `wasm-bindgen-cli 0.2.130`, builds with `--target web`, copies `web/` to the Pages artifact, and deploys with the official `actions/upload-pages-artifact` and `actions/deploy-pages` steps.
- [ ] **Step 2: Verify locally** by building `--target web` into `web/pkg`, serving `web/` with `python3 -m http.server`, and fetching `index.html` and `pkg/scalia_wasm.js` with `curl` (status 200). Expected: both 200. A visual check is only possible on Pages; say so in the final message.
- [ ] **Step 3: Run** `node --check web/main.js`. Expected: no output.
- [ ] **Step 4: Commit** `feat(web): Pages page for blueprints and planet chunk`.

### Task 11: Status and ledger

**Files:** Modify `STATUS.md`.

- [ ] **Step 1: Update `STATUS.md`** with an M1 acceptance table (tests count, bench numbers, wasm hash check, golden hashes) taken from real output of `bash scripts/verify.sh`.
- [ ] **Step 2: Run** `bash scripts/verify.sh`. Expected: all stages pass.
- [ ] **Step 3: Commit** `docs: M1 status`.
