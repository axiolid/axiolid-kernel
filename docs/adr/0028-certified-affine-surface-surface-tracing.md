# ADR 0028: Certified affine NURBS surface/surface tracing

- Status: Accepted
- Date: 2026-08-31
- Owners: `axiolid-nurbs`

## Context

Surface/surface intersection is generically a one-dimensional manifold in four
parameters:

$$
\mathbf S_1(u,v)-\mathbf S_2(s,t)=\mathbf 0.
$$

This is not a square isolated-root problem. Applying the existing 3x3 Krawczyk
solver directly to all four parameters would leave one degree of freedom
unconstrained, while ordinary floating-point marching could skip components,
cross singularities, or invent topology at seams and patch boundaries.

A first production-integrated slice must therefore certify what it emits and
retain every unsupported candidate explicitly rather than presenting a sampled
polyline as a complete intersection curve.

## Decision

`axiolid-nurbs` owns `intersect_surface_surface_certified` with a bounded,
non-exhaustive result contract:

- `Complete`: every refined patch pair was excluded, or one supported affine
  patch pair was proved to contain the returned transverse trace;
- `Unresolved`: certified traces already established are retained while
  conservative four-parameter boxes identify unsupported or proof-insufficient
  patch pairs.

The input domain matches ADR 0027: finite clamped polynomial/rational surfaces,
strictly positive finite rational weights, and internally continuous spans with
internal multiplicity `1..=degree`. Full-multiplicity internal knots remain
valid neutral NURBS representation but are unsupported by this certified query.

General tensor rational-Bezier patch pairs receive conservative coordinate-hull
exclusion. A complete trace is currently emitted only when both inputs are
single-span, degree-1 in both axes, polynomial, and exactly affine. Affineness is
proved as a zero bilinear cross term in exact binary64 expansion arithmetic;
there is no tolerance-based planar classification.

## Affine trace certificate

For a supported pair:

1. Outward tensor rational-Bezier partial enclosures form both plane-normal
   enclosures.
2. The interval cross product of those normals must have a strictly positive
   squared-norm lower bound. This proves the planes are transverse throughout
   both parameter rectangles.
3. Each of the four exact boundary curves of each patch is queried against the
   other surface using ADR 0027's strict-interior 3x3 Krawczyk certificates.
4. Exactly two pairwise-disjoint boundary-root certificates establish the
   endpoints of the clipped affine intersection line. Convexity of affine
   parameter rectangles and transversality then imply one connected line
   segment and no additional component.
5. Endpoint certificates preserve native parameter intervals on both surfaces,
   representative points, and conservative residual bounds. The trace carries
   the positive normal-cross squared lower bound.

Zero endpoints prove an empty clipped intersection only after all eight bounded
boundary queries complete. One endpoint, more than two endpoints, overlapping
endpoint boxes, a singular normal cross, or any unresolved/capacity-limited
boundary query yields `Unresolved`.

## Resource contract

- `max_refinement_work` is nonzero and capped at 100,000 shared work units for
  both surface refinements and the initial patch-pair product.
- `max_boundary_nodes` is nonzero and capped at 100,000 for each of exactly eight
  boundary curve/surface queries; total nested node work is therefore bounded by
  `8 * max_boundary_nodes`.
- `max_depth` is nonzero and capped at 64 for every nested query.
- Patch-pair candidates, boundary controls/knots, endpoints, and trace results use
  checked lengths and fallible reservation.
- Nested boundary capacity exhaustion is an unresolved candidate, not absence.
  Malformed input, refinement overflow, and refinement-budget exhaustion return
  structured errors.

## Consequences

This adds a real certificate-bearing surface/surface intersection-curve result
for a deliberately narrow but useful affine subset, plus conservative broad
phase behavior for general NURBS. It creates stable four-parameter boxes and
trace endpoint vocabulary for later topology-aware consumers.

It does **not** implement or claim:

- certified curved NURBS pseudo-arclength continuation;
- discovery or exclusion of closed interior intersection loops;
- tangency, coincidence, or overlap classification;
- ownership/deduplication across multispan, seam, periodic, or trim boundaries;
- stitching trace fragments across patch pairs;
- pcurve construction, face splitting, trimming, or B-rep insertion;
- tolerance-only marching as a substitute for a proof.

Those capabilities require separate proof and topology contracts. Consumers must
not reinterpret `Unresolved` as absence.

## Evidence

- `crates/algorithms/parametric/nurbs/tests/certified_surface_surface_intersection.rs`
- `crates/algorithms/parametric/nurbs/src/certified_surface_surface_intersection.rs`
- ADR 0027 boundary-root certificates
