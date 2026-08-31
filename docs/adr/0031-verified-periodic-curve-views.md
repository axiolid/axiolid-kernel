# 0031 — Periodic curve behavior is an opt-in verified view

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Axiolid maintainers
- **Supersedes:** —

## Context

A geometrically closed clamped B-spline curve is not automatically an
algebraically periodic B-spline representation. A `closed` flag alone neither
proves endpoint continuity nor defines a periodic knot/control-net topology.
Silently wrapping the neutral evaluator or preserving `closed` through every
edit would therefore invent semantics not carried by the format-neutral model.

## Decision

Axiolid will provide periodic curve evaluation and parameterized editing only
through borrowed `PeriodicCurve2` and `PeriodicCurve3` views.

- Construction requires declared closure plus independently evaluated endpoint
  position continuity under an explicit tolerance. The view records the highest
  verified native seam class: position, first derivative, or second derivative.
- Only the view wraps finite parameters. Ordinary curve evaluation and editing
  retain their existing native-domain behavior.
- Parameters outside the active domain wrap into its half-open period. An
  already in-domain upper endpoint remains the upper endpoint so its one-sided
  neutral jet semantics are preserved.
- Point and second-order jet evaluation use the canonical native parameter after
  wrapping. Position-only seams do not claim derivative continuity.
- Knot insertion canonicalizes periodic-equivalent exterior parameters, rejects
  parameters equivalent to either seam endpoint, delegates to the existing
  shape-preserving clamped insertion, and revalidates the edited seam. It returns
  a neutral curve, not a new periodic control-net representation.
- Splitting canonicalizes an interior parameter and returns two explicitly open
  neutral curves. Cutting the cycle does not preserve `closed` metadata.
- Non-finite parameters, non-finite/degenerate periods, unverified seams, and
  seam-equivalent edits fail explicitly.

No periodic surface evaluator, periodic B-spline control-net schema,
seam-preserving topology edit, or automatic conversion from geometric closure
is implied by this decision.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Wrap every curve marked `closed` | Metadata does not prove a continuous seam. |
| Change the neutral scalar evaluator | It would silently alter established native endpoint behavior. |
| Return a periodic view from every edit | Clamped insertion/split does not establish algebraically periodic control-net topology. |
| Map the in-domain upper endpoint to the lower endpoint | It erases the neutral evaluator's distinct one-sided endpoint jet. |
| Treat position closure as C1/C2 periodicity | Derivative continuity must be evaluated and reported separately. |

## Consequences

**Positive**

- Periodic behavior is explicit, validated, and additive to the neutral model.
- Existing evaluators and editors remain backward compatible.
- Callers can inspect the actual verified seam continuity before relying on
  derivative behavior.
- Wrapped edits have deterministic parameter semantics without false topology
  preservation claims.

**Negative / costs**

- Callers must construct and retain a verified view before wrapped evaluation.
- Editing returns neutral values and may require constructing a fresh verified
  view afterward.
- Geometric closure is not converted into an algebraically periodic NURBS form.

**Follow-ups / risks to watch**

- A genuine periodic control-net representation needs its own schema, knot
  invariants, serialization contract, and shape-preserving edit proofs.
- Periodic surface evaluation and seam topology require a separate decision.
- Any higher-order seam guarantees must remain tied to explicit tolerance and
  native parameterization.

## Relation to existing code

- `crates/axiolid-nurbs/src/periodic.rs`
- `crates/axiolid-nurbs/src/transform.rs`
- `crates/axiolid-nurbs/tests/periodic_view.rs`
- `crates/axiolid-scalar/src/curve.rs`
