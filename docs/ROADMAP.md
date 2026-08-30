# Roadmap

Axiolid is an early geometry kernel. This roadmap orders work; it is not a promise of dates or a claim that every listed capability exists today.

## Current foundation

- Format-neutral values, meshes, profiles, curves, surfaces, topology, and an immutable geometry graph.
- Feature-gated facade with a portable scalar reference path and separate CPU/GPU provider seams.
- Architecture, feature-isolation, and layering gates that keep IFC, source formats, and concrete providers out of the kernel.
- Validated scalar polynomial/rational B-spline evaluation with analytic first derivatives and bounded conforming pcurve-trimmed curved-face tessellation, including oriented/reordered bounds, holes, guarded structured-grid/Earcut seeds, elementary periodic face charts, and aggregate work budgets; this is a reader/reference capability, not NURBS authoring.

## Next: trustworthy discrete geometry

- Finish and differentially test discrete mesh operations against the scalar oracle.
- Expand fixture-based robustness coverage for triangulation, mesh booleans, bounds, and spatial queries.
- Publish measured correctness and performance evidence before enabling optimized paths by default.

## Then: parametric and compiled geometry

- Strengthen exact curve, surface, profile, and topology handling without leaking source-format semantics into the model.
- Turn geometry graphs into reproducible operation plans with explicit tolerance, memory-budget, and provenance contracts.
- Keep imported data representable even where an operation provider cannot yet execute it.

## Later: execution providers

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
