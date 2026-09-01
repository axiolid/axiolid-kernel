# ADR 0027: Certified clamped NURBS curve/surface root isolation

- Status: Accepted
- Date: 2026-08-31
- Owners: `axiolid-nurbs`

## Context

ADR 0026 established a bounded planar curve/curve certificate path, but exact B-rep construction also needs roots of

```text
C(t) - S(u, v) = 0
```

in three spatial coordinates. A tolerance-only Newton solve is not enough: it can miss roots, merge nearby roots, accept a singular contact, or silently stop at a work limit. Surface/surface tracing and topology must not be built on such an ambiguous result.

The neutral representations already preserve clamped polynomial/rational B-spline controls, compact knots, multiplicities, and positive weights. The scalar oracle already provides native-parameter curve and surface jets. The remaining requirement is a reusable L2 proof path with explicit completion and failure semantics.

## Decision

`axiolid-nurbs` owns `intersect_curve_surface_certified` for finite, clamped 3D NURBS curves and tensor-product NURBS surfaces with continuous internal span joins (internal knot multiplicity `1..=degree`) and finite, strictly positive rational weights. Full-multiplicity internal knots remain valid neutral NURBS representation but are outside this certified query's accepted domain.

The public result is either:

- `Complete`: every candidate box was excluded or proved to contain exactly one transverse root;
- `Unresolved`: proved transverse roots are retained, while conservative parameter boxes identify singular, tangential, boundary-owned, capacity-limited, or otherwise proof-insufficient candidates.

A transverse certificate contains native `t`, `u`, and `v` intervals no wider than the requested parameter tolerance, a representative point, a conservative residual upper bound over the certified box, and a positive interval lower bound for the absolute Jacobian determinant.

Public result and certificate types are non-exhaustive. Additive classifications can therefore be introduced without forcing downstream exhaustive matches.

## Proof construction

1. Validate compact knot/control/weight structure and continuous internal span joins before amplification. Periodic/unclamped forms and full-multiplicity internal knots are rejected by this certified-query contract rather than reinterpreted; the latter remain valid neutral NURBS representation.
2. Refine the curve and both surface axes into outward-rounded homogeneous rational Bézier cells/patches. Surface refinement stays in interval homogeneous coordinates; it does not round-trip through scalar Euclidean controls.
3. Reject a curve-cell/patch pair when any Euclidean coordinate hull is disjoint.
4. Reconstruct only the current restricted curve cell and surface patch from immutable base cells plus a native-parameter box.
5. Enclose curve derivatives and rational surface partial derivatives with native-span scaling and the quotient rule.
6. Form the interval Jacobian columns `[C'(t), -S_u(u,v), -S_v(u,v)]` and require its interval determinant to exclude zero.
7. Use a finite scalar midpoint Jacobian only as an arbitrary Krawczyk preconditioner. The midpoint residual itself is evaluated by outward homogeneous de Casteljau arithmetic. Therefore scalar preconditioner roundoff can weaken or lose a proof but cannot create one.
8. Accept existence and uniqueness only when the full three-dimensional Krawczyk image lies strictly inside the current parameter box.
9. Narrow a proved root through a proof-derived contraction, then reconstruct and reprove it. No stale proof is carried across contraction.
10. Emit a certificate only after all three native parameter intervals satisfy the requested resolution.

Strict interior deliberately excludes roots on a generated subdivision boundary. Such roots remain `Unresolved` until an ownership-aware boundary policy exists.

## Resource and termination contract

- `max_nodes` is nonzero and capped at 100,000.
- `max_depth` is nonzero and capped at 64.
- Curve refinement, tensor surface refinement, initial curve-cell/patch products, contractions, and generated subdivisions share one checked work budget.
- Tensor refinement preflights checked `u128` work before control-net conversion or patch emission.
- Deferred DFS entries contain only base indices, three parameter intervals, and depth. They retain no cloned Bézier controls.
- The depth-first stack is pre-reserved to the bounded depth policy.
- Control, knot, result, midpoint, split, restriction, and patch allocations use checked lengths and fallible reservation; allocation failure returns `GeomError::BudgetExceeded`.
- Overflow, non-finite interval arithmetic, non-progressing splits, work exhaustion, and depth exhaustion fail closed.

## Consequences

This closes a bounded, reusable reference slice for isolated transverse curve/surface roots over clamped polynomial and rational NURBS with continuous internal span joins, including multispan surfaces. It supplies certificate-bearing input for later topology work.

It does **not** implement or claim:

- general tangency, coincidence, or overlap classification;
- ownership/deduplication for patch, knot, periodic, or trim boundaries;
- periodic curve/surface solving;
- full-multiplicity internal knots or discontinuous span joins;
- surface/surface intersection or intersection-curve tracing;
- pcurve construction, face splitting, trimming, or watertight B-rep integration;
- globally certified surface closest-point projection.

Those capabilities require separate contracts and tests. They must consume this result explicitly and must not reinterpret `Unresolved` as absence.

## Evidence

- `crates/algorithms/parametric/nurbs/tests/certified_curve_surface_intersection.rs`
- tensor refinement/partial tests in `crates/algorithms/parametric/nurbs/src/certified_surface_bezier.rs`
- public exports from `crates/algorithms/parametric/nurbs/src/lib.rs`
