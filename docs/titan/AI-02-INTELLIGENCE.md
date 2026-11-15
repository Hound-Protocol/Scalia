# Titan System — AI-02

## Titan Intelligence: shared battlefield reports

### Existing system findings
- `Battle` currently stores contacts per squad.
- A contact identifies an enemy, last-known position, observation time and confidence.
- Perception reports sightings to the squad; other squads do not consume a shared intelligence contract.
- Existing squad contact decay and tactical decisions remain in `Battle` / `leader`.

### Implemented foundation
Added `scalia_ai::intelligence`:

- `IntelKey`: team + enemy identity; teams cannot read one another's reports.
- `IntelReport`: last-known position, simulation timestamp, confidence, source and reporting squad.
- `IntelSource`: Visual, IncomingFire or Audio.
- `TitanIntelligence`: deterministic report registry, one best-known report per team/enemy pair.
- Newer reports replace older reports. At equal timestamps, only stronger confidence replaces the current report.
- Confidence can be queried with linear time decay; future-dated reports are not exposed.
- Reports can be expired using an explicit simulation-time retention cutoff.

### Safety / determinism rules
- Uses simulation timestamps only; no wall-clock dependency.
- Uses ordered storage for deterministic iteration.
- A report is a last-known observation, not a live enemy transform.
- Team identity scopes access; no cross-team leakage.
- Existing squad-local contacts remain unchanged in this work order.

### Tests
Unit tests cover report ordering, equal-time confidence, decay/future timestamps, team isolation and expiry.

### Integration boundary
This is a shared intelligence data contract, not yet wired into perception, squad planners, battle save state or replay history. AI-03/AI-04 should integrate it through explicit deterministic events and persistence rather than silently replacing the current contact vectors.
