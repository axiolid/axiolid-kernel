# Changelog

All notable changes to Axiolid are documented in this file.

## [Unreleased]

### Added
- Added analytic rational B-spline surface partials and normals, plus bounded conforming support-surface refinement for pcurve-trimmed curved B-rep faces with holes, periodic charts, guarded structured-grid/Earcut seeds, and shared seam vertices.

### Changed
- Topology audit and planar B-rep compilation now reject empty loops, invalid outer-bound cardinality, undersized bounds, and zero/non-finite-area bounds instead of silently emitting empty or filled geometry; `BRepHealth` exposes dedicated empty-loop and multiple-outer counters.
- Reject malformed compact knot encodings, non-finite controls/frames/derived evaluations, and non-positive rational weights before or during spline evaluation.
- Curve flattening and curved-face boundary/interior tessellation now preserve explicit outer/bound orientation and fail closed on non-finite error metrics or unmet tolerance, depth, segment, per-face, input, and aggregate work limits.
- Extracted the format-agnostic geometry kernel from the Nehirde workspace and renamed its public crate prefix from `geom-` to `axiolid-`.
