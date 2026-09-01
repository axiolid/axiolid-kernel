# axiolid-backend-cpu instructions

Purpose: Portable/runtime-specialized CPU execution context.

Allowed internal dependencies: axiolid-kernel plus L2 algorithms. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

features.rs; config.rs; execution.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

This crate is an execution **context** (ISA detection, worker pool, policy). It
is explicitly **not** the correctness oracle: per `docs/adr/0012` the scalar
reference implementation is owned by `axiolid-scalar`, and the scalar
implementation of an operation lands before any optimized implementation of it.

Portable path is the differential oracle's target, not its owner. SIMD requires runtime detection. Optional Rayon uses a
local bounded pool. Operation providers compose this context and implement a
capability trait only when the algorithm works. Feature-gated tests must prove
default scalar selection, SIMD runtime selection, disabled-parallel rejection,
and configured local-pool worker counts. Never compile the whole workspace for
the build host only.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
