# Scalia Agent Handoff

## Current task
PR #45 — AN-2.1 prone/crawl rifle support
Branch: `work/scalia-an2-1-prone-crawl-repair-2026-11-16`

## CI failure diagnosed (run 43807919617)
The Verify workflow failed in `cargo test --workspace (debug)`:
- `prone_and_crawl_keep_the_rifle_in_both_hands`: prone support hand missed the handguard by 0.282 m.
- `grip_moves_the_arm_and_rifle_no_faster_than_a_blend_allows`: Stand -> Prone joint velocity reached 15.269 rad/s.
- JavaScript syntax checks passed; workflow setup and Rust installation passed.

## Changes made on PR branch
- `crates/scalia-render/src/pose.rs`: reduce reach damping progressively as `prone` increases; replace the abrupt prone grip-cap threshold with continuous interpolation from `GRIP_MAX` to `PRONE_GRIP_MAX`.
- `crates/scalia-render/src/wgsl/common.wgsl`: mirror both changes in the shader.
- Existing 4-variable DLS support-arm solver and regression test remain in place.

## Verification status
- Changes pushed to PR branch in commits:
  - `6cb80b71dc40b0213b95bcc893b22d9ed8906415`
  - `ed5e5e32054db7b1c04db529e431213924f6f56e`
  - `60b1ad55df1edbbe729a8200ae7538d511815f51`
  - `4af3eb8e0e2c8f1d33942d0ff855476db9566544`
- CI has not yet reported a run for the latest head. Do not claim fixed until exact-head CI passes.
- Next: inspect latest PR workflow run, retrieve failing job log if any, iterate on the Rust/WGSL solver in lockstep.
- Visual in-game QA remains outstanding.

## Audit notes
- PR is still draft; do not mark ready or merge without passing CI and visual QA.
- No existing `HANDOFF.md` or `AGENTS.md` was present on the PR branch when checked.
