# axiolid-tessellation-contract instructions

Purpose: Exact-to-discrete conversion contracts.

Allowed internal dependencies: all needed L1 representations. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

options.rs; tessellator.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Tolerance and chord error are explicit. Shared topological edges are discretized once and reused; independent per-face tessellation is not watertight.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
