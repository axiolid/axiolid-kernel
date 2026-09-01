# axiolid-topology instructions

Purpose: Typed-handle B-rep topology.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

id.rs; entity.rs; brep.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Never use bare usize across topology kinds. Geometry support is generic G; do not depend on one curve/surface model. Keep orientation explicit.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
