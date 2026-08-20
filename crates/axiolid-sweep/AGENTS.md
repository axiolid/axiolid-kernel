# axiolid-sweep instructions

Purpose: Sweep algorithm extension points.

Allowed internal dependencies: axiolid-core, axiolid-model, axiolid-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

sweeper.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Consume exact SolidOperation values. Do not own IFC semantics or silently tessellate unsupported directrices.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
