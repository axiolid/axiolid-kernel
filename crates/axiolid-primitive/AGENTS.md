# axiolid-primitive instructions

Purpose: Exact CSG leaf solids and half-spaces.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

solid.rs; half_space.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Values are exact intent, not meshes. Finite clipping policy is explicit. Do not hide tessellation or boolean work in constructors.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
