# ADR 0029: Certified finite traces become split faces plus embedded curves

- **Status:** Accepted
- **Date:** 2026-08-31
- **Depends on:** ADR 0024, ADR 0028

## Context

ADR 0028 provides bounded traces for transverse single-span affine NURBS
patches. Each endpoint carries native parameter boxes for both patches and a
conservative residual bound. A trace is not by itself a valid trimmed B-rep.

A straight chord partitions a rectangular face only when both endpoints are on
that face's boundary. If both endpoints are interior to another rectangular
face, placing the chord in a loop on that face would create an open or doubled
wire; it would not partition a disk. Extending it to the boundary would invent
geometry outside the certified bounded intersection.

The strict `ExactBRep` contract also requires explicit analytic 3D supports,
pcurves, and native parameter intervals. It must not infer pcurves by inversion.

## Decision

`axiolid-generate` owns the L2 topology integration function
`split_surface_pair_certified`.

The first supported arrangement requires exactly one complete affine trace:

1. Both endpoint boxes are fixed on two distinct native boundary sides of one
   patch, and their side coordinates are strictly interior to those sides.
2. Both endpoint representatives are strictly interior to the other patch's
   native rectangle.
3. The trace's conservative residual upper bound is finite and no larger than
   the caller's explicit `max_surface_residual`.

The boundary-owned patch becomes two closed trimmed faces. Its rectangle
boundary is split at the two certified endpoints. One shared intersection edge
closes both loops, once forward and once reversed, with a separate pcurve for
each traversal.

The containing patch remains one closed rectangular face. The same edge is
attached through `EmbeddedFaceCurve`, which stores the face, edge, pcurve, and
native pcurve interval explicitly. This relation records an interior arrangement
feature without pretending it is a closed trim.

The result retains the original `TransverseSurfaceSurfaceTrace3`, deterministic
face/edge identities, and a global carrier-to-surface residual bound. For affine
supports, the carrier and pcurve images are affine in the same unit parameter;
the norm of their difference over the segment is bounded by the maximum endpoint
certificate.

## Resource and failure policy

Topology size is fixed for this slice: ten vertices, eleven edges, three loops,
three faces, eleven 3D line supports, thirteen pcurves, and two surfaces.
Generic topology arenas and strict B-rep catalogs expose fallible capacity
reservation. Surface control nets, knot vectors, multiplicities, and optional
weights are copied with fallible reservation. Loop and face vectors are also
reserved fallibly.

The operation returns `Unresolved` and preserves the original intersection
evidence for:

- incomplete or multiple traces;
- mixed, same-side, corner, or dual-boundary ownership;
- boundary-touching endpoints on the embedded face;
- non-finite, degenerate, or residual-policy-exceeding representatives;
- all unsupported cases inherited from ADR 0028.

Allocation failure and an internal strict B-rep contract violation remain
structured `GeomError` failures. A disjointness certificate returns `Empty`
without constructing a B-rep.

## Consequences

- The result is a topology-aware open arrangement, not a solid or Boolean.
- The split patch has valid closed trim loops and explicit analytic supports.
- The unsplit patch retains the finite intersection as an explicit embedded
  feature available to later arrangement/Boolean algorithms.
- No endpoint box is widened, no pcurve is inferred, and no artificial seam is
  extended beyond the certified trace.
- Aligned traces owned by both patch boundaries remain unresolved until the
  intersection layer proves boundary-root ownership and deduplication.
- Curved traces, periodic seams, closed intersection loops, tangency,
  coincidence, overlap, multi-span stitching, and solid classification remain
  future work.
