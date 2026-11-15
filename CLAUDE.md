# Scalia: notes for agent sessions

Browser co-op tactical command game. Rust (wgpu, WASM) in `crates/`; the page is in `web/`; live at
hound-protocol.github.io/Scalia (GitHub Pages builds on push to main). The current plan and state of play are in
`docs/handoff/` (work order, AI and animation handoff): read those first instead of exploring.

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
- Commits: the description goes into the in-game changelog (write it for players) and carries an `Update: <Area>`
  trailer.

## Keep token use low (the user's standing request)
- Grep and read line ranges; never read whole large files (`viewer.rs`, `battle.rs`, `leader.rs`, `game.js`).
- Keep command output short: `| tail -n 5`, `-q` flags, `scripts/check.sh` rather than `verify.sh`.
- Avoid screenshots. Verify visuals with numbers (pose dumps, measured positions) and take an image only when the
  look itself is the question; then use one small frame or a montage, not a series.
- Don't re-read a file right after editing it; don't repeat results back to the user.
- Batch independent commands into one call; batch edits into one script.
- Short replies to the user: what changed, what's next.
