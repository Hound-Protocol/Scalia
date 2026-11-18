# Contributing to Scalia

Start with the [README](README.md) for the repository map and [HANDOFF.md](HANDOFF.md) for the current lane. This document applies to human and AI contributors.

## Work orders and handoffs

- Keep the work-order ID stable across revisions, branches, PRs, and handoffs. If a work order has no ID yet, record one in the work order or PR before review. Use a short scope slug in the branch only for maintenance without a formal work order.
- Record the work-order ID, revision (for example `v1.0`), status, date, scope or acceptance criteria, and the next action. Bump the work-order revision when scope or acceptance criteria change materially; implementation progress alone is a status update.
- Record the relevant base or implementation SHA, PR, and verification evidence when available. Keep **in progress**, **merged**, **deployed**, and **visually verified** distinct. Verify current state with Git and CI, and confirm deployment separately.

## Branch model

| Branch | Purpose | Rule |
|---|---|---|
| `main` | Integration and current live build | All feature work enters through a pull request. |
| `stable` | Release-ready baseline | Update only through a reviewed, verified pull request from `main`. |
| `milestone-0` | Historical milestone snapshot | Preserve; do not use for active development. |
| `safety/*` | Recovery snapshots | Preserve until the project lead explicitly approves a retention policy. |

The repository currently uses `main` as its default branch. Any planned rename to `Build` is not complete until the GitHub default branch, Pages workflow, CI filters, links, and collaborator instructions are migrated together.

## Work branches

Use one branch per focused work order:

```text
<area>/<ticket-or-work-order>-<short-description>
```

Examples: `anim/an-02-s03-prone-aim-transitions`, `ui/ui-05-inventory-panel`, `fix/startup-console-init`.

- Branch from the latest `main` unless the work order explicitly names another base.
- Keep the diff focused; do not bundle unrelated cleanup.
- Do not force-push shared branches. If history must be rewritten, coordinate first.
- Do not commit directly to `main` or `stable`.

## Pull requests

- Target `main` for normal development.
- Target `stable` only for an approved release promotion or urgent backport.
- For work-order PRs, title the PR `<WORK-ORDER-ID>: <short change summary>`, for example `AN-02-S03: Keep prone aim steady through crawl transitions`. For maintenance without a work order, use an explicit area such as `Docs: Clarify setup prerequisites`. Do not leave the raw branch name as the title.
- Keep PRs small and independently reviewable.
- Fill in the [PR template](.github/PULL_REQUEST_TEMPLATE.md): reference the work order and revision, explain what changed and why, list acceptance evidence, and say what remains open.
- Record the tested SHA, checks run, and the result of manual/browser validation where applicable. Include the build and environment for observations that depend on the platform.
- Keep drafts open while implementation or required validation is incomplete.
- Do not merge while required checks fail or while a material review concern remains unresolved.

## Engineering standards

- **Determinism:** Keep the simulation authoritative and presentation separate. Compare before/after hashes on the same seed and duration for simulation-neutral work. If a change is meant to alter the simulation, document why the hash changed and validate the new behaviour with a repeatable test.
- **Compatibility:** Review changes to saves, replays, content data, and native/WASM interfaces for compatibility. For a breaking change, identify the affected formats and agree on a version and migration plan before merging.
- **Rendering and animation:** Keep shared behaviour aligned between Rust and WGSL, add or update parity tests, and run the animation audit. Review a stance sheet or in-game capture for animation work; for live transitions, check motion in the browser, not only still frames.
- **Performance:** Avoid adding unmeasured cost that grows with the number of agents. Use representative benchmarks, document the load and environment, and do not loosen budgets to hide a regression.
- **Scope:** Prefer focused fixes over unrelated refactors. Keep generated build output out of the PR and record new limitations in the relevant work order or handoff.

## Verification

Run the repository's canonical check before requesting review:

```bash
bash scripts/check.sh
```

For browser-facing changes, also test the affected flow in a real browser and report the environment and result. For audio, rendering, input, or other perceptual changes, include a concise A/B or reproduction note; automated tests do not replace listening or visual validation.

When a check cannot run, record what was blocked and why. Do not mark a missing check as passed or treat an automated pass as browser sign-off.

## Release identity

The **PR title** identifies the work. The **commit SHA** identifies the exact code. The **release series, generated patch number, and update label** identify a deployed build. Do not substitute one for another.

Include exactly one standalone `Update: <Area>` line in both the PR description and the final commit message, with the same value. Choose a meaningful, consistent area, for example `Animation`, `Audio`, or `Docs`. Do not use `Build`. Make the commit subject and body understandable to players; the release generator uses them for the in-game changelog.

```text
Keep prone aim steady through crawl transitions

The rifle stays on target as the soldier starts or stops crawling.

Update: Animation
```

[VERSION](VERSION) declares the release series. [The Pages workflow](.github/workflows/pages.yml) runs [the release metadata script](scripts/release_metadata.py) on `main`; the script derives the patch number and per-area update counter from first-parent history and stamps the build SHA, release string, PR reference where available, and changelog. Do not hand-edit generated version or changelog files or bump `VERSION` on routine work orders. Treat a change to the release series as a separate approved release decision. A merge is not proof of deployment; check the [live build identity](https://hound-protocol.github.io/Scalia/version.json).

## Branch retirement

After a feature PR is merged, its work branch may be deleted once the merge is confirmed and no open PR or dependent work uses it. Keep `main`, `stable`, `milestone-0`, and all `safety/*` snapshots. Do not delete an unmerged, draft, or active work branch as part of routine cleanup.

## Repository administration

GitHub branch protection/rulesets should enforce pull requests and successful verification on `main` and `stable`. Documentation alone does not enforce these settings; an organization/repository administrator must configure them in GitHub.
