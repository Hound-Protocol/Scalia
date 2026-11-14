# Scalia (working title)

A co-op tactical command game in the browser: you are the commander on foot, giving intent to squads whose leaders plan and fight on their own, in one shared universe that zooms from the galaxy down to a single soldier.

Built on **Pack Engine**, Hound Protocol's own engine in Rust: a deterministic integer simulation (world, groups, battle AI with squad leaders, save and replay) and a wgpu renderer (WebGPU in the browser), compiled to WebAssembly.

Live demo (blueprints and a planet chunk, computed in WebAssembly): https://hound-protocol.github.io/Scalia/

## Layout

| Path | What it is |
|---|---|
| `crates/scalia-core` | Deterministic, integer-only simulation: groups, derived individuals, boarding, planet terrain |
| `crates/scalia-ai` | Pack Engine battle AI: sight, cover, movement, fire, squad leaders (GOAP), wounded and casualty care; commands, replay and saves |
| `crates/scalia-blueprint` | Vehicles as RON data: validation, mesh, SVG drawing, boarding layout |
| `crates/scalia-wasm` | The same code compiled to WebAssembly for the browser |
| `crates/scalia-bench` | Budget checks (tick, derive, chunk) |
| `blueprints/` | The six original shipped vehicles |
| `web/` | The static page deployed to Pages |
| `docs/` | Design drafts and implementation plans |

## Check everything

```
bash scripts/verify.sh
```

Needs Rust stable with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.130 (`cargo install wasm-bindgen-cli --version 0.2.130 --locked`) and Node 22. See `STATUS.md` for what is measured and what is open.
