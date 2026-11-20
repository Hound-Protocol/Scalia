# Docs index

One line per document, with its status. Status values: **current** (keep in step with the code), **reference** (stable background), **draft** (not approved), **historical** (a record, not live).

Start with [../README.md](../README.md), [../CONTRIBUTING.md](../CONTRIBUTING.md) and [../HANDOFF.md](../HANDOFF.md).

## Orientation

| Doc | Status | What it is |
|---|---|---|
| [architecture.md](architecture.md) | current | Crate map, data flow, determinism rules |
| [work-orders/TEMPLATE.md](work-orders/TEMPLATE.md) | current | Template for new work orders |

## Animation

| Doc | Status | What it is |
|---|---|---|
| [animation-codex.md](animation-codex.md) | reference | Canonical animation pipeline |
| [handoff/work-orders-ai-animation.md](handoff/work-orders-ai-animation.md) | current | AI and animation work orders (AN-xx, AI-xx) |
| [handoff/mixamo-prone-reference.md](handoff/mixamo-prone-reference.md) | reference | Measured prone and crawl joint ranges |
| [handoff/AN-02-S02.md](handoff/AN-02-S02.md) | historical | Prone, crawl and fast crawl (merged #63, #64) |
| [superpowers/specs/2026-11-18-remotion-m0-design.md](superpowers/specs/2026-11-18-remotion-m0-design.md) | approved, deferred | ReMotion M0: motion file format and playback |

## Tactical AI (Titan)

| Doc | Status | What it is |
|---|---|---|
| [titan/AI_BEHAVIOUR_ROADMAP.md](titan/AI_BEHAVIOUR_ROADMAP.md) | current | Behaviour roadmap |
| [titan/TITAN_SYSTEM.md](titan/TITAN_SYSTEM.md) | current | System architecture |
| [titan/AI-02-INTELLIGENCE.md](titan/AI-02-INTELLIGENCE.md) | reference | AI-02 intelligence layer |
| [titan/AI-03-TASK-LIFECYCLE.md](titan/AI-03-TASK-LIFECYCLE.md) | reference | AI-03 task lifecycle |
| [handoff/ai-animation.md](handoff/ai-animation.md) | historical | AI and animation handoff, 15 Nov |
| [handoff/combat-readiness.md](handoff/combat-readiness.md) | draft | Combat readiness design (not implemented) |

## Engine, performance, audio

| Doc | Status | What it is |
|---|---|---|
| [pack-engine-roadmap.md](pack-engine-roadmap.md) | current | Pack Engine roadmap |
| [work-orders/performance-workorders.md](work-orders/performance-workorders.md) | current | Performance audit and work orders (implementation not started) |
| [auralis-audio-engine.md](auralis-audio-engine.md) | reference | Auralis audio architecture |
| [work-orders/auralis-audio-workorders.md](work-orders/auralis-audio-workorders.md) | current | Audio work orders |

## Cross-system

| Doc | Status | What it is |
|---|---|---|
| [handoff/work-order-v2.md](handoff/work-order-v2.md) | historical | Cross-system work order from 14 Nov |
| [archive/](archive/) | historical | Retired handoffs |
| [../STATUS.md](../STATUS.md) | historical | Milestone 0 and 1 measurements |

## Conventions

- New work orders copy [work-orders/TEMPLATE.md](work-orders/TEMPLATE.md) and go in `work-orders/`. Older ones remain in `handoff/` and `titan/` to keep links stable.
- When a document stops being current, change its status here and add a one-line note at its top.
- `scripts/check.sh` fails on broken relative links in Markdown files.
