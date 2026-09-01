# axiolid-spatial instructions

Purpose: Spatial acceleration contracts.

Allowed internal dependencies: axiolid-core, axiolid-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

index.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Queries use callbacks to avoid mandatory allocation. Broad phase returns candidates only; do not label AABB overlap an exact clash. Output order must be deterministic when requested.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
