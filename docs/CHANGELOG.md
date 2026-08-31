# Changelog

All notable changes to Axiolid are documented in this file.

## [Unreleased]

### Added
- Adaptive analytic `Curve3` directrix sampling and validated `parameter_range` trimming for sweeps, with dimension-generic chord subdivision shared by the 2D and 3D flatteners.
- Test suites for `axiolid-curve`, `axiolid-surface`, `axiolid-primitive`, `axiolid-profile`, `axiolid-tessellate`, and `axiolid-backend-cpu`, pinning vocabulary contracts, validation refusals, and CPU feature selection.
- Added analytic rational B-spline surface partials and normals, plus bounded conforming support-surface refinement for pcurve-trimmed curved B-rep faces with holes, periodic charts, guarded structured-grid/Earcut seeds, and shared seam vertices.
- Added the format-neutral `axiolid-nurbs` algorithm crate and `axiolid/nurbs` facade feature with analytic second-order differential geometry, explicitly budgeted curve/surface projection, verified closed-curve seam wrapping, exact curve knot insertion/reversal/split/Bézier decomposition, and exact surface U/V insertion/reversal.

### Changed
- Documented the kernel's direction: Axiolid is striving to be a multipurpose **exact B-rep kernel**, with tessellation as a requested output rather than the model. Surface/surface intersection and geometric inversion are now in scope; see [ADR 0020](./adr/0020-exact-brep-kernel-model.md). Performance work is explicitly parked behind capability work on the roadmap.
- Topology audit and planar B-rep compilation now reject empty loops or outer shells, invalid outer-bound cardinality, undersized bounds, and zero/non-finite-area bounds instead of silently emitting empty or filled geometry; `BRepHealth` exposes dedicated empty-loop and multiple-outer counters.
- Reject malformed compact knot encodings, non-finite controls/frames/derived evaluations, and non-positive rational weights before or during spline evaluation.
- Curve flattening and curved-face boundary/interior tessellation now preserve explicit outer/bound orientation and fail closed on non-finite error metrics or unmet tolerance, depth, segment, per-face, input, and aggregate work limits.
- Extracted the format-agnostic geometry kernel from the Nehirde workspace and renamed its public crate prefix from `geom-` to `axiolid-`.
- Extracted scalar solid generation — profiles, lofts, sweeps, revolutions, extrusion, and bounded half-space clipping — from the L3 DAG compiler into the new L2 `axiolid-generate` crate. `axiolid-compile` now owns graph traversal, caching, model-driven directrices, and B-rep tessellation only; see [ADR 0023](./adr/0023-solid-generation-is-an-l2-crate.md).

### Removed
- Removed the `axiolid-sweep` crate and the facade's misleading `sweeps` feature and `axiolid::sweep` module. The crate held a single `Sweeper` trait with no implementors, no tests, and no references. Its former construction code is now properly extracted into `axiolid-generate`; [ADR 0021](./adr/0021-capability-seams-live-in-the-kernel.md) is superseded by [ADR 0023](./adr/0023-solid-generation-is-an-l2-crate.md).
