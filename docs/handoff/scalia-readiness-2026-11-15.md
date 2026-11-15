# Scalia side-development readiness and work orders

Date: 15 November 2026
Repository: Hound-Protocol/Scalia
Execution branch: work/scalia-readiness-2026-11-15
Baseline: main at 1ce63601f9755e9881bcbed7f7445f8f5adc558a
Safety snapshot: safety/baseline-main-1ce6360-2026-11-15 (same commit as baseline)
Execution branch at this check: bb3f31508897d70d333dae953181175e68ffebce (one commit ahead of main, zero behind)

## Purpose and operating boundary

This file is the execution index for the isolated side-development effort. It does not replace the detailed AI, animation, audio, or game-direction handoffs. It records the verified starting point, the immediate engineering work orders, and the standards used to decide whether work is complete.

Hard rules:
- Never write, commit, push, force-push, or merge to main.
- Work only on work/scalia-readiness-2026-11-15 unless the project lead explicitly approves a different isolated branch.
- Keep the safety branch intact. It is a recovery reference, not a substitute for branch protection.
- Read the current branch tip and file SHA before every write. Use compare-and-swap updates; stop if the branch moved unexpectedly.
- Prefer small, reversible changes. Roll back published commits with git revert; do not rewrite shared history.
- Do not remove or replace working systems without a tested reason and a written migration path.
- A feature is not complete because a pose, enum, menu item, or code path exists. Require the relevant automated test and, for animation or visuals, an in-game or generated visual check.
- Report only tests actually run. A passing Pages build is not proof that the full Rust test and benchmark suite passed.
- Do not open or merge a pull request without the project lead's approval.

## Verified starting point

- The repository is public and the connected GitHub account has admin, push, and pull permissions.
- Current refs: `main` and `safety/baseline-main-1ce6360-2026-11-15` remain at `1ce63601f9755e9881bcbed7f7445f8f5adc558a`; the execution branch is `bb3f31508897d70d333dae953181175e68ffebce`, one commit ahead and zero behind main.
- GitHub reports `protected=false` for main, the work branch and the safety snapshot. The safety branch is therefore not immutable; the no-write rule is operational, not GitHub-enforced.
- The current work-branch tip is the documentation-only commit `Add safe Scalia side-development work orders and audit`. Its combined commit status has no reported checks.
- The latest Pages run is a successful deployment for main SHA `1ce63601f9755e9881bcbed7f7445f8f5adc558a` (15 Nov 2026, 05:35:46 UTC: https://github.com/Hound-Protocol/Scalia/actions/runs/43681755469). It is not a run for the work-branch commit.
- The Pages workflow builds the WASM app and deploys GitHub Pages. It does not run `scripts/verify.sh` or `cargo test --workspace`.
- scripts/verify.sh is the substantive validation sequence: workspace tests in debug and release, animation audit, simulation budget, battle budget, WASM build, and native/WASM checks. scripts/check.sh is a quiet logging wrapper around verify.sh.

## Architecture map (first-pass)

- crates/scalia-core: deterministic integer simulation primitives and world systems.
- crates/scalia-ai: tactical battle simulation, soldier minds, squad leadership, perception, tactics, and AI tests.
- crates/scalia-blueprint: vehicle blueprint parsing, validation, drawing, and mesh generation.
- crates/scalia-render: Rust/wgpu renderer, procedural poses, mesh/camera code, WGSL shaders, and renderer tests.
- crates/scalia-wasm: browser-facing WASM bridge, commander/soldier animation controller, battle-to-render integration, and viewer.
- crates/scalia-audio: acoustic propagation and occlusion model (listener environment, wall crossings, echoes and field-based occlusion); browser playback/spatial source construction lives in web/audio.js.
- crates/scalia-world: RON catalog/map loading, placed objects, building references, doors, interaction spots and the static testing-ground definition (crates/scalia-world/src/lib.rs:135-154, 159-193, 243-294).
- crates/scalia-bench: simulation and battle benchmark executables.
- web/: browser shell and page modules (game/menu/audio/debug/console), assets, and sound files.
- blueprints/: shipped vehicle data; docs/ and docs/handoff/: design, plans, and agent handoffs.

Important ownership boundary: scalia-ai owns tactical decisions; scalia-wasm adapts simulation state into per-figure animation drives; scalia-wasm/src/anim.rs owns the shared Animator; scalia-render owns procedural pose construction and the WGSL-side rendering implementation. Keep those boundaries explicit when adding features.

## Findings from the starting-point audit

### Confirmed from current source

1. The shared animation controller exists at crates/scalia-wasm/src/anim.rs. Its module documentation says it is used for the commander, soldiers, and enemies. Drive/Frame/Animator separate controller intent from the drawn pose.
2. The prone drop/get-up transition is implemented through Pose::Drop and the drop phase. The animator tests include going_prone_kneels_plants_and_lies_down_then_gets_back_up and crouching_joins_the_drop_at_the_knee_and_a_change_of_mind_turns_back. The handoff that lists all stand/crouch/prone transitions as missing is stale at least for this tested controller path. Visual verification and checking every caller remain part of the animation audit.
3. crates/scalia-ai/tests/reference_squad.rs exists and contains the ten-step reference squad test plus a separate outmatched-squad-abandons-the-attack test. The newer work-orders-ai-animation.md marks AI-3 complete. The older ai-animation.md still lists the reference squad as future work; update the older handoff rather than reimplementing AI-3.
4. crates/scalia-ai/src/battle.rs currently collects occupied cover into a taken list once before the soldier decision loop. The earlier O(N^2) repeated scan described in the 14 November audit has already been changed; do not reapply that optimization.
5. The verification script includes a benchmark gate for `battle_bench --check 1000`, in addition to tests, animation audit, aggregate budget checks and WASM consistency checks (`scripts/verify.sh`).
6. Needs drain is already frame-rate invariant: `scalia-core/src/world.rs:28-35` uses cumulative absolute-time drain; `scalia-core/tests/world.rs:157-174` asserts exact equality for 30 fps, 144 fps and a single large tick.
7. GPU surface acquisition errors are still collapsed: `scalia-render/src/gpu.rs:498-503` accepts success/suboptimal, then reconfigures and skips for every other result. Timeout, outdated/lost surface and out-of-memory/device-loss handling are not distinguished there.
8. The intro zoom target is still the widest field: `web/menu.js:28, 481` uses `FIELD_M = 50`, while `viewer.rs:97-98, 1894-1897` maps `end_intro()` to `STEP_FIELD_MAX`. This conflicts with the 15 Nov requirement in work-order-v2 H to end at the closest on-foot zoom.
9. The audio architecture is present but not browser-verified in this pass: `scalia-audio/src/lib.rs:223-258, 329-333` models acoustic fields/occlusion; `web/audio.js:108, 181, 288` creates the Web Audio context, decodes buffers and creates playback sources.

### Verified workflow and process gaps

1. .github/workflows/ currently contains the Pages workflow only. There is no dedicated pull-request or all-branch workflow that runs scripts/check.sh / scripts/verify.sh.
2. pages.yml triggers on pushes to main and workflow_dispatch. A side-branch push does not automatically build a safe preview. Do not use the production Pages deployment as a casual branch-preview mechanism.
3. The branch endpoints report protected=false for main, the work branch, and the safety snapshot. A mistaken push can therefore reach main unless the agent's branch checks are followed.

### Documentation drift to reconcile

- The older ai-animation.md had marked AI-3 as future work and listed prone transitions as missing. This pass reconciles it with work-orders-ai-animation.md and current controller/test evidence; aimed-state and visual sign-off remain open.
- STATUS.md had GPU timestamp timing listed as both implemented and open. The stale open item is removed; distinct surface-error handling is now called out instead.
- README.md now explains that scripts/check.sh is the quiet wrapper and scripts/verify.sh prints the stages; STATUS.md is explicitly labelled historical milestone evidence.
- work-order-v2.md now marks the old cover-search and needs-drain issues as resolved in source/tests, rather than asking the next agent to redo them.

## Work order 001 — Safe side-branch operation

Priority: Critical. Owner: every coding agent. Dependencies: none.

Objective: prevent accidental writes to main and ensure every change can be rolled back without rewriting shared history.

Files/systems: CLAUDE.md; this document; Git branch/ref workflow; optionally add a preflight script after WO-002 verifies the local development environment.

Steps:
1. Before editing, run git status --short --branch and git branch --show-current. Require the exact execution branch name in the header of this document.
2. Fetch origin and record origin/main, origin/safety/baseline-main-1ce6360-2026-11-15, and the execution branch SHAs. If the execution branch is missing, create a new isolated branch from the latest verified main only after preserving any previous work branch and confirming it is genuinely absent.
3. If main moves during work, stop and compare histories. Do not silently rebase or reset the work branch.
4. Before each GitHub file write, fetch the target file on the work branch and use its current blob SHA. Before moving the branch ref, require the previously observed branch SHA as the expected SHA.
5. Review the diff and run the required tests before pushing. Use a normal fast-forward update; do not force-push.
6. Use small commits with a player-readable description and the repository's Update: <Area> trailer. Keep implementation, tests, and changelog text consistent.
7. To undo a published commit, create a revert commit on the work branch. Never use reset --hard or force-push as a rollback for shared commits.
8. A PR, merge, deployment to production, or change to branch protection requires explicit project-lead approval.

Acceptance criteria:
- The agent reports the current branch and base SHA before editing.
- A deliberate preflight on main is rejected before any write; the execution branch passes.
- A stale expected SHA causes the write to stop rather than overwrite newer work.
- A test commit can be reverted with a new commit and the work branch remains readable.
- main and the safety snapshot remain unchanged throughout the task.

Risks/rollback: if any write reaches the wrong branch, stop immediately, do not make further commits, capture the affected refs and commit SHA, and notify the project lead. Do not attempt an unapproved force-reset. Revert only after the affected history and dependencies are reviewed.

## Work order 002 — Repository and architecture audit

Priority: Critical. Owner: ChatGPT / Claude. Dependencies: WO-001.

Objective: produce an evidence-backed map of what is implemented, partially implemented, duplicated, unconnected, untested, or only described in design docs.

Files/systems: all eight Rust crates, web/, blueprints/, scripts/, .github/workflows/, tests, benches, STATUS.md, and docs/superpowers/.

Steps:
1. Inventory every crate's public modules, dependencies, entry points, and tests. Identify actual runtime call paths from web/game.js through WASM to simulation, AI, rendering, and audio.
2. For each feature in the current handoffs, record source symbol, caller, automated test, visual/lab proof, and status (complete / partial / static / disconnected / missing / unverified).
3. Audit determinism boundaries: fixed simulation step, integer/fixed-point state, iteration order, random streams, save/replay, and native/WASM parity.
4. Audit performance hot paths using the existing bench binaries first. Record median, p99, worst case, workload size, and hardware/runtime. Do not optimize from intuition alone.
5. Map browser/WebGPU compatibility and error paths, including device loss, surface acquisition, memory pressure, audio startup, and page lifecycle.
6. Identify architectural duplication and oversized modules, but split files only when a change can preserve behavior and tests.
7. Write findings as verified observations with file paths and symbol/test names. Separate measured facts from hypotheses.

Acceptance criteria:
- All eight crates and the browser runtime are represented in the architecture map.
- Every high-priority feature in the current handoffs has an evidence-backed status.
- Existing golden hashes and test/benchmark baselines are recorded before behavior-changing work.
- Each finding has impact, reproduction/evidence, priority, proposed fix, and acceptance test.

Risks/rollback: this work order is audit-first. Do not refactor or change simulation behavior merely to make the architecture look cleaner. Any implementation discovered to be unsafe becomes a separate, test-backed work order.

## Work order 003 — Documentation and handoff reconciliation

Priority: Critical. Owner: ChatGPT / Claude. Dependencies: WO-001 and the source inventory from WO-002.

Objective: make the README, agent instructions, milestone status, and handoffs describe the same current project without erasing design intent.

Files/systems: README.md, STATUS.md, CLAUDE.md, docs/handoff/work-order-v2.md, docs/handoff/work-orders-ai-animation.md, docs/handoff/ai-animation.md, docs/handoff/combat-readiness.md, and relevant superpowers specs/plans.

Steps:
1. Establish ownership: README = project overview and top-level commands; STATUS = verified milestone/release state; CLAUDE = mandatory agent rules and repo map; work-orders-ai-animation = active AI/animation queue; ai-animation = architecture and honest implementation status; work-order-v2 = original consolidated project-lead requirements.
2. Update stale AI-3 status in ai-animation.md to reflect the reference_squad.rs tests and the newer work-order. Preserve AI-4 as the active next track.
3. Update the animation status for prone transitions to say that the code path and automated tests exist. Keep visual sign-off as unverified until a stance sheet or in-game capture is checked.
4. Resolve the contradictory GPU timing item in STATUS.md by matching the status text to actual HUD and benchmark behavior.
5. Explain scripts/check.sh as a quiet wrapper around scripts/verify.sh in README and CLAUDE, so contributors know both names lead to the same full suite.
6. Update STATUS only from tested source/CI evidence. Mark any old hardware measurements with their environment and date; never present them as universal browser performance.
7. Add links between the active work-order queue and the architecture/status docs. Avoid duplicating long design specs.

Acceptance criteria:
- No completed item is still listed as not started in another authoritative handoff.
- No feature is labelled complete without code plus required tests; animation additionally requires visual proof.
- Commands, deployment triggers, and CI claims agree with actual scripts and workflows.
- Unverified, old, or hardware-specific measurements are labelled clearly.

Risks/rollback: documentation edits can change project direction if design intent is rewritten. Preserve original requirements and append a dated correction when intent is ambiguous. Revert only the documentation commit if a reconciliation changes meaning incorrectly.

## Work order 004 — Continuous verification for side branches

Priority: High. Dependencies: WO-001.

Objective: make every pull request and development branch run the same validation suite before it is considered merge-ready.

Files/systems: .github/workflows/verify.yml (new), scripts/check.sh, scripts/verify.sh, rust-toolchain.toml, Node setup, wasm-bindgen CLI.

Steps: add a separate CI workflow for pull_request and push to development branches; install the pinned Rust toolchain, wasm32 target, Node version, and wasm-bindgen CLI; run scripts/check.sh; preserve logs/artifacts on failure; keep Pages deployment separate and main-only.

Acceptance criteria: a deliberately failing test blocks the PR; a passing run shows all verify.sh stages; the workflow does not deploy Pages; no secrets are required.

## Work order 005 — AI and animation continuation

Priority: High. Dependencies: WO-002 and WO-003.

AI: continue from AI-4 in docs/handoff/work-orders-ai-animation.md, not the already-completed AI-3. Preserve the stated sequence: tactical memory and contact ageing; sight/hearing/radio-only knowledge with delayed reports; Combat readiness; interruptible plans and recovery; then platoon/company command. Keep the reference squad as a regression test and retain stable hash_battle unless an intended behavior change is documented.

Animation: verify the current shared Animator wiring and transition visuals first. Then work through the active queue: life in still poses; backward/sideways crawl and prone aim/fire; casualty locomotion/reactions/carry/help-up; pistol and weapon-class handling; terrain moves; animation LOD. Connect AI reactions to animation states. Every change needs an animation audit and a stance sheet/in-game capture.

Acceptance criteria: all AI behaviors have deterministic lab tests; animation is not marked complete from a static pose alone; measured per-figure CPU cost stays within the agreed scale budget.

## Work order 006 — Audio, menu/intro, rendering and performance audits

Priority: High after WO-002.

Audio: begin from the latest Audio update rather than reimplementing it. Verify spatial attenuation, occlusion, door/window propagation, distant gunfire, ambience transitions, footsteps, and browser autoplay/recovery behavior.

Menu/intro: use section D and H of docs/handoff/work-order-v2.md as the acceptance source. Check music startup and gapless looping, caution-window sequencing, cinematic camera beats, and the post-eye-open zoom target.

Rendering/performance: retain the Rust/wgpu renderer and WGSL parity tests. Measure scene preparation, animation, mesh building, uploads, GPU passes, allocations, navigation, and memory before optimization. Keep GPU device/surface failures distinct and recoverable where possible.

Acceptance criteria: each subsystem has a reproducible test or browser verification note, before/after measurements for performance changes, and a rollback path.

## Immediate execution order

1. WO-001 safety rules (this commit also adds the concise mandatory rules to CLAUDE.md).
2. WO-002 full repository audit and evidence matrix.
3. WO-003 reconcile stale handoff and status claims.
4. WO-004 add CI verification on development branches and pull requests.
5. Continue WO-005 AI/animation and WO-006 audio/menu/rendering from the newest handoffs.

## Audit limitation

This first pass used the repository tree, current docs, targeted source/test excerpts, and GitHub Actions metadata. The latest Pages workflow succeeded, but that workflow does not execute the full verify.sh suite. No local Rust build, browser session, GPU test, or full scripts/verify.sh run was executed during this first pass. Those checks must be run in the development environment before implementation is called verified.
## Current execution audit (15 Nov 2026)

### Repository and architecture evidence

- Workspace membership is defined by the wildcard `crates/*` in the root `Cargo.toml`; the current tree contains eight crates: core, AI, audio, benchmark, blueprint, render, WASM and world.
- Browser entry: `web/game.js` imports error/menu modules, initializes the WASM package and enters the game loop. `crates/scalia-wasm/src/lib.rs` exports the shared animation, assault, effects and motion modules; the viewer is compiled for wasm32.
- Animation ownership is split cleanly: `scalia-wasm/src/anim.rs` defines `Drive`, `Frame` and `Animator`; `assault.rs:379-396` converts each battle soldier to a drive and steps its animator; `viewer.rs:3075+` owns the commander update path. Procedural pose construction and the matching WGSL mirror are in `scalia-render/src/pose.rs` and `wgsl/common.wgsl`.
- AI ownership is in `scalia-ai`: `battle.rs` runs the fixed-step battle; `mind.rs` handles soldier goals; `leader.rs` handles squad plans; `tactics.rs` provides cover selection; `reference_squad.rs` is the current ten-step regression scenario.
- `scalia-world` currently supplies loaded RON map/catalog data and object/door/spot definitions; it is not the same thing as `scalia-core::world::World`'s group/needs simulation.

### Confirmed corrections and remaining gaps

1. **AI-3 is implemented.** `reference_squad.rs:1-3, 34-48, 98-120` documents the 12-riflemen-plus-sergeant scenario, submits one squad move order, checks ten observable steps, and separately checks an outmatched squad abandoning. Keep it as regression coverage; AI-4 is next.
2. **Prone drop/get-up is implemented in controller and pose code.** `anim.rs:317-356, 442-476` and `pose.rs:89-95, 1014-1033, 1523+` show the transition and tests. Do not describe the whole feature as missing. Aimed-state coverage and visual sign-off are still open.
3. **Cover search was already optimized.** `battle.rs:944-950` creates the occupied-cover list once before the soldier decision pass; do not reapply the old O(N²) fix.
4. **Needs-drain rounding is already fixed and tested.** `world.rs:28-35` uses absolute-time cumulative drain; `tests/world.rs:157-174` compares 30 fps, 144 fps and one large tick exactly. Do not add a redundant accumulator for the old fractional-loss issue.
5. **GPU surface recovery remains coarse.** `gpu.rs:498-503` groups every non-success/non-suboptimal result into reconfigure-and-skip. Add typed handling and reason-specific reporting as a separate implementation work order, with tests for each recoverable/fatal case.
6. **Intro zoom target remains wrong against the 15 Nov requirement.** `menu.js:28, 481` ends at the widest 50 m field target and `viewer.rs:97-98, 1894-1897` resets to `STEP_FIELD_MAX`. Change both ends together in a dedicated fix; confirm the exact nearest zoom value from the ladder before editing.
7. **Audio is architecturally split but not acoustically validated here.** `scalia-audio` models propagation/occlusion; `web/audio.js` handles Web Audio playback. Verify real browser behavior, occlusion, doors/windows, ambience and autoplay before claiming completion.

### Verification boundary

- No local shell, Rust toolchain, browser or GPU is available through this GitHub-only pass. `scripts/check.sh` / `scripts/verify.sh` were inspected but not executed.
- The work-branch commit has no reported status checks. The only recent successful workflow found is the main-only Pages deployment listed above; it is not a substitute for the full suite.
- No gameplay code was changed. No PR was opened, no merge/deploy was requested, and main plus the safety baseline were not written.
