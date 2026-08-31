# axiolid-nurbs plan

Status: first general-kernel milestone implemented.

## Implemented

- Differential geometry from analytic second-order jets.
- Exact shape-preserving curve reversal, insertion, split, and Bézier decomposition.
- Exact tensor-product surface U/V insertion and reversal.
- Explicitly budgeted curve/surface inverse queries with honest local status.
- Outward-rounded globally certified clamped curve projection and curve-pair
  minimum distance, including interval-aware homogeneous knot refinement.
- Bounded planar clamped curve/curve isolation for exact-sign lines and
  contractive transverse polynomial/rational Bézier boxes, with explicit
  structural tangency/overlap and unresolved outcomes.
- Verified closed-curve seam classification and parameter wrapping.
- Optional `axiolid/nurbs` facade feature and `parametric` bundle adoption.

## Later

- Knot removal and degree operations.
- Ownership-aware boundary roots, general tangent/overlap classification,
  higher-dimensional intersections, and globally certified surface projection.
- Surface-periodic seam wrapping.
- Fitting, interpolation, lofting, and blending operations.
- Benchmarked optimized providers after scalar differential validation.

No item is a capability claim until its public API, tests, and facade feature land.
