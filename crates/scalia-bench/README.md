# scalia-bench

Binaries used by CI to guard determinism and performance.

| Binary | Purpose |
|---|---|
| `hash_battle` | Hash of a seeded battle. Must be unchanged by any simulation-neutral change |
| `hash_scenario` | Hash of a scenario run |
| `sim_bench` | Simulation budgets |
| `battle_bench` | Battle step budget (`--check`, 1,000 in contact) |

Run, for example: `cargo run -q --release -p scalia-bench --bin hash_battle`.
