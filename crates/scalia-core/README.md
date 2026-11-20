# scalia-core

Deterministic, integer-only simulation of groups and the individuals derived from them: fixed 50 ms steps, navigation, commands, boarding, planet and flight, and the state hash used to prove determinism.

- Depends on: nothing in this workspace.
- Used by: every other crate.
- Rule: no floats in simulation state; no wall-clock time; no randomness outside the seeded generator. Changes that alter battle outcomes must say so (compare `hash_battle` before and after).
- Entry points: `src/lib.rs`, `state_hash.rs`, `hash.rs`, `scenario.rs`.
- Tests: `cargo test -p scalia-core`.
