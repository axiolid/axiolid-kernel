# Capabilities and status

Axiolid is **striving to be a multipurpose, exact B-rep geometry kernel** —
usable for CAD construction, for rule checking over building models, and for
the analytical work those applications need. It is explicitly *not* intended to
be another tessellation pipeline. Exact geometry is the model; tessellation is
one output you can ask for, with a tolerance you choose. See
[ADR 0020](/adr/0020-exact-brep-kernel-model).

That is the **ambition**. This page separates it from the **status**, which is
deliberately conservative:

- **Implemented** — a focused crate or provider has executable behavior and tests.
- **Represented** — the neutral type vocabulary or contract exists.
- **Seam** — an extension boundary exists; not a claim that Axiolid bundles a
  production implementation.
- **Planned** — accepted as in-scope by an ADR, not yet built.

Where the two differ, the status column wins. An accepted ambition is not
evidence that an algorithm exists.

## Where the implementation stands against the ambition

The kernel today evaluates geometry exactly but **stores results as triangles**:
`axiolid-compile` memoises every graph node as a `TriMesh`, so a surface it can
represent exactly is discretised at the first graph edge. For a unit cylinder
this under-reports volume by 6.4e-03 at 32 segments and 2.5e-05 at 512, always
in the same direction, and chained operations compound it.

Closing that gap is the kernel's main line of work. Until it is closed, treat
any exactness claim as applying to *evaluation*, not to *operation results*.

## Core and reference work

| Capability | Status | Evidence / boundary |
| --- | --- | --- |
| Scalar values, frames, transforms, bounds, intervals, tolerance | Implemented | `axiolid-core` |
| Mesh values, polygon/triangle utilities and views | Implemented | `axiolid-mesh` |
| Robust-orientation and in-circle / in-sphere predicate reference paths | Implemented | `axiolid-scalar` with degeneracy and filter tests |
| Scalar mesh generation from profiles and paths | Implemented discrete reference path | `axiolid-generate`; direct extrusion, revolution, lofting, sweep families, centre-line profiles, and bounded half-space clipping. It is model-free and deliberately reports `scalar-generate`. New APIs must select exact B-rep or explicit tolerance-bearing tessellation; see [ADR 0023](/adr/0023-solid-generation-is-an-l2-crate) and [ADR 0024](/adr/0024-exact-brep-result-contracts) |
| Scalar graph compilation | Implemented reference path | `axiolid-compile`; DAG traversal, caching, model-driven directrices, and B-rep face tessellation. Consumes `axiolid-generate` rather than owning sweeps. Produces meshes today; see [ADR 0020](/adr/0020-exact-brep-kernel-model) |
| Polygon triangulation | Implemented provider | `axiolid-tessellate` adopts Earcut under the contract in [ADR 0015](/adr/0015-adopt-earcut-polygon-triangulation) |
| Mesh Boolean execution | Optional provider | `axiolid-boolmesh`; limited to its mesh contract and tests |

## Geometry representation

| Capability | Status | Notes |
| --- | --- | --- |
| Primitive solids and half-spaces | Represented | `axiolid-primitive` owns neutral values and validation |
| Profiles / contours | Represented with validation | `axiolid-profile` |
| Curves and surfaces | Represented | `axiolid-curve`, `axiolid-surface`; representation alone is not an evaluator claim |
| Polynomial/rational B-spline evaluation | Implemented scalar oracle | `axiolid-scalar`; validates compact knot/control/weight data and exposes homogeneous point, first-, and second-derivative curve/surface jets |
| Analytic curve evaluation and adaptive flattening | Implemented | `axiolid-scalar`; native-parameter evaluation with a chord-error budget, fail-closed on depth exhaustion and unbisectable intervals |
| Point→parameter inversion, elementary surfaces | Implemented | `axiolid-scalar`; plane/cylinder/cone/sphere/torus with residual validation, refusing degenerate axis/apex/pole points |
| General NURBS differential analysis | Implemented reference path | `axiolid-nurbs`; curve tangents/curvature and surface fundamental forms, normals, Gaussian/mean/principal curvature; see [ADR 0022](/adr/0022-general-nurbs-kernel-capability) |
| NURBS inverse queries | Implemented reference paths | `axiolid-nurbs`; deterministic budgeted multistart curve/surface candidates plus globally certified clamped curve projection and curve-pair minimum distance. Certified results carry outward-rounded lower/upper bounds and unresolved parameter cells; see [ADR 0025](/adr/0025-certified-nurbs-subdivision-oracle) |
| Planar clamped NURBS curve/curve roots | Implemented bounded reference slice | `axiolid-nurbs`; exact-sign single-span line/point predicates with a distinct zero-length `PointContact`, contractive transverse polynomial/rational Bézier boxes via strict-interior Krawczyk proof, explicit native-parameter resolution, localized structural overlap/endpoint tangency, compact parameter-only DFS work items, hard work ceilings, and explicit unresolved singular/boundary boxes. General tangency, seam ownership/deduplication, and higher-dimensional intersections remain unimplemented; see [ADR 0026](/adr/0026-certified-planar-nurbs-root-isolation) |
| NURBS exact transformations | Implemented | `axiolid-nurbs`; homogeneous curve knot insertion/reversal/split/Bézier decomposition and surface U/V insertion/reversal preserve the represented shape |
| Closed-curve seam semantics | Implemented | `axiolid-nurbs`; native-parameter continuity through second derivative and wrapping only after declared closure plus verified position continuity |
| Trimmed curved-face tessellation | Implemented bounded reference path | `axiolid-compile`; endpoint-inclusive pcurve boundaries and holes, guarded structured-grid/Earcut seeds, topological seam reuse, explicit outer/bound orientation, face-level conforming support-surface refinement, periodic face charts, and fail-closed input/per-edge/per-face/aggregate/depth limits; see [ADR 0019](/adr/0019-validate-and-refine-nurbs-on-the-scalar-read-path) |
| Topology / B-rep vocabulary | Represented | `axiolid-topology` provides generic typed-role topology; `axiolid-brep` provides strict owned analytic catalogs and required native trim spans |
| Exact B-rep result contract | Implemented contract, no exact constructors yet | `axiolid-brep::ExactBRep` refuses missing supports/spans and generic topology failures; see [ADR 0024](/adr/0024-exact-brep-result-contracts) |
| Exact B-rep operation results | Planned | The central implementation gap. `GenerationRequest` now distinguishes exact B-rep from explicit tolerance-bearing tessellation; `axiolid-compile` still returns meshes today |
| Surface/surface intersection | Planned | In scope per [ADR 0020](/adr/0020-exact-brep-kernel-model). A bounded planar curve/curve root slice now exists, but curve/surface and surface/surface solving, tracing, pcurve construction, and B-rep splitting do not |
| Immutable shared geometry DAG | Implemented structural model | `axiolid-model` uses typed IDs and backward references |
| Sweeps / extrusions / revolutions / lofts | Implemented discrete reference path; exact-result contract ready | `axiolid-generate` does not yet construct exact B-rep sweeps. Its L2 result boundary now requires an explicit exact B-rep or tolerance-bearing tessellation request; exact construction remains L2 and must not return mesh fallbacks |
| Spatial, measures, healing | Focused crates / staged capability | Consult each crate’s `PLAN.md`; do not infer broad CAD coverage |

## Execution and acceleration

| Capability | Status | Boundary |
| --- | --- | --- |
| Portable CPU context | Implemented shell | `axiolid-backend-cpu`; portable defaults and explicit feature tiers |
| Parallel / SIMD | Opt-in context features | They require measurement and differential validation before performance claims |
| GPU graph execution | Contract seam | `axiolid-backend-gpu` provides an API-neutral seam, not a bundled GPU algorithm suite |
| Native CUDA/HIP | Planned out-of-tree providers | See [ADR 0011](/adr/0011-native-accelerator-backends-out-of-tree) |

Performance work is deliberately deferred; see
[ADR 0013](/adr/0013-deferred-performance-techniques) and the
[roadmap](./ROADMAP.md). Correctness and exactness come first, and the
architecture is kept clean so optimization stays possible later.

## In scope, not yet built

Accepted by [ADR 0020](/adr/0020-exact-brep-kernel-model) as work the kernel
intends to do. These exact/certified forms do not exist today:

- Exact B-rep results carried through operations, rather than meshes between
  graph nodes.
- Curve/surface and surface/surface intersection, plus general tangent/overlap
  and seam-owned completion of the bounded planar curve/curve slice.
- Globally certified surface projection and closest-point inversion; certified
  clamped curve projection and curve-pair minimum distance now exist, while the
  surface API remains bounded multistart.
- Exact booleans, section curves, offsets, and fillets, which all sit
  downstream of intersection.

## Explicit non-goals today

- Source-format parsing or semantic interpretation.
- A claim of OpenCascade compatibility or replacement coverage.
- Bundled production CUDA, HIP, Metal, Vulkan, or WebGPU compute kernels.
- A global hidden tolerance policy.


## How to evaluate a claim

Use the evidence nearest the implementation:

1. The relevant crate’s public API and tests.
2. The architecture decision that defines the contract.
3. Feature-isolation and layering gates.
4. Benchmark reports for performance statements.

The [research comparison](./research/geometry-kernel-capability-comparison.md) is useful context, but it is not a capability declaration.
