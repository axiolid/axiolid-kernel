# Axiolid restructure: current-to-target crate map

Status: implementation baseline at `372b16f64fa9be962df48751a654f8cda3f3b4a0`.

This document records repository truth before the physical move. It is not a capability-status claim.

## Baseline

- 24 workspace packages, approximately 33k Rust source lines.
- Root package graph is acyclic and currently enforced by a mutation-probed coarse tier test.
- `axiolid-kernel` mixes guarantee vocabulary, common provider/execution contracts, and three operation seams.
- `axiolid-field` mixes stored sampled-field values with sampling, morphology, clearance, and navigation algorithms.
- `axiolid-scalar` is the portable correctness/reference implementation, not a CPU runtime.
- `axiolid-generate` owns constructive modeling algorithms.
- `axiolid-boolmesh` is an adopted, replaceable mesh-Boolean provider.
- `GeometryCompiler` currently means graph-to-`TriMesh`; ADR 0020 requires tessellation to become explicit rather than the universal exact-geometry result.

## Mechanical move map

The first stage changes paths only. Package names and Rust APIs remain stable.

| Current package path | Mechanical target path | Architectural role |
|---|---|---|
| `crates/foundation/core` | `crates/foundation/core` | foundational values |
| `crates/representations/analytic/curve` | `crates/representations/analytic/curve` | analytic representation |
| `crates/representations/analytic/surface` | `crates/representations/analytic/surface` | analytic representation |
| `crates/representations/analytic/primitive` | `crates/representations/analytic/primitive` | analytic/volumetric intent |
| `crates/representations/region/profile` | `crates/representations/region/profile` | region/profile representation |
| `crates/representations/topology` | `crates/representations/topology` | neutral topology representation |
| `crates/representations/brep` | `crates/representations/brep` | geometry-supported exact B-rep |
| `crates/representations/discrete/mesh` | `crates/representations/discrete/mesh` | discrete representation |
| `crates/representations/modeling/graph` | `crates/representations/modeling/graph` | authored modeling DAG |
| `crates/contracts/operations/tessellate` | `crates/contracts/operations/tessellate` | existing explicit tessellation seam |
| `crates/contracts/common` | `crates/contracts/common` | temporary mixed contract package |
| `crates/algorithms/reference` | `crates/algorithms/reference` | portable reference/oracle algorithms |
| `crates/algorithms/parametric/nurbs` | `crates/algorithms/parametric/nurbs` | parametric algorithms |
| `crates/algorithms/construction/construct` | `crates/algorithms/construction/construct` | constructive modeling algorithms |
| `crates/algorithms/planar/overlay` | `crates/algorithms/planar/overlay` | planar algorithms |
| `crates/algorithms/query/spatial` | `crates/algorithms/query/spatial` | spatial query algorithms |
| `crates/algorithms/query/measure` | `crates/algorithms/query/measure` | measurement algorithms |
| `crates/algorithms/sampled/field` | `crates/algorithms/sampled/field` | temporary mixed sampled field package |
| `crates/algorithms/repair/heal` | `crates/algorithms/repair/heal` | repair algorithms |
| `crates/providers/mesh/boolmesh` | `crates/providers/mesh/boolmesh` | adopted mesh Boolean provider |
| `crates/execution/compile` | `crates/execution/compile` | graph traversal/orchestration |
| `crates/execution/cpu` | `crates/execution/cpu` | CPU execution context |
| `crates/execution/gpu` | `crates/execution/gpu` | GPU execution context/adapter |
| `crates/facade/axiolid` | `crates/facade/axiolid` | optional facade |

## Semantic target changes

After the mechanical checkpoint:

1. `axiolid-kernel` is decomposed into `axiolid-guarantees`, `axiolid-contracts`, and real operation contracts. Mesh Boolean and mesh section become `axiolid-op-mesh-boolean` and `axiolid-op-mesh-section`. The existing tessellation seam becomes `axiolid-op-tessellate` only when its complete public contract is defined.
2. `GeometryCompiler -> TriMesh` is removed as a universal compilation claim. Graph traversal/caching remains in `axiolid-compile`; graph-to-mesh becomes explicit tessellation. No universal geometry-result enum is introduced.
3. `axiolid-scalar` becomes `axiolid-reference`, preserving portable oracle semantics and mutation suites.
4. `axiolid-generate` becomes `axiolid-construct`.
5. `axiolid-field` retains data/config/evidence only at `representations/sampled/field`; sampling, morphology, clearance, and navigation move to `axiolid-field-ops` under algorithms.
6. `axiolid-boolmesh` becomes `axiolid-provider-boolmesh`.
7. The facade retains additive feature behavior and directly consumable leaf crates.

## Material conflicts and resolutions

- **Brief versus ADR 0020:** the brief agrees with ADR 0020. Exact B-rep remains the kernel model; mesh compilation is reclassified as explicit tessellation.
- **Proposed linear DAG versus real composition:** enforcement is role-based. Narrow representation-to-representation edges are declared rather than forcing a fake linear chain.
- **`axiolid-tessellate` name:** current code is already a contract crate, not an implementation. Its physical move precedes any package rename.
- **Field placement during migration:** the mixed package is temporarily classified as an algorithm. It may not claim representation-only status until field algorithms are extracted.
- **Guarantee vocabulary:** extraction is mandatory from the mixed contract package. Certified predicate implementation remains in the reference algorithm, not foundation.
- **Narrow numeric substrate:** extraction is deferred until dependency, consumer, conformance, and timing evidence passes the brief's threshold.
- **MCS/Axioval source:** no MCS/Pkl source document or generated DTO exists in this repository. The supplied restructure brief defines the boundary; no transport/schema package is introduced into Axiolid.
- **Downstream IFC:** current IFC pins pre-OpenProfile Axiolid `1db0184...`. Rewiring must update to the landed restructure commit and adopt the format-neutral OpenProfile declaration without introducing operation/provider dependencies.

## Landing order

1. Axiolid mechanical move and checker.
2. Axiolid package renames and semantic splits.
3. Axiolid immutable review and guarded publication.
4. IFC child pin/import/feature/OpenProfile adaptation.
5. IFC child review and publication.
6. Openbim superproject pin update, only after child commits are remote-verifiable.
