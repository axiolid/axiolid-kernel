# axiolid-nurbs plan

Status: first general-kernel milestone implemented.

## Implemented

- Differential geometry from analytic second-order jets.
- Exact shape-preserving curve reversal, insertion, split, and Bézier decomposition.
- Exact tensor-product surface U/V insertion and reversal.
- Explicitly budgeted curve/surface inverse queries with honest local status.
- Outward-rounded globally certified clamped curve projection and curve-pair
  minimum distance, including interval-aware homogeneous knot refinement.
- Verified closed-curve seam classification and parameter wrapping.
- Optional `axiolid/nurbs` facade feature and `parametric` bundle adoption.

## Later

- Knot removal and degree operations.
- Certified intersection root isolation and globally certified surface projection.
- Surface-periodic seam wrapping.
- Fitting, interpolation, lofting, and blending operations.
- Benchmarked optimized providers after scalar differential validation.

No item is a capability claim until its public API, tests, and facade feature land.
