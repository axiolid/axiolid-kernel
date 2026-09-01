# Roadmap

Axiolid is an early geometry kernel **striving to become a multipurpose, exact
B-rep kernel** — for CAD construction, for rule checking over building models,
and for the analysis those applications need. This roadmap orders work; it is
not a promise of dates or a claim that every listed capability exists today.
For what does exist, see [capabilities](./capabilities.md).

The ordering principle: **build the meat of the kernel first, shape it for
performance later.** Correctness and exact geometry come before speed. We keep
following good practice — layering, feature isolation, provider seams, measured
oracles — so that optimization stays possible when its turn comes, but we do not
trade capability for benchmarks now.

## Current foundation

- Format-neutral values, meshes, area profiles, authored bounded-open profile declarations, curves, surfaces, topology, and an immutable geometry graph.
- Feature-gated facade with a portable scalar reference path and separate CPU/GPU provider seams.
- Architecture, feature-isolation, and layering gates that keep IFC, source formats, and concrete providers out of the kernel.
- A first-class general NURBS algorithm crate with analytic second-order curve/surface jets, differential geometry, bounded local projection, globally certified clamped curve projection and curve-pair minimum distance, verified closed-curve wrapping, and exact shape-preserving insertion/reversal/split/Bézier operations; import and tessellation remain consumers.
- Bounded conforming pcurve-trimmed curved-face tessellation, including oriented/reordered bounds, holes, guarded structured-grid/Earcut seeds, elementary periodic face charts, and aggregate work budgets.
- Adaptive analytic curve flattening and point→parameter inversion for the elementary analytic surfaces, both fail-closed on degeneracy.

## Now: exact geometry through operations

The kernel evaluates exactly but stores every graph node as triangles, so
exactness is lost at the first edge ([ADR 0020](./adr/0020-exact-brep-kernel-model.md)).
Closing that is the main line of work.

- [x] Define the strict exact B-rep result contract: typed analytic support catalogs, generic topology roles, and required native edge/pcurve spans in `axiolid-brep` (ADR 0024).
- Migrate the compiler away from a mesh-only cache, so a cylinder survives an operation as a cylinder.
- Implement exact construction families in `axiolid-generate` only where all supports and trims can be populated without approximation.
- Make tessellation an explicit, tolerance-carrying output rather than the currency between nodes.
- Keep operations fail-closed: refuse what cannot be done exactly instead of silently substituting an approximation.

## Next: intersection and inversion

The prerequisites for exact booleans, analytic section curves, offsets, and fillets.
Certified clamped NURBS curve projection and global curve-pair minimum distance now
provide exhaustive outward-rounded subdivision bounds. A bounded planar curve/curve
slice certifies exact-sign lines and contractive transverse polynomial/rational
Bézier boxes while returning explicit unresolved singular or boundary boxes.
General root ownership/classification and surface certification remain open.

- Complete curve/curve boundary ownership, deduplication, and general tangent/overlap classification.
- Curve/surface and surface/surface intersection, including intersection-curve construction.
- Globally certified surface projection and closest-point inversion.
- Independent oracles for each, in mapped 3D rather than parameter space alone.

## Then: trustworthy discrete geometry

The mesh path remains supported for callers who explicitly want discrete
results, and as a differential oracle for the exact path.

- Finish and differentially test discrete mesh operations against the scalar oracle.
- [x] Add bounded mesh-derived plane-section contours with exact binary64 side classification and explicit approximation provenance (ADR 0033).
- Expand fixture-based robustness coverage for triangulation, mesh booleans, bounds, and spatial queries.

## Then: parametric and compiled geometry

- Strengthen exact curve, surface, profile, and topology handling without leaking source-format semantics into the model.
- Extend the general NURBS kernel with certified projection/intersection methods, knot removal, degree operations, fitting, lofting, and surface-periodic seam semantics only as independently tested slices.
- Turn geometry graphs into reproducible operation plans with explicit tolerance, memory-budget, and provenance contracts.
- Keep imported data representable even where an operation provider cannot yet execute it.

## Parked: performance and execution providers

Deliberately deferred until the kernel's capability surface is real. See
[ADR 0013](./adr/0013-deferred-performance-techniques.md). Benchmarks exist for
`axiolid-scalar` and `axiolid-boolmesh` only; there are no broad performance
claims to defend, and none should be made without same-harness evidence.

- Publish measured correctness and performance evidence before enabling optimized paths by default.
- Add measured runtime-selected CPU specialization without `target-cpu=native` assumptions.
- Mature the pure-Rust GPU graph executor behind capability reporting and scalar differential checks.
- Keep future native CUDA/HIP integrations out of tree and behind the provider contract.

## Release discipline

Axiolid is currently version `0.1.0`. Releases will use semantic versioning: patch for compatible fixes, minor for compatible capability additions during `0.x`, and a documented breaking-change policy before `1.0`.

The [changelog](./CHANGELOG.md) records user-visible changes. Automated versioning and release PRs are not configured yet; until they are, releases are deliberate, reviewed changes rather than automatic tags.

## Explicit non-goals

- Becoming an IFC, STEP, or CAD file parser.
- Pulling C++/OpenCascade into the dependency graph.
- Treating a type, feature flag, or provider seam as evidence that an algorithm is production-ready.
