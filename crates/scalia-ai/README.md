# scalia-ai

Tactical AI (the Titan system): the battle simulation, squad leader drills, each soldier's goals, sight and cover, GOAP planning, doors and building plans, scenarios and saves.

- Depends on: `scalia-core`.
- Key files: `battle.rs` (fire, sight, cover use), `mind.rs` (individual goals), `leader.rs` (squad drills), `tactics.rs` (cover map). `battle.rs` and `leader.rs` are over 2,000 lines each: grep, do not read whole.
- Design docs: [docs/titan/](../../docs/titan/).
- Rule: deterministic. Check `hash_battle` before and after any change.
- Tests: `cargo test -p scalia-ai`.
