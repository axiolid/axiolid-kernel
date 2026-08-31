# 0025 — Certified NURBS queries use outward-rounded subdivision

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Axiolid maintainers
- **Supersedes:** —

## Context

Local Newton refinement can return an excellent NURBS candidate without proving
that a better point does not exist elsewhere. Intersections need exhaustive
parameter-domain pruning, and imported compact knots are untrusted allocation
inputs. “Certified” therefore needs an executable, bounded contract rather than
a convergence label.

## Decision

The scalar certification oracle will use outward-rounded intervals over
homogeneous rational Bézier cells.

- Finite stored `f64` values are interpreted as exact inputs. Every derived
  interval operation widens to adjacent representable values.
- Clamped polynomial and positive-weight rational B-splines are refined to
  Bézier spans by interval-aware homogeneous knot insertion. Refinement rejects
  malformed compact knots, non-positive weights, overflow, and unsupported
  discontinuities before amplification. A checked closed-form preflight charges
  initial homogeneous controls, expanded knots, every knot-insertion output,
  emitted Bézier controls, and cells against the caller's node allowance before
  control conversion or insertion begins; pair queries share this allowance
  across both curves.
- Positive rational Bézier cells use the convex-hull property for conservative
  spatial bounds. Binary homogeneous de Casteljau subdivision tightens them.
- Search is deterministic branch-and-bound with explicit node and depth budgets.
  Budget exhaustion is an error, never a partial certificate.
- Success requires `upper_distance - lower_distance <= tolerance.linear()`.
  Results retain unresolved parameter intervals or pair boxes so multiple
  minimizers are not hidden by one representative.
- Cartesian curve-pair root cardinality is checked against the node budget
  before allocating pair cells.

The first public operations are globally bounded point-to-curve projection and
global curve-pair minimum distance in 2D and 3D. They coexist with the faster,
non-global multistart projection APIs.

## Capability boundary

A curve-pair minimum-distance certificate is an intersection prerequisite. It
is **not** yet root-existence proof, exhaustive intersection enumeration,
overlap classification, or intersection-curve construction. Those claims need
interval root isolation and explicit transverse, tangent, coincident, and
degenerate outcomes.

Current interval refinement supports clamped continuous B-splines. Periodic and
unclamped evaluator/editing semantics remain separate future work. The scalar
oracle is correctness-first; optimized providers require same-contract
differential evidence.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Report multistart Newton as global | Sampling density is not an exhaustive proof. |
| Subdivide ordinary floating Bézier controls | Rounded knot insertion can invalidate hull bounds without propagated error. |
| Return one parameter only | It hides equal or unresolved minimizers. |
| Unbounded Cartesian span pairing | Hostile compact knots can amplify memory before useful work. |
| Add a large CAD dependency | It would violate the pure-Rust boundary and still require a locally auditable certificate contract. |

## Consequences

**Positive**

- Certificates carry checkable global lower/upper bounds and bounded work.
- One enclosure primitive can support projection, intersection pruning, and
  later surface patch subdivision.
- Invalid or resource-hostile inputs fail closed before large allocations.

**Negative / costs**

- The scalar AABB oracle can visit many cells at tight tolerances.
- Interval refinement and tests add code beyond the existing local optimizer.
- Positive weights and clamped continuity are current proof preconditions.

**Follow-ups / risks to watch**

- Add interval root isolation and intersection classification; do not infer an
  exact intersection from distance below tolerance.
- Generalize the enclosure to tensor-product surface patches.
- Benchmark tighter convex-hull bounds before introducing an optimized provider.
- Extend periodic and discontinuous semantics with separate proofs.

## Relation to existing code

- `crates/axiolid-nurbs/src/certified_bezier.rs`
- `crates/axiolid-nurbs/src/certified_refinement.rs`
- `crates/axiolid-nurbs/src/certified_curve_projection.rs`
- `crates/axiolid-nurbs/src/certified_curve_distance.rs`
- `crates/axiolid-nurbs/src/certified_projection.rs`
