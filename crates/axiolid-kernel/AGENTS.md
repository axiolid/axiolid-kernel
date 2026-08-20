# axiolid-kernel instructions

Purpose: Backend-neutral capability contract.

Allowed internal dependencies: axiolid-core, axiolid-model, axiolid-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

backend.rs; capability.rs; certainty.rs; execution.rs; error.rs; compile.rs; boolean.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

No concrete implementation or hardware API dependency. Open traits remain
downstream-implementable. A trait implementation is the capability proof;
descriptors carry identity only. Determinism is three
distinct contracts (topological, numerically bounded, bitwise) compared by
strength, never equality. Residency is part of the plan: an operation declares
where inputs live and where outputs are wanted. Hot-path traits declare a
`ScratchRequirement`; the registry checks it against the memory budget *before*
dispatch, and an unbounded requirement never fits a declared budget.

A topology decision may only consume a `Certified` sign, never a bare float:
`Certified::Uncertain` carries no sign, so an undecided predicate cannot be
misread as zero. `EscalationLadder` steps f32 -> f64 -> exact; `Precision::Mixed`
is a strategy, not a rung. Batch-producing operations declare an `OutputBound`
so callers can scan per-element counts into disjoint write offsets instead of
using a global atomic or a growing vector. `compile_batch_into` is the seam a
batching provider overrides; overriding only `compile_batch` leaves the `_into`
shape on the serial fallback.

Registries store executable trait objects,
never boolean capability claims. Registry batch methods dispatch to provider
batch overrides; only `Unsupported` and `Unavailable` permit fallback, while
all other errors fail fast.

`BackendId` stores fixed-capacity inline UTF-8 so a driver-enumerated
accelerator identity (`cuda:0`, `hip:1`) is constructible at runtime without
leaking, while staying `Copy` inside error variants. Over-long identities are
rejected, never truncated: truncation would alias two distinct devices. See
`docs/adr/0011`.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
