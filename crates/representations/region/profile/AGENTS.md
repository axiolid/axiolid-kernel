# axiolid-profile instructions

Purpose: Exact 2D sweep sections.

Allowed internal dependencies: axiolid-core, axiolid-curve. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

contour.rs; parameterized.rs; validate.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Keep circles/sections parameterized. A contour segment is bounded; never store an infinite line as a closed edge. Profile placement uses Derived, not baked tessellation.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
