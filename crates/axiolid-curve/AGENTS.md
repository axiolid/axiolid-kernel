# axiolid-curve instructions

Purpose: Atomic exact curves and evaluation seams.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

linear.rs; conic.rs; spline.rs; evaluate.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Composite/trim/offset/surface relations belong in axiolid-model to avoid curve-surface cycles. Preserve knots, multiplicities, weights, and domains.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
