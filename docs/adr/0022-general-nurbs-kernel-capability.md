# ADR 0022: General NURBS algorithms are a kernel capability

- Status: Accepted
- Date: 2026-08-30

## Context

Axiolid already represented polynomial and rational B-spline curves/surfaces in
format-neutral L1 crates. The scalar oracle evaluated them for importer and
B-rep tessellation consumers. That proved the read path, but it left reusable
analysis, inverse queries, periodic semantics, and exact editing without a
public capability boundary.

NURBS is an important part of Axiolid, but it is not the organizing principle
of the whole kernel. Mesh, topology, fields, spatial work, compilation, and
provider seams remain peer capabilities.

## Decision

Add `axiolid-nurbs` as an L2 algorithm crate and expose it through the optional
`axiolid/nurbs` feature and the `parametric` bundle.

Ownership is divided as follows:

- `axiolid-curve` and `axiolid-surface` own format-neutral B-spline values.
- `axiolid-reference` owns the portable f64 evaluation oracle, including analytic
  first- and second-order homogeneous derivatives.
- `axiolid-nurbs` owns reusable differential analysis, bounded inverse queries,
  verified curve seam/wrap semantics, and exact shape-preserving transformations.
- importers own source schema decoding, source knot conventions, units,
  placements, and lowering into neutral values.
- tessellators and B-rep compilers consume the representations/evaluators; they
  do not own NURBS semantics.

The first public algorithm set is:

- 2D/3D curve tangent, signed/unsigned curvature, and curvature vectors;
- surface first/second fundamental forms, normals, Gaussian/mean/principal
  curvature;
- deterministic multi-start curve/surface projection with explicit per-span,
  per-start-iteration, and aggregate-start budgets;
- closed-curve seam classification through second native-parameter derivative
  and wrapping only after verified position continuity;
- exact curve reversal, knot insertion, splitting, and Bézier decomposition;
- exact tensor-product surface U/V knot insertion and U/V reversal.

## Numerical contracts

- Rational derivatives use homogeneous quotient recurrences, not finite
  differences.
- Curve/surface regularity is checked against caller-provided tolerance.
- Projection always includes active-domain boundaries and seeds every active
  knot span, but it is a bounded local search. A returned status is not a
  certificate of the global nearest point.
- Work budgets are explicit and non-zero; aggregate start overflow fails closed.
- `closed` metadata alone never enables wrapping. Endpoint position continuity
  must be evaluated successfully within tolerance.
- Editing operations validate before expanding multiplicities and perform
  affine interpolation in homogeneous coordinates.

## Consequences

General clients can use NURBS algorithms without an IFC reader or tessellator.
The facade's small default remains unchanged; NURBS is independently opt-in and
part of the broader `parametric` bundle.

The new crate depends on `axiolid-reference` as its portable oracle. It remains an
L2-to-L2 dependency permitted by the executable layering policy and avoids a
second evaluator.

## Explicit remaining work

This decision does not claim:

- globally certified closest points;
- curve/curve, curve/surface, or surface/surface intersections;
- knot removal, degree elevation/reduction, interpolation, fitting, lofting, or
  blending;
- surface-periodic seam wrapping;
- formal global tessellation Hausdorff bounds;
- optimized performance beyond the scalar reference implementation.

Those capabilities require their own algorithms, tests, and evidence before
being added to the capability matrix.
