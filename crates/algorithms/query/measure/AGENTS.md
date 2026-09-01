# axiolid-measure instructions

Purpose: Metric and mass-property contracts.

Allowed internal dependencies: axiolid-core, axiolid-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

properties.rs; measure.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Reject undefined volume for open/non-manifold input. Carry signed and absolute values deliberately; do not return plausible zeros for unsupported geometry.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
