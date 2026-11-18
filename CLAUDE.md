# Scalia: notes for agent sessions

Start with [README.md](README.md), [AGENTS.md](AGENTS.md), and [CONTRIBUTING.md](CONTRIBUTING.md).
Read [HANDOFF.md](HANDOFF.md) and the work order for your lane before exploring. This file is a code map and local workflow aid, not a separate contribution policy.

## Map
- `crates/scalia-ai`: the battle simulation (fixed 50 ms steps, deterministic). `battle.rs` (fire, sight, cover use),
  `mind.rs` (each soldier's goals), `leader.rs` (squad drills), `tactics.rs` (cover map).
- `crates/scalia-render`: `pose.rs` (procedural poses), mirrored by hand in `wgsl/common.wgsl`; parity tests keep
  them equal. `gpu.rs` renderer.
- `crates/scalia-wasm`: `viewer.rs` (the app; very large: outline it or grep, don't read it whole), `anim.rs` (the one
  animation library every figure uses), `assault.rs` (battle to instances), `motion.rs`, `effects.rs`.
- `web/`: `game.js` (page logic), `menu.js` (menu and intro), `audio.js`, `console.js`.

## Checks
- `scripts/check.sh`: the full verification, quiet (one line on success). Run it before every push.
- Determinism: `cargo run -q --release -p scalia-bench --bin hash_battle` must give the same hash before and after a
  change, unless the change is meant to alter the battle.
- Follow the [PR naming](CONTRIBUTING.md#pull-requests) and [release identity](CONTRIBUTING.md#release-identity) rules. Write commit subjects and bodies for players; they appear in the in-game changelog.

## Keep token use low (the user's standing request)
- Grep and read line ranges; never read whole large files (`viewer.rs`, `battle.rs`, `leader.rs`, `game.js`).
- Keep command output short: `| tail -n 5`, `-q` flags, `scripts/check.sh` rather than `verify.sh`.
- Avoid screenshots. Verify visuals with numbers (pose dumps, measured positions) and take an image only when the
  look itself is the question; then use one small frame or a montage, not a series.
- Don't re-read a file right after editing it; don't repeat results back to the user.
- Batch independent commands into one call; batch edits into one script.
- Short replies to the user: what changed, what's next.
