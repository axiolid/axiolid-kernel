# 0011 — Native accelerator backends arrive out of tree

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** Friedrich, axiolid
- **Supersedes:** —

## Context

`axiolid-backend-gpu` is an API-neutral adapter contract, not a GPU implementation.
No GPU work exists yet; `docs/ROADMAP.md` Stage 6 defers a concrete executor to
measured, batch-friendly workloads, and explicitly notes mesh boolean is a poor
GPU fit (branchy, topological, precision-sensitive, per-element work too small
to amortize a PCIe transfer).

Two questions were open:

1. Which GPU path do we take first?
2. If a native CUDA/HIP backend is wanted later, does today's structure make
   that a small addition or a rewrite?

Forces:

- **No C++ in the dependency graph** is the project premise (ADR 0003, ADR 0001,
  `docs/ROADMAP.md` non-goals). Requiring a C++ toolchain to `cargo build` is
  what we are differentiating against.
- Accelerator devices are **enumerated at runtime**. A driver reports `cuda:0`,
  `hip:1`, adapter names, ordinals. None of that is known at compile time.
- Rust panics unwinding into foreign frames are **undefined behaviour**.
- The representation crates are currently FFI-transferable — flat `Vec` arenas,
  owned plain data, no trait objects, no closures, no borrowed references — but
  only by good taste, with nothing enforcing it.

## Decision

We will make **pure-Rust GPU the first and default path**, and treat a native
CUDA/HIP backend as a supported *later* addition that arrives as an
**out-of-tree crate** implementing the existing `GpuGraphExecutor` seam. No
native code, no C ABI, and no build script enter this workspace to enable it.

To keep that addition cheap rather than a rewrite, four properties are now
enforced executably rather than assumed:

1. **Runtime device identity.** `BackendId` stores fixed-capacity inline UTF-8
   (`BackendId::CAPACITY`) instead of `&'static str`, so a driver-enumerated
   name is constructible via `BackendId::try_new` without leaking. It stays
   `Copy`, so it remains embeddable in `GeomError` variants and
   `DevicePreference` without adding an allocation or a lifetime to the error
   type. Over-long identities are **rejected, never truncated** — truncation
   would alias two distinct devices and corrupt both selection and blame.
2. **Transferable data plane.** Public payload types across `axiolid-core`,
   `axiolid-mesh`, `axiolid-model`, `axiolid-curve`, `axiolid-surface`, `axiolid-profile`,
   `axiolid-primitive`, and `axiolid-topology` must remain owned plain data. Trait
   objects, callables, borrowed references, raw pointers, shared-ownership
   handles, and interior mutability are rejected by an executable gate. Error
   types are exempt: a bridge converts an error to a code plus a message at the
   boundary rather than uploading it.
3. **Unprivileged seam.** `GpuGraphExecutor` must be implementable using only
   published items — no `pub(crate)` helpers, no internal modules, no hidden
   constructors. This is proven by an integration test that links the crate
   externally and builds a simulated native executor.
4. **Unwind containment is the implementor's duty.** A native executor must
   wrap foreign calls so a panic never crosses an FFI frame, and must surface
   the failure as `GeomError::BackendContractViolation` attributed to its own
   `BackendId`.

This ADR does **not** add a flat serialized graph view, a C ABI, or `#[repr(C)]`
guarantees. Those are speculative until a real bridge exists; the gates above
only preserve the *option* of building one cheaply.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Add a C ABI + `cbindgen` layer now | Speculative. No consumer exists; it would freeze a wire format before a single measured GPU workload justifies one. |
| Vendor a C++ CUDA/HIP kernel in-tree | Breaks the project premise: every consumer would need a C++ toolchain to `cargo build`. This is exactly the IfcOpenShell/OCCT cost we exist to avoid. |
| Bind `manifold3d` or OCCT for GPU CSG | Excluded on the same no-C++ rule (ADR 0003). Kept only as a documented fallback if pure Rust proves insufficient. |
| Declare native backends unsupported forever | Overcommits. Large regular batch workloads may genuinely justify CUDA/HIP later, and the seam already permits it at near-zero present cost. |
| Leave the constraints undocumented and unenforced | The failure mode is silent: one `Box<dyn …>` field in a node payload destroys native viability, and the cost surfaces years later when the bridge is written. |
| Make `BackendId` an owned `String` | Would drop `Copy`, forcing an allocation into every `GeomError` and `DevicePreference` value on hot error paths. |
| Keep `BackendId(&'static str)` and `Box::leak` runtime names | Leaks unboundedly on device re-enumeration, and buries a memory bug inside a bridge crate we do not control. |

## Consequences

**Positive**

- A pure-Rust GPU backend (`wgpu`, or a driver-API binding such as `cudarc`,
  which binds the CUDA *driver C API*, not C++) can be added in-workspace
  without violating the premise.
- A native CUDA/HIP backend can be added out of tree, by us or a third party,
  under Rust's orphan rules, with no workspace change at all.
- Device identity, precision policy, cardinality, and failure attribution are
  already contract-checked, so a bridge inherits them rather than reinventing
  them.
- The gates fail loudly at build time if a future change forecloses the option.

**Negative / costs**

- `BackendId` is 48 bytes instead of a 16-byte fat pointer. It sits in error
  variants on cold paths only; no measurement suggests this matters, and none
  is claimed.
- `BackendId::new` is now a `const fn` that panics on over-long literals rather
  than accepting any `&'static str`. Over-long literals fail the build.
- Two additional executable gates to maintain.

**Follow-ups / risks to watch**

- If a native bridge is ever built, it will need a flat graph view (arena plus
  offsets). The gates keep this cheap but do not implement it.
- `BackendId::CAPACITY` (47 bytes) is a judgement call. If a real driver
  produces longer canonical identities, raise it deliberately with a test rather
  than truncating.
- Do not claim a GPU performance win without a measurement. ADR 0009 already
  requires benchmarking before setting dispatch thresholds.

## Relation to existing code

- `crates/contracts/common/src/capability.rs` — inline `BackendId`,
  `BackendId::try_new`, `BackendId::CAPACITY`, `BackendIdTooLong`.
- `crates/execution/gpu/src/executor.rs` — the `GpuGraphExecutor`
  seam, including the required `validate_options` policy hook.
- `crates/execution/gpu/src/adapter.rs` — device, root, and
  cardinality validation; `BackendContractViolation` attribution.
- `crates/execution/gpu/tests/out_of_tree_executor.rs` — proves
  the seam is satisfiable with published API only, using runtime device
  identities and contained unwinds.
- `crates/representations/modeling/graph/tests/native_backend_readiness.rs` — enforces
  the transferable data plane across all eight representation crates.
- `docs/adr/0009-layered-geometry-dag.md` — the layering and open-seam decision
  this refines.
- `docs/adr/0003-pure-rust-mesh-boolean.md` — the no-C++ rule this preserves.
