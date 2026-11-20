# Scalia

Scalia is a browser-based co-op tactical command game built on Pack Engine. You play a commander on foot, giving intent to squads whose leaders plan and act on their own. The same world spans the galaxy down to an individual soldier. Pack Engine uses deterministic Rust simulation, a wgpu renderer, WebAssembly in the browser, and the Auralis Audio Engine.

[Live game](https://hound-protocol.github.io/Scalia/) · [Model viewer](https://hound-protocol.github.io/Scalia/models.html) · [Live build identity](https://hound-protocol.github.io/Scalia/version.json)

## Start here

For humans and coding agents, the path is the same:

1. **Read [CONTRIBUTING.md](CONTRIBUTING.md).** It defines branch and PR naming, work-order revisions, release identity, engineering standards, and verification.
2. **Read [HANDOFF.md](HANDOFF.md), then open the work order for your lane** (the [docs index](docs/README.md) lists every document with its status, and [architecture](docs/architecture.md) maps the crates). Confirm the work-order ID, scope, acceptance criteria, and next action. Coding agents also read [AGENTS.md](AGENTS.md).
3. **Check [`origin/main`](https://github.com/Hound-Protocol/Scalia/tree/main) and [open pull requests](https://github.com/Hound-Protocol/Scalia/pulls).** A handoff is a record, not proof of what's merged or deployed. Use Git history and CI to check what landed, and the live build identity to check what players are running.
4. **Deliver one focused PR.** Branch from current `main` unless the work order names another base. Put the work-order ID in the branch and PR title, run the relevant checks and record the evidence, and leave the PR in draft if required validation is outstanding.

## Find your workstream

| Workstream | Start with | Code entry point |
| --- | --- | --- |
| Animation and rig | [Animation Codex](docs/animation-codex.md), [animation work orders](docs/handoff/work-orders-ai-animation.md), [current handoff](HANDOFF.md) | [Animator](crates/scalia-wasm/src/anim.rs), [Rust pose rig](crates/scalia-render/src/pose.rs), [WGSL mirror](crates/scalia-render/src/wgsl/common.wgsl) |
| Tactical AI and command | [Titan behaviour roadmap](docs/titan/AI_BEHAVIOUR_ROADMAP.md), [Titan system](docs/titan/TITAN_SYSTEM.md), [AI and animation handoff](docs/handoff/ai-animation.md) | [scalia-ai](crates/scalia-ai), [scalia-core](crates/scalia-core) |
| Pack Engine and performance | [Pack Engine roadmap](docs/pack-engine-roadmap.md), [performance work orders](docs/work-orders/performance-workorders.md) | [scalia-core](crates/scalia-core), [scalia-render](crates/scalia-render), [scalia-bench](crates/scalia-bench) |
| Auralis audio | [Auralis architecture](docs/auralis-audio-engine.md), [audio work orders](docs/work-orders/auralis-audio-workorders.md) | [scalia-audio](crates/scalia-audio), [browser audio graph](web/audio.js) |
| Browser, UI, world and content | [Cross-system work orders](docs/handoff/work-order-v2.md), [milestone status](STATUS.md) | [web](web), [WASM bridge](crates/scalia-wasm), [world catalogues](crates/scalia-world), [blueprints](crates/scalia-blueprint) |

Use the work order for scope and acceptance, the subsystem references for design and code orientation, and GitHub for current implementation status. [Milestone status](STATUS.md) and dated handoffs preserve historical evidence; they are not live release dashboards.

## Engineering standards

- **Determinism:** Keep simulation authoritative. Animation, audio, UI, and rendering must not change battle outcomes. For changes that should be simulation-neutral, compare the same seeded battle replay before and after; explain any intentional change.
- **Compatibility:** Treat save/replay formats, authored data, and native/WASM boundaries as contracts. Document migrations and compatibility implications before changing those contracts.
- **Scale:** Measure performance changes against representative loads. Keep procedural pose work on the GPU where practical; keep Rust and WGSL rig changes in parity.
- **Evidence:** Add a regression check for changed behaviour. For browser-facing changes, include real browser evidence and report the environment. A green build is not visual or listening sign-off.

See [the full engineering and verification rules](CONTRIBUTING.md#engineering-standards) for what to record in the PR.

## Versioning and traceability

| Identifier | What it answers | Where to find it |
| --- | --- | --- |
| Work-order ID and revision | What are we doing, and which scope was approved? | Work order or handoff; keep the ID in the branch and PR title. |
| PR number and commit SHA | Which change was reviewed, tested, or merged? | GitHub PR, Git history, and CI run. Record the SHA when reporting test results. |
| Game build version and update label | What is actually deployed? | [`VERSION`](VERSION) declares the release series. The Pages build derives the patch number, update label, PR reference where available, and deployed SHA; see [live build identity](https://hound-protocol.github.io/Scalia/version.json). |

For example, a PR titled `AN-02-S03: Keep prone aim steady through crawl transitions` identifies the work order. A matching `Update: Animation` line in its description and final commit identifies the changelog area. Those fields are separate. See [release identity](CONTRIBUTING.md#release-identity) for the full rules.

## Verify locally

Use the Rust toolchain and WASM target declared in [rust-toolchain.toml](rust-toolchain.toml), Node.js 22, and `wasm-bindgen-cli` 0.2.130. Bring Python 3 for release-metadata tests.

```bash
bash scripts/check.sh
```

The wrapper runs [the canonical verification suite](scripts/verify.sh) and writes its full log to `/tmp/verify.log`. For CI or deployment issues, start with [Verify](.github/workflows/verify.yml) or [Pages](.github/workflows/pages.yml).
