# axiolid-core instructions

Purpose: Dependency-root geometry values.

Allowed internal dependencies: none. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

bounds.rs; primitives.rs; scalar.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Keep tolerance explicit and validated. Keep f64 storage and format-neutral units. No algorithms, serialization, or source identifiers.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
