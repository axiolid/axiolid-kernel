# axiolid-surface instructions

Purpose: Atomic exact surfaces and evaluation seams.

Allowed internal dependencies: axiolid-core, axiolid-curve. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

elementary.rs; spline.rs; evaluate.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Bounded/swept/offset relationships belong in axiolid-model. Preserve tensor-product knot grids and rational weights exactly.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
