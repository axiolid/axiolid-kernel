# axiolid-nurbs plan

Status: first general-kernel milestone implemented.

## Implemented

- Differential geometry from analytic second-order jets.
- Exact shape-preserving curve reversal, insertion, split, and Bézier decomposition.
- Exact tensor-product surface U/V insertion and reversal.
- Explicitly budgeted curve/surface inverse queries with honest local status.
- Outward-rounded globally certified clamped curve projection and curve-pair
  minimum distance, including interval-aware homogeneous knot refinement.
- Bounded planar clamped curve/curve isolation for exact-sign lines and points,
  and contractive transverse polynomial/rational Bézier boxes, with explicit
  native-parameter resolution, distinct zero-length point contact, localized
  structural endpoint tangency/overlap, compact parameter-only DFS work items,
  allocation-safe work ceilings, and unresolved outcomes.
- Bounded clamped 3D curve/surface isolation with continuous internal span joins
  (internal knot multiplicity `1..=degree`) for isolated transverse roots; valid
  full-multiplicity internal knots remain unsupported by this certified query.
  The path uses outward tensor rational-Bézier refinement, conservative native-span
  surface partials, strict-interior 3×3 Krawczyk proofs, explicit `t/u/v`
  resolution, retained partial certificates, compact parameter-only DFS work,
  shared hard work ceilings, fallible allocations, and unresolved outcomes.
- Bounded clamped surface/surface patch-pair exclusion plus complete transverse
  intersection segments for single-span polynomial affine patches. Affine identity
  is exact over binary64 controls, normal transversality is outward-interval proved,
  endpoints retain both native surface charts through strict curve/surface proofs,
  and all unsupported curved/ownership/degenerate cases remain unresolved.
- Verified closed-curve seam classification and parameter wrapping.
- Optional `axiolid/nurbs` facade feature and `parametric` bundle adoption.

## Later

- Knot removal and degree operations.
- Ownership-aware boundary roots, full-multiplicity internal span joins, general
  tangent/overlap classification, curved surface/surface tracing and multispan
  stitching beyond the affine reference slice, and globally certified surface projection.
- Surface-periodic seam wrapping.
- Fitting, interpolation, lofting, and blending operations.
- Benchmarked optimized providers after scalar differential validation.

No item is a capability claim until its public API, tests, and facade feature land.
