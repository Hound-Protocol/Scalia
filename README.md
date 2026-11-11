# Scalia (working title)

A top-down isometric browser strategy game on one shared server: fly through the universe, build cities, sustain an economy and command millions of soldiers, zooming from the galaxy down to a single soldier, in continuous real time.

Live demo (blueprints and a planet chunk, computed in WebAssembly): https://hound-protocol.github.io/Scalia/

## Layout

| Path | What it is |
|---|---|
| `crates/scalia-core` | Deterministic, integer-only simulation: groups, derived individuals, boarding, planet terrain |
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
