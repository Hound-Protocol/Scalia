# Scalia core status

Milestone 0 (core) and Milestone 1 (blueprints, planet, Pages page) are below. Everything is verified with one command: `bash scripts/verify.sh`.

# Milestone 0

Branch `milestone-0`. Measured in the build sandbox (2 CPU cores, no GPU) with `--release`.

## Acceptance checks

| # | Check | Result | Measured |
|---|---|---|---|
| 1 | Tests with golden results pass in debug and release | PASS | 54 tests, 0 failures in both; golden state hash `0xcc763f567496bd24` identical in both |
| 2 | One tick of 1,000,000 soldiers (6,667 groups) under 5 ms | PASS | 0.012 to 0.018 ms median over 30 ticks |
| 3 | Deriving 20,000 individuals under 2 ms | PASS | 0.23 to 0.38 ms median over 30 runs |
| 4 | Identical output on two runs, and identical native and WebAssembly hash | PASS | two runs equal; native = wasm = `11470447963499381888` |
| 5 | 5,000-soldier boarding gives the same progress at any query time | PASS | `five_thousand_boarding_query_invariant`: 200 times, in order and shuffled |

Run with: `cargo test --workspace`, `cargo test --workspace --release`,
`cargo run --release -p scalia-bench --bin sim_bench`, then build the wasm package
(`cargo build -p scalia-wasm --target wasm32-unknown-unknown --release`,
`wasm-bindgen --target nodejs --out-dir crates/scalia-wasm/pkg target/wasm32-unknown-unknown/release/scalia_wasm.wasm`)
and `node crates/scalia-wasm/scripts/check_wasm.mjs`.

## Decisions made where the plan left a choice

- `march_pos` returns `Option<Pos>`: an idle or boarding process has no march position, and a dummy origin would hide bugs.
- Between reaching the ramp top and the seated time, a soldier is `Walking` aboard (ramp top to seat). `Seated` starts exactly at `seated_ms`, so counts match `seated_count`.
- Boarding hashes use `hash4(seed, stream, group, slot)`; formation hashes use `hash4(seed, group, slot, stream)` as the plan specifies. The streams are independent, so the order does not matter.
- A march, or boarding a ship, in another region returns `RegionMismatch` (no teleporting).
- A layout with zero seat columns reports `ZeroSeatCols` before `ZeroThroughput`.
- Needs drain floors per tick, so drift is under one raw unit (2^-32) per tick; the test tolerance is 256 raw units.
- `Sim.tick` throws a JavaScript error if time goes backwards. Positions in `Sim.derive` are `f32` metres relative to the view centre.
- `wasm-bindgen` is pinned to 0.2.130 and the CLI must match (installed with `cargo install wasm-bindgen-cli --version 0.2.130 --locked`).
- `sim_bench` also prints a sanity line (group 5 food after the benchmark ticks) so the optimiser cannot drop the tick work.

## Open items

- Persistent individuals (players, officers, spies): only the `FLAG_PERSISTENT` exclusion exists; the table is not built.
- Only infantry and medics are derived as individuals. Tanks and trucks have counts but no derived vehicles; vehicle derivation waits for the blueprint format.
- `ShipLayout` is hand-set in the scenario. It should come from vehicle blueprints once that format exists.
- Combat, small-fight projectile simulation, economy, networking and persistence are out of scope for this milestone.
- Anything that needs a real GPU is untested here: WebGPU rendering speed, drawing 5,000 to 20,000 individuals, and the PS1-style look. The first scene on GitHub Pages is the test host.

# Milestone 1: blueprints, planet, Pages page

Measured in the same sandbox, `bash scripts/verify.sh` exit 0: 115 tests in each of debug and release, 0 failures; wasm check 11 of 11 OK.

| Check | Result | Measured |
|---|---|---|
| Six original blueprints (RON) load, validate and stay under their triangle budgets | PASS | 3 troopers, tank Bulwark, truck Hauler, transport Ox; golden mesh hash per file |
| Bad files report the field at fault; no panic on overflow or hostile nesting | PASS | 13 validation tests, plus a 200,000-deep nesting probe |
| `ShipLayout` comes from a blueprint and drives core boarding | PASS | `transport_boards_a_group_through_core`: 500 seated from the Ox |
| Planet height is deterministic, in range, continuous through zero, with golden hashes | PASS | 7 height tests, 7 chunk tests; negative chunk indices agree on shared edges |
| 64 by 64 chunk under 1 ms | PASS | 0.39 to 0.42 ms median |
| Native and wasm agree on the scenario hash and on a planet chunk and a mesh hash | PASS | scenario hash `11470447963499381888`; chunk golden `15242347213423968789` |
| Core golden state hash unchanged by this milestone | PASS | `0xcc763f567496bd24` |

## Decisions made where the plan left a choice

- Frame for blueprints: the hull is centred on the origin in x (width) and y (length), z from 0 up. Boxes, wedges and rooms give a minimum corner; a cylinder gives the centre of its base; wedges rise toward +y; cylinders are vertical.
- Planet noise: base lattice 32 km, halving over four octaves, weights 8, 4, 2, 1; heights -2,000 m to 8,000 m. Faces differ by hash only; terrain does not yet continue across a face edge.
- A process counts as running until its scheduled end, so a finished boarding no longer blocks a new march (`ProcessError::Busy` otherwise).
- Groups above 10,000,000 people are refused (`Groups::try_spawn`); `Sim::new` in wasm also refuses scenarios above 20,000,000 people.
- Wasm exports cap `cells` at 1024.

## Open items

- The Pages page draws blueprints as SVG and one chunk on a 2D canvas. It could not be viewed in the sandbox, only built and fetched. The workflow deploys from `main`; nothing is deployed until this branch is merged (or the workflow is run on a branch the `github-pages` environment allows).
- No vehicle is derived as an individual yet, even though blueprints exist. Rendering the meshes (WebGPU), textures, the flight model and the planet-to-planet flight are the next milestone.
- Cross-face terrain continuity, planet-scale (zoomed-out) terrain levels of detail, and a texture for the PS1 look are not built.

# Milestone 2a: orbits and first flight (core)

Done in `scalia-core` and exported to wasm; the renderer (M2b) is next.

| Check | Result |
|---|---|
| Integer sine/cosine (`trig`), error under 1e-6, wraps for any turns value | 5 tests |
| Star system with circular orbits (`system`), pure function of time | 8 tests, golden hash |
| Eased flight that leads a moving target (`flight`), exact arrival | 12 tests, golden plan |
| Flight as a world process; soldiers aboard are not derived; state hash | 5 tests; old golden state hash unchanged |
| Wasm: bodies, flight plan; native/wasm agreement | 9 new checks in `check_wasm.mjs` |

Decisions: orbit radius error is under 1e-6 of the radius (about 27 m on the 90,000 km demo orbit); the planner iterates 12 times; a ship parks 50 km from its body on +y (`PARK_OFFSET`); the demo flight takes about 100 s at a peak of 2,000 km/s.

# Milestone 2b: WebGPU viewer

Verified in a real Chrome on GitHub Pages (software WebGPU adapter, so frame rates are low-end numbers): ships, ground and system scenes draw; the Aster-to-Brine flight runs for about 97 s with a trail.

- Renderer: `web/render.js` (WebGPU only, flat-shaded, 16-colour palette, 480x270 target scaled with nearest-neighbour, "full resolution" toggle), scenes in `web/viewer.js`. Camera and meshes are built in JavaScript; the core stays integer-only and exports `blueprint_mesh`.
- Not built: shader validation outside a browser, instanced figures (soldiers are rebuilt each frame, capped at 400), a continuous zoom from galaxy to one soldier, marching in the viewer, textures.
