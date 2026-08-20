# 0013 — Deferred performance techniques and their trigger conditions

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** Friedrich, axiolid
- **Supersedes:** —

## Context

A survey of GPU/SIMD geometry-kernel technique raised a large set of candidate
optimisations: SoA/AoSoA layouts, GPU shared-memory tiling, atomics versus
block-aggregated scans, prefix-sum compaction, sort/partition/scatter/gather
primitives, tensor cores, FMA, SVE/SVE2, coordinate quantisation, Morton codes,
integer/exact predicates, branchless mask discipline, and divergence-class
queues.

Nearly all of them are correct in principle. The question is not *whether* they
are good ideas but *when* adopting them is cheaper than deferring them.

The distinguishing test used here is the one that governed ADR 0011 and 0012:

> Does deferring this force a **breaking change** later?

If yes, the seam is added now. If no, the technique lives inside a backend
behind an existing trait and can arrive whenever a measurement demands it.
Adding it earlier is speculative design against an unmeasured workload.

The project's own optimisation ordering (`docs/ROADMAP.md`, Stage 4) already
says algorithmic complexity and robust numerical formulation come before data
layout, batching, threading, and SIMD. Every technique below is at the later
stages, while the numerical foundation -- predicates and their error bounds --
does not yet exist.

## Decision

**Landed now** (API shape; expensive to retrofit):

- `Precision::Exact` plus `Certified`/`Sign`/`EscalationLadder` in
  `axiolid-kernel::certainty` -- the f32 -> f64 -> exact ladder, with an explicit
  *uncertain* state so an undecided sign cannot reach a topology decision.
- `OutputBound` with `write_offsets` -- the exclusive prefix scan that turns
  per-element output counts into disjoint write offsets, removing the need for
  a global atomic counter or a growing vector on the hot path.
- `compile_batch_into` -- batch operations write into a caller-owned
  destination, so a provider can reserve once and workers can fill disjoint
  slots.

**Deferred, with the trigger that should un-defer each:**

| Technique | Why deferred | Trigger to adopt |
| --- | --- | --- |
| SoA / AoSoA compute views | `TriMesh` is a *representation*; a compute view belongs to a backend and can be added without changing it. Choosing a layout before a kernel exists is guessing. | First SIMD or GPU kernel whose profile is load-bound. |
| GPU shared/local-memory tiling | Lives entirely inside a `GpuGraphExecutor` implementation. `GpuFeatures::max_workgroup_size` and `subgroups` already expose what a tiler needs. | First GPU kernel with measured reuse across a workgroup. |
| Atomics vs. block-aggregated scan | An implementation choice inside one kernel. `OutputBound::write_offsets` already provides the aggregate-then-allocate shape. | Measured contention on a global counter. |
| Sort / partition / compact / scatter / gather | Backend-internal primitives; no public contract mentions them. | First kernel needing candidate compaction. |
| Divergence-class queues (normal / coplanar / degenerate) | Requires a predicate implementation to classify with. The `Certified::Uncertain` state is the classification signal it would key on. | Measured divergence cost in a real narrow-phase kernel. |
| Branchless / mask discipline | A coding technique inside a kernel, not an interface. | Applies from the first vector kernel onward. |
| FMA (`mul_add`) | Detected already (`CpuFeatures::avx2_fma`); needs an arithmetic kernel to use it. Fewer roundings also *changes results*, so it interacts with the `Determinism::Bitwise` contract and must be introduced deliberately. | First arithmetic kernel; must be introduced together with its differential test. |
| SVE / SVE2 | `CpuInstructionSet` is `#[non_exhaustive]`, so adding a rung is not a breaking change. No AArch64 SVE hardware is available to measure on. | Access to SVE hardware, or a user requiring it. |
| Coordinate quantisation, Morton codes, integer predicates | Real techniques, but they are *implementations* of the exact tier and of spatial indexing, both of which already have seams (`Precision::Exact`, `SpatialIndex`). | Adopting or writing the predicate implementation. |
| GPU-filtered predicates with CPU escalation | Attractive, but it makes the GPU part of the **correctness** path rather than an optional accelerator, which contradicts the GPU's current opt-in status. Needs its own decision. | A separate ADR, after CPU predicates exist and are the oracle. |

**Rejected outright:**

- **Tensor cores.** They compute `D = AB + C` on small low-precision matrices.
  The dev GPU (RTX 4000 Ada, CC 8.9) runs FP64 at 1/64 rate and its tensor
  cores are FP16/BF16/TF32 only -- there is no usable FP64 tensor path outside
  datacentre parts. A correctness-first f64 geometry kernel gains nothing. The
  plausible exceptions are all too small or too memory-bound to matter: batched
  `Transform3` application is bandwidth-limited, and the inertia tensor in
  `MassProperties` is 3x3. Revisit only if a geometry-ML feature is ever added,
  which is out of scope.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Implement everything now | Speculative design against a workload that does not exist. Most of it would be rewritten once real kernels and measurements arrive, and it would delay the predicates that everything else depends on. |
| Record nothing and revisit ad hoc | The reasoning evaporates. A later contributor re-litigates tensor cores without knowing the FP64-rate argument, or adds SoA to the representation crate because nobody wrote down that it belongs in a compute view. |
| Add empty placeholder crates/traits per technique | Scaffolding with no implementation invites false confidence, and the project forbids placeholder files. |

## Consequences

**Positive**

- The three genuinely expensive-later decisions are made while they are cheap.
- Every deferred technique has a written trigger, so adopting one is a decision
  with a stated cause rather than a preference.
- Tensor cores are closed with a hardware-specific reason, not an opinion.

**Negative / costs**

- Deferred techniques will each need their own measurement before adoption,
  which is slower than building them speculatively now.
- `OutputBound` and the `_into` seam have no batching consumer yet, so they are
  currently proven only by contract tests, not by a production kernel.

**Follow-ups / risks to watch**

- The main risk is inversion: an AVX-512 or GPU kernel landing before certified
  predicates exist, because it is more interesting to write. ADR 0012's
  ordering rule (scalar reference first) and this ADR's trigger table are the
  guard.
- `compile_batch_into` is the seam a batching provider must override.
  Overriding only `compile_batch` silently leaves `_into` on the serial
  fallback -- correct results, no batching. Pinned by
  `both_batch_call_shapes_use_a_single_submission`.

## Relation to existing code

- `crates/axiolid-kernel/src/certainty.rs` -- `Certified`, `Sign`,
  `EscalationLadder`; the f32/f64/exact ladder.
- `crates/axiolid-kernel/src/execution.rs` -- `OutputBound`,
  `ScratchRequirement`, `DataResidency`, `Determinism`.
- `crates/axiolid-kernel/src/compile.rs` -- `compile_batch_into`,
  `output_bound`.
- `crates/axiolid-backend-cpu/src/features.rs` -- `avx2_fma` detection
  already present, unused pending an arithmetic kernel.
- `crates/axiolid-spatial/src/index.rs` -- `SpatialIndex`, the seam a
  BVH/Morton implementation fills.
- `docs/adr/0012-scalar-reference-ownership.md` -- the ordering rule this ADR
  relies on.
