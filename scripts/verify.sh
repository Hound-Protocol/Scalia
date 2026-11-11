#!/usr/bin/env bash
# Runs every check in order and stops at the first failure.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo test --workspace (debug)"
cargo test --workspace -q
echo "== cargo test --workspace --release"
cargo test --workspace --release -q
echo "== budgets (sim_bench)"
cargo run -q --release -p scalia-bench --bin sim_bench
echo "== wasm build"
cargo build -q --release --target wasm32-unknown-unknown -p scalia-wasm
wasm-bindgen --target nodejs --out-dir crates/scalia-wasm/pkg target/wasm32-unknown-unknown/release/scalia_wasm.wasm
echo "== wasm check (native hash equals wasm hash, blueprints, planet)"
node crates/scalia-wasm/scripts/check_wasm.mjs
echo "ALL CHECKS PASSED"
