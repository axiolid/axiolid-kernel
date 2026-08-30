# Capabilities and status

This page is deliberately conservative. **Implemented** means a focused crate or provider has executable behavior and tests. **Represented** means the neutral type vocabulary or contract exists. **Seam** means an extension boundary exists; it is not a claim that Axiolid bundles a production implementation.

## Core and reference work

| Capability | Status | Evidence / boundary |
| --- | --- | --- |
| Scalar values, frames, transforms, bounds, intervals, tolerance | Implemented | `axiolid-core` |
| Mesh values, polygon/triangle utilities and views | Implemented | `axiolid-mesh` |
| Robust-orientation and in-circle / in-sphere predicate reference paths | Implemented | `axiolid-scalar` with degeneracy and filter tests |
| Scalar graph compilation | Implemented reference path | `axiolid-compile`; intended as an oracle, not a performance claim |
| Polygon triangulation | Implemented provider | `axiolid-tessellate` adopts Earcut under the contract in [ADR 0015](/adr/0015-adopt-earcut-polygon-triangulation) |
| Mesh Boolean execution | Optional provider | `axiolid-boolmesh`; limited to its mesh contract and tests |

## Geometry representation

| Capability | Status | Notes |
| --- | --- | --- |
| Primitive solids and half-spaces | Represented | `axiolid-primitive` owns neutral values and validation |
| Profiles / contours | Represented with validation | `axiolid-profile` |
| Curves and surfaces | Represented | `axiolid-curve`, `axiolid-surface`; representation alone is not an evaluator claim |
| Polynomial/rational B-spline evaluation | Implemented scalar read path | `axiolid-scalar`; validates compact knot/control/weight data, evaluates homogeneous curves/surfaces, and exposes analytic first derivatives/partials |
| Trimmed curved-face tessellation | Implemented bounded reference path | `axiolid-compile`; endpoint-inclusive pcurve boundaries and holes, guarded structured-grid/Earcut seeds, topological seam reuse, explicit outer/bound orientation, face-level conforming support-surface refinement, periodic face charts, and fail-closed input/per-edge/per-face/aggregate/depth limits; see [ADR 0019](/adr/0019-validate-and-refine-nurbs-on-the-scalar-read-path) |
| Topology / B-rep vocabulary | Represented | `axiolid-topology`; stored topology is distinct from geometry |
| Immutable shared geometry DAG | Implemented structural model | `axiolid-model` uses typed IDs and backward references |
| Sweeps, spatial, measures, healing | Focused crates / staged capability | Consult each crate’s `PLAN.md`; do not infer broad CAD coverage |

## Execution and acceleration

| Capability | Status | Boundary |
| --- | --- | --- |
| Portable CPU context | Implemented shell | `axiolid-backend-cpu`; portable defaults and explicit feature tiers |
| Parallel / SIMD | Opt-in context features | They require measurement and differential validation before performance claims |
| GPU graph execution | Contract seam | `axiolid-backend-gpu` provides an API-neutral seam, not a bundled GPU algorithm suite |
| Native CUDA/HIP | Planned out-of-tree providers | See [ADR 0011](/adr/0011-native-accelerator-backends-out-of-tree) |

## Explicit non-goals today

- A complete CAD / B-rep modeling kernel.
- General NURBS editing, knot/degree operations, intersections, projection, or closest-point inversion.
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
