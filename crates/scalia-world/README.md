# scalia-world

World objects. One catalogue (`data/catalog.ron`) defines each kind of object: shape, material, the states it can be in (a door: open, closed, locked, breached) and where people use it. Maps (`data/*.ron`) place objects from the catalogue.

- Depends on: `scalia-core`.
- Single file: `src/lib.rs`.
- Tests: `cargo test -p scalia-world`.
