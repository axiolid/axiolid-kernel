# Geometry package instructions

Applies to `crates/**`. A deeper `AGENTS.md` adds crate/module
specific invariants.

## Scope

This package is a pure-Rust, IFC-agnostic geometry stack. No file-format entity,
GlobalId, IFC unit, IFC placement, representation identifier, or vendor model
type may cross this directory boundary. Source adapters lower into
`axiolid-model::GeometryGraph`; applications choose operation providers and
execution contexts.

`PLAN.md` files are deliberately not ambient context. Read a plan only when the
assigned task is implementing or reviewing roadmap work. Ordinary consumers and
maintenance agents should follow `AGENTS.md` without ingesting speculative work.

## Dependency direction

```text
L0 values
  axiolid-core
       |
L1 representations
  axiolid-mesh  axiolid-profile  axiolid-curve  axiolid-surface
  axiolid-topology  axiolid-primitive  axiolid-model
       |
L2 algorithms and contracts
  axiolid-tessellate  axiolid-spatial  axiolid-measure
  axiolid-heal   axiolid-kernel
       |
L3 execution/adapters
  axiolid-backend-cpu  axiolid-backend-gpu
       |
L4 opt-in facade
  axiolid
```

Dependencies point downward only. `axiolid-model` composes L1 values but performs
no algorithms. `axiolid-kernel` is operation traits/policy/errors only. Execution
contexts and operation adapters remain separate crates so Cargo feature
unification cannot leak an implementation into `ifc-geometry`.

## Stable boundaries

- `axiolid-core`: f64 values, transforms, bounds, explicit tolerance. No algorithms
  beyond local value operations and no serialization policy.
- `axiolid-model`: immutable append-only DAG. Every edge references a prior node,
  making CSG/mapped-item cycles impossible after construction.
- `axiolid-kernel`: narrow operation traits (`GeometryCompiler`, `MeshBoolean`),
  identity descriptors, execution policy, structured errors. Implementing an
  operation trait is the only capability claim; never duplicate it with flags.
- Backend crates: runtime hardware contexts or operation-specific adapters. They
  do not implement an operation trait until a working algorithm exists.
- `axiolid`: convenience reexports and semantic feature bundles only.

## Representation rules

- Preserve exact intent until an explicit tessellation call. Never approximate
  circles, NURBS, profiles, booleans, or placements in a source adapter.
- Keep n-gons and holes as `PolygonMesh`; emit `TriMesh` only after explicit
  triangulation.
- Keep topology separate from geometry. `BRep<G>` uses typed handles and a
  caller-chosen geometry handle.
- Reuse geometry via DAG `NodeId` and `Instance`; do not recursively clone
  mapped geometry.
- Units are already resolved when data enters this package. Generic geometry
  does not know which source unit was used.
- Every costly or tolerance-sensitive operation receives policy explicitly.
  No process-global epsilon, thread pool, backend, or model tolerance.

## Backend and hardware rules

- Every optimized operation provider requires a portable scalar oracle. Until
  that oracle works, the provider does not implement/register the trait.
- SIMD code uses target-specific modules plus runtime feature detection. Never
  set workspace-wide `target-cpu=native`.
- Parallel execution is optional and bounded. Use context-local pools; do not
  mutate Rayon's global pool from a library.
- GPU contracts expose device facts and narrow batch operations, not
  CUDA/Metal/Vulkan/WebGPU types. API-specific crates implement
  `GpuGraphExecutor` or another operation-specific executor.
- GPU f32 is not equivalent to the f64 model. Each operation provider validates
  `ExecutionOptions` and rejects work whose precision it cannot honor.
- Third-party or future AArch64/x86/GPU/accelerator providers remain possible by
  implementing open traits for downstream-owned types.
- Do not rank GPU above CPU by folklore. Selection thresholds require benchmark
  evidence for the workload and target.

## Public API conventions

- Public value types implement `Debug` and `Clone`; add `Copy`, `Eq`, `Hash`,
  `Default`, `Display`, `Error`, `IntoIterator`, or `AsRef` only when their
  semantics are honest.
- Use typed newtype IDs instead of interchangeable integers.
- Use builders for validated multi-field configuration; do not use builders for
  trivial values.
- Mark extensible public enums/errors `#[non_exhaustive]` unless exhaustiveness
  is a deliberate compatibility contract.
- Return structured errors. Unsupported capability is distinct from invalid
  input, unavailable hardware, cancellation, and numerical failure.
- Prefer borrowed views (`MeshView`) and batch APIs to forced copies and one-item
  accelerator dispatch.
- Representation, facade, contract, and GPU adapter crates forbid unsafe code.
  CPU provider crates may use localized unsafe intrinsics only with an invariant
  comment, Miri-capable scalar tests where applicable, and measured need.

## File and module growth

Split by responsibility before a Rust file reaches roughly 500 lines. A file may
exceed that only when generated or when splitting would obscure one cohesive
algorithm. Keep data, validation, algorithms, dispatch, and tests in separate
modules. Do not add empty placeholder modules: add the file when it owns a real
type, trait, invariant, test, or implementation.

## Gates

Run targeted crate tests while iterating. Before merging geometry-wide changes:

```bash
cargo test -p axiolid-core --test layering
scripts/geometry-feature-matrix.sh
cargo test -p ifc-geometry --test declaration_manifest
cargo test -p ifc-geometry --test no_backend_dependency
scripts/gate.sh
```

The feature matrix must include no-default, every facade capability alone, full,
CPU parallel/SIMD, and a non-x86 compile target. Architecture gates must be
mutation-verified before being trusted.
