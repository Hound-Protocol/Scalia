# Scalia core: Milestone 0 status

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
