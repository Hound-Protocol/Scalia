# Contributing to Scalia

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

Examples: `audio/au-10-room-tone`, `ui/ui-05-inventory-panel`, `fix/startup-console-init`.

- Branch from the latest `main` unless the work order explicitly names another base.
- Keep the diff focused; do not bundle unrelated cleanup.
- Do not force-push shared branches. If history must be rewritten, coordinate first.
- Do not commit directly to `main` or `stable`.

## Pull requests

- Target `main` for normal development.
- Target `stable` only for an approved release promotion or urgent backport.
- Keep PRs small and independently reviewable.
- State what changed, why, and any known limitations.
- Include automated test results and manual/browser validation where applicable.
- Keep drafts open while implementation or required validation is incomplete.
- Do not merge while required checks fail or while a material review concern remains unresolved.

## Verification

Run the repository's canonical check before requesting review:

```bash
bash scripts/check.sh
```

For browser-facing changes, also test the affected flow in a real browser and report the environment and result. For audio, rendering, input, or other perceptual changes, include a concise A/B or reproduction note; automated tests do not replace listening or visual validation.

## Commit metadata

The Pages workflow reads the `Update:` commit trailer to group shipped changes in the in-game changelog. Use a meaningful area name:

```text
Implement acoustic transmission tuning

Update: Audio
```

Use one consistent area per commit. Avoid vague values such as `Misc` or `Fix`.

## Branch retirement

After a feature PR is merged, its work branch may be deleted once the merge is confirmed and no open PR or dependent work uses it. Keep `main`, `stable`, `milestone-0`, and all `safety/*` snapshots. Do not delete an unmerged, draft, or active work branch as part of routine cleanup.

## Repository administration

GitHub branch protection/rulesets should enforce pull requests and successful verification on `main` and `stable`. Documentation alone does not enforce these settings; an organization/repository administrator must configure them in GitHub.
