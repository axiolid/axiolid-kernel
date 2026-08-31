# 0030 — Surface closest-point certificates require bounded global branch-and-bound

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Axiolid maintainers
- **Supersedes:** —

## Context

`project_surface` is a deterministic multistart local optimizer. It can produce a
useful candidate but cannot prove that no better point exists elsewhere on the
surface. Calling that result global would conflate local convergence with an
exhaustive proof. Tensor-product rational surfaces also amplify hostile compact
knot inputs, so certification must bound refinement, search, allocation, and
floating-point progress.

## Decision

Axiolid will expose global surface closest-point projection through the separate
`project_surface_certified` API.

- The certified slice accepts finite, open, clamped polynomial or
  positive-weight rational B-spline surfaces over their full rectangular native
  domain. Internally continuous multispan surfaces are decomposed into rational
  Bézier patches. Trims and closed/periodic axes are outside this proof slice.
- Stored binary64 values are interpreted exactly. Interval-aware homogeneous
  refinement and de Casteljau evaluation widen derived arithmetic outward.
- Each pending native parameter box receives a global distance lower bound from
  the conservative Euclidean image AABB of its rational Bézier restriction.
- Every incumbent upper bound is attained at an exact binary64 `(u, v)` pair.
  The mathematical surface point at that pair is interval-evaluated before its
  distance upper bound is accepted. The scalar point and distance are a
  deterministic representative, not the proof primitive.
- Pruning is strict: a box is removed only when its lower bound is greater than
  the attained global upper bound. Equality remains possible, preserving ties
  and continua of minimizers.
- `Complete` requires both an outward distance gap within the requested linear
  tolerance and every retained native parameter box within the requested
  parameter tolerance. The result does not claim uniqueness or an exact
  minimizer; all global minimizers remain inside the returned boxes.
- Depth exhaustion and binary64 no-progress return a sound `Unresolved`
  certificate with retained boxes and an explicit reason. One checked work
  budget covers Bézier conversion, generated search cells, root/child hull and
  representative construction, and both child patch restrictions (including
  temporary de Casteljau controls). The full child charge occurs before either
  restriction begins. Work exhaustion, counter overflow, or fallible-allocation
  failure returns an error and never a complete certificate.
- Work and depth options have hard public ceilings. Pending records contain only
  patch indices, parameter bounds, lower bounds, depth, and deterministic serial
  order; patch control nets are not cloned into the queue.

The local `project_surface` API remains available and keeps its existing
non-global semantics.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Relabel multistart/Newton output as global | Sampling and local stationarity do not exclude a better basin. |
| Return only the best representative | It hides equal, continuous, or not-yet-separated global minimizers. |
| Prune `lower >= upper` | Equality can contain a valid tied minimizer. |
| Use ordinary floating-point AABB distances | A rounded-up lower bound can unsoundly discard the true minimizer. |
| Run subdivision until convergence | It permits adversarial time/memory growth and binary64 infinite loops. |
| Infer periodic surface semantics from metadata | Closure flags do not define a periodic control-net proof or seam ownership. |

## Consequences

**Positive**

- A complete result proves a global distance enclosure over the whole supported
  native surface domain.
- Multiple and continuum minimizers are represented without a uniqueness claim.
- Invalid, unsupported, resource-hostile, and unresolved cases fail closed.
- The implementation remains pure Rust, format-neutral, deterministic, and
  auditable at the interval-operation boundary.

**Negative / costs**

- Tight parameter tolerances can retain many boxes and cost substantially more
  than local projection.
- Conservative rational image bounds can force deeper subdivision than a
  specialized convex optimizer.
- Closed/periodic surfaces, trims, discontinuous internal knots, and uniqueness
  proofs remain unsupported.

**Follow-ups / risks to watch**

- Benchmark tighter conservative image bounds before changing the proof model.
- Add trim-domain exclusion only with certified pcurve ownership and coverage.
- Define periodic surface representation and seam topology separately; do not
  broaden this API from closure metadata alone.

## Relation to existing code

- `crates/axiolid-nurbs/src/certified_surface_projection.rs`
- `crates/axiolid-nurbs/src/certified_surface_bezier.rs`
- `crates/axiolid-nurbs/src/certified_bezier.rs`
- `crates/axiolid-nurbs/src/certified_projection.rs`
- `crates/axiolid-nurbs/tests/certified_surface_projection.rs`
