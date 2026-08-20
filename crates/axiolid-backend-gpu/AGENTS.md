# axiolid-backend-gpu instructions

Purpose: API-neutral GPU executor adapter.

Allowed internal dependencies: axiolid-kernel, axiolid-model, axiolid-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

device.rs; executor.rs; adapter.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

No fake device or claimed operation. Concrete API crates implement
`GpuGraphExecutor` or another narrow executor. Batch boundaries amortize
transfer. The adapter validates device preference, f32/f64 policy, graph-root
ownership, and result cardinality, then invokes the executor's required
operation-specific option-validation hook before submission. Result residency is
validated before dispatch: a device cannot deliver into another device's memory,
and an unrecognized future residency is refused rather than assumed. The adapter overrides `compile_batch_into` (not `compile_batch`) so both batch
call shapes reach the device in one submission. Executor output
contract violations use `BackendContractViolation`; they are not caller input
errors. Concrete executors must honor forwarded determinism and memory-budget
requirements.

The seam must stay satisfiable by an out-of-tree crate using published items
only; `tests/out_of_tree_executor.rs` proves this with a simulated native
backend. A native (CUDA/HIP) implementor must contain unwinds at its FFI
boundary and report faults as `BackendContractViolation`. See `docs/adr/0011`.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
