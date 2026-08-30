# axiolid-nurbs plan

Status: first general-kernel milestone implemented.

## Implemented

- Differential geometry from analytic second-order jets.
- Exact shape-preserving curve reversal, insertion, split, and Bézier decomposition.
- Exact tensor-product surface U/V insertion and reversal.
- Explicitly budgeted curve/surface inverse queries with honest local status.
- Verified closed-curve seam classification and parameter wrapping.
- Optional `axiolid/nurbs` facade feature and `parametric` bundle adoption.

## Later

- Knot removal and degree operations.
- Certified closest points and intersections.
- Surface-periodic seam wrapping.
- Fitting, interpolation, lofting, and blending operations.
- Benchmarked optimized providers after scalar differential validation.

No item is a capability claim until its public API, tests, and facade feature land.
