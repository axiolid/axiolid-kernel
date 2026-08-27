# 0018 — Curve evaluation belongs to the scalar reference

- **Status:** Accepted
- **Date:** 2026-08-27
- **Deciders:** Friedrich, axiolid
- **Relates to:** 0012 (scalar reference ownership), 0015 (adopted earcut)

## Context

`axiolid-curve` declared `Curve2`/`Curve3` with five families each — line,
circle, ellipse, polyline, B-spline — and a `CurveEvaluator<C>` trait describing
`domain`, `evaluate`, and `derivative`.

Nothing in the workspace implemented that trait. A search for
`impl .* CurveEvaluator .* for` returned zero results, as it did for
`SurfaceEvaluator`, `Sweeper`, `Tessellator`, `Diagnose`, and `Repair`. Every
declared curve family was inert data.

`axiolid-compile` needed flattened rings anyway, so it grew its own private
path in `profile.rs`:

- a `circle_segments(radius, chord_error)` closed-form count, and a
  `circle_ring` sampler built on it;
- inline evaluation of `Line` and `Polyline`;
- a hard `Unsupported { operation: CurveEvaluation }` for `Ellipse` and
  `BSpline`, with a `debug_assert!` documenting that those two were the only
  unreachable families.

Three consequences followed:

1. **Ellipses and B-splines could not be built at all.** Not approximated —
   refused. `Profile::Ellipse` was likewise unreachable, and any
   `ContourProfile` containing an elliptical or spline segment failed.
2. **The tolerance contract was decorative.** `ExecutionOptions::tolerance()`
   fed `chord_error(options)`, which reached exactly one closed-form formula
   that only models circles. Nothing measured whether the resulting mesh
   honoured the budget.
3. **The algorithm sat at L3.** Revolution, surface tessellation, and sweeps
   all need curve evaluation. Each would have re-derived it, or imported it
   from an implementation crate, inverting the layer order.

ADR 0012 already governs this case: the scalar reference owns algorithms, and
it lands *before* an optimized path. Here there was no scalar reference to land
after — the only implementation was an ad-hoc one inside a consumer.

## Decision

**Curve evaluation is a scalar-reference algorithm and lives in
`axiolid-scalar` (L2). `axiolid-compile` consumes it.**

Concretely:

- `axiolid_scalar::curve` implements `CurveEvaluator<Curve2>` and
  `CurveEvaluator<Curve3>` for `ScalarCurve`, plus free functions
  `evaluate2/3`, `derivative2/3`, `domain2/3`, and `flatten2`.
- Evaluation is **analytic per family**, never generic subdivision. Derivatives
  are closed-form, not finite differences: a difference-quotient derivative
  would make the curvature oracle self-referential.
- B-splines use **de Boor**, evaluating rational curves in homogeneous space
  and projecting at the end. The derivative uses the hodograph plus the
  quotient rule, which is the only way `C = A/w` differentiates correctly.
- Flattening is **adaptive on measured sagitta**, bounded by an explicit
  `max_depth`. A closed-form segment count is a circle-only device; measured
  deviation generalizes to every family, including rational splines whose
  curvature varies within a span.
- `axiolid-compile` deletes `circle_segments` and `circle_ring` and routes
  `segment_points`, `circle_rings`, and the new `ellipse_rings` through
  `flatten2`.

### Frames are applied as written

Imported frames may be non-orthonormal. Evaluation uses the axes as given
rather than orthonormalizing, so a caller sees the geometry its source declared.
Validation is `axiolid-heal`'s concern, not evaluation's.

### Polyline parameterization is one unit per segment

Chosen over arc-length parameterization because it is exact and stable under
zero-length segments, which imported data contains. This is a **breaking
semantic**: a caller declaring `Interval::new(0.0, 1.0)` for a closed 4-point
ring previously got all four vertices (the old code ignored the domain) and now
gets one edge.

Rather than silently returning a degenerate ring, `flatten2` **refuses** a
polyline domain narrower than one segment when the polyline has more. Silent
vertex loss produces a wrong wall; a refusal produces a fixable error.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Leave evaluation in `axiolid-compile` | Puts an L2 algorithm at L3. Revolution and surface tessellation would each re-derive it or depend upward. |
| Put it in `axiolid-curve` (L1) | L1 is representation: types and data, no solving. Evaluation is an algorithm, and mixing them removes the ability to use curve data without pulling in evaluation. |
| Keep the closed-form segment count and extend it per family | There is no closed-form count for a rational B-spline whose curvature varies inside a span. Each family would need bespoke analysis, and none would be verifiable against a stated tolerance. |
| Generic subdivision for all families | Throws away exactness where it is available. A circle's points would carry subdivision error instead of being right to machine precision. |
| Finite-difference derivatives | Makes the curvature oracle compare a difference quotient against a difference quotient. The tests would pass while the derivative was wrong. |
| Normalize the polyline domain to `(0, 1)` | Hides the segment count from the caller and makes a 3-point and a 300-point polyline indistinguishable at the API. The domain *is* the information. |

## Consequences

**Positive**

- `Profile::Ellipse`, elliptical contour segments, and B-spline contour
  segments extrude. They were refused before.
- The chord tolerance is now measurable end to end. Volume error against an
  exact cylinder falls as O(chord):

  ```text
    chord     vertices   volume error   error/chord
    1e-2            32       2.015e-2         2.015
    1e-3           128       1.261e-3         1.261
    1e-4           256       3.154e-4         3.154
    1e-5          1024       1.971e-5         1.971
    1e-6          4096       1.232e-6         1.232
  ```

- Revolution, surface evaluation, and sweeps have a supported dependency
  instead of a reason to duplicate.
- The extrusion identity `volume == area * depth` is now asserted, for
  rectangles, circles, annuli, ellipses, elliptical contours, and B-spline
  contours. It was never checked before, despite extrusion being implemented.

**Negative / costs**

- `axiolid-scalar` gains a dependency on `axiolid-curve`. Both are within their
  layers (L2 -> L1), so the DAG is unchanged.
- The polyline domain semantics are stricter, and one in-tree fixture declared
  `(0, 1)` for a 4-segment ring. It was wrong and is fixed; out-of-tree callers
  making the same mistake now get an error rather than a silently truncated
  ring. This is deliberate.
- Adaptive subdivision costs more evaluations than a precomputed count for the
  circle case specifically. Unmeasured, and irrelevant next to correctness for
  the families that previously did not work at all.

**Not decided here**

- `SurfaceEvaluator`, `Sweeper`, `Tessellator`, `Diagnose`, and `Repair` remain
  unimplemented. Revolution is the natural next consumer of this work, and can
  reuse both the flattening and the capping logic.
