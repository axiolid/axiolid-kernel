# axiolid instructions

Purpose: Feature-gated end-user facade.

Allowed internal dependencies: optional dependencies on all geometry crates. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

lib.rs and Cargo feature table. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Keep default small and portable. Features are additive capabilities; bundles never mention IFC or vendor names. Leaf crates remain directly usable.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
