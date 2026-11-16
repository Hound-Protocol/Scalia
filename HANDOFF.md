# Scalia Agent Handoff

## Active task
PR #45 — AN-2.1 prone/crawl rifle support
Branch: `work/scalia-an2-1-prone-crawl-repair-2026-11-16`

## CI diagnosis
Workflow Verify repeatedly failed in `cargo test --workspace (debug)`.
- Original failures: support hand missed the handguard by 0.282 m; Stand -> Prone joint speed reached 15.27 rad/s.
- Root geometry issue: the original prone right-hand/rifle anchor placed the handguard outside the left arm's reachable workspace.
- Subsequent pose tuning exposed the grounded-rig constraints; keep the right elbow planted and the right forearm raised.
- Latest failure (run 350): all 46 pose tests passed; `wgsl_parity::drop_keys_match` failed because Rust `drop_plant()` changed `lean`/ `twist` but WGSL's generated `DROP_PLANT_P` constant was stale.

## Current implementation
- `crates/scalia-render/src/pose.rs`
  - Prone/crawl keep a supported rifle anchor on the right arm; left support hand is solved through the existing grip IK.
  - Prone torso lean/twist and right-arm parameters are tuned to keep elbow grounded and forearm raised.
  - `GRIP_SWAP` restored to 0.6 to preserve transition timing.
  - Grip correction cap interpolates continuously with prone weight.
  - Four-variable DLS solver includes left-arm abduction.
- `crates/scalia-render/src/wgsl/common.wgsl`
  - Mirrors the Rust pose/IK changes.
  - Updated `DROP_PLANT_P` generated constant to match Rust `drop_plant()`.

## Latest commits
- `0e8dcd78020fd34e6ff744b9c2e117aa3da119e5` — prone/crawl right-arm tuning (46 pose tests passed; WGSL parity still failed)
- `f6b3396aa60bc730d71cb00784e8fed698f7317a` — sync generated drop-plant constant

## Verification
- Latest code head: `f6b3396aa60bc730d71cb00784e8fed698f7317a`
- Exact-head CI pending; do not mark PR fixed until Verify passes.
- Visual in-game QA remains outstanding.
- PR remains draft; do not merge or mark ready without passing CI and visual QA.

## Next steps
1. Check exact-head Verify run for `f6b3396aa60bc730d71cb00784e8fed698f7317a`.
2. If green, review Rust/WGSL parity and inspect rendered prone/crawl poses in-game.
3. If failing, download `scalia-verification-log` and fix the specific failing test; do not weaken tests.
