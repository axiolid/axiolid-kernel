# axiolid-mesh instructions

Purpose: Discrete mesh exchange representations.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

triangle.rs; polygon.rs; view.rs; error.rs; audit.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Preserve n-gons/holes until triangulation. Keep indices u32 and validate before indexing. MeshView must permit zero-copy foreign meshes. Rendering materials are not geometry. Budgeted callers use `try_audit_mesh` plus `audit_mesh_scratch_bytes`; never allocate before admitting that checked bound.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
