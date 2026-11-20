# scalia-wasm

The JavaScript bridge: the same deterministic core compiled to WebAssembly, plus the viewer app and the animator.

- Depends on: all other crates except `scalia-bench`.
- `src/viewer.rs`: the app (over 4,500 lines; outline or grep, do not read whole).
- `src/anim.rs`: the one animation library every figure uses. `assault.rs` turns battle state into render instances; `motion.rs`, `effects.rs`, `ground.rs`, `hints.rs` support them.
- Output: `web/pkg/` (generated, not committed).
- Checked by `scripts/verify.sh`: native hash equals wasm hash.
