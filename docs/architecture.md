# Architecture

Scalia is a browser tactical game today, moving to native Windows and Linux builds after alpha. The simulation is deterministic Rust. Rendering, audio and UI read from it and never change battle outcomes.

## Crates

| Crate | Role | Depends on |
|---|---|---|
| `scalia-core` | Deterministic, integer-only simulation: groups, derived individuals, navigation, state hash | none |
| `scalia-world` | World object catalogue (`data/catalog.ron`) and maps: what things are, their states, where people use them | core |
| `scalia-blueprint` | Vehicles as data (RON in `blueprints/`): types, layout, validation, meshes | core |
| `scalia-ai` | Tactical AI (Titan): battle, squad drills, each soldier's goals, cover map, GOAP | core |
| `scalia-audio` | Pack Engine acoustics: how a sound reaches the listener through the world | core, ai, world |
| `scalia-render` | Camera, meshes, the procedural pose rig (`pose.rs`), the WGSL mirror, the wgpu renderer | none of the above |
| `scalia-wasm` | JavaScript bridge, the viewer app, the animator, battle to render instances | all of the above |
| `scalia-bench` | Hash and budget binaries used by CI | core, ai |

## Data flow

simulation (core, ai) -> state -> `scalia-wasm` (animator, assault, viewer) -> `scalia-render` (pose params -> GPU) and `web/` (page logic, audio graph).

Animation reads simulation state and produces pose parameters. It does not write back.

## Determinism rules

- The simulation uses fixed 50 ms steps and integer maths.
- Animation, audio, UI and rendering must not change the battle hash. Check `cargo run -q --release -p scalia-bench --bin hash_battle` before and after.
- The native hash must equal the wasm hash (checked by `scripts/verify.sh`).

## The pose rig and its shader mirror

`crates/scalia-render/src/pose.rs` defines `PoseParams` and every pose. `crates/scalia-render/src/wgsl/common.wgsl` repeats the same maths for the GPU. Parity tests in `crates/scalia-render/tests/wgsl_parity.rs` keep them equal. Any pose change is made in both files in the same commit.

## Web client

`web/game.js` (page logic), `web/menu.js`, `web/audio.js`, `web/console.js`, and the model viewer (`web/models.html`, `web/models.js`). Generated WASM output goes to `web/pkg/` and is not committed. The Pages workflow stamps asset URLs with the commit so browsers do not run a stale copy.

## Large files

`viewer.rs`, `battle.rs`, `leader.rs`, `pose.rs` and `game.js` are each over 1,700 lines. Outline or grep them; do not read them whole. Splitting them is planned as separate work.
