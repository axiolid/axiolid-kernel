# 0032 — Periodic B-spline surfaces use an explicit cyclic schema

- **Status:** Accepted
- **Date:** 2026-09-01
- **Deciders:** Axiolid maintainers

## Context

A neutral `BSplineSurface` closure flag does not define cyclic knots, control aliases, parameter wrapping, or seam-preserving edits. Those semantics must be opt-in.

## Decision

`axiolid-nurbs` owns `PeriodicBSplineSurface`, a validated owned wrapper. Neutral evaluators and editors remain unchanged. At least one axis must be periodic.

For periodic degree `p`, stored control count `N`, and unique count `n=N-p`: `p>=1`, `n>p`, and the expanded knot vector has `N+p+1` finite entries.

The active quotient domain is `[U_p,U_N)`, with positive finite period `T=U_N-U_p`. Exact cyclic knots satisfy `U_(n+i)-U_i=T` for `i=0..2p`.

Periodic multiplicities are `1..=p`. The last `p` control rows/columns and rational weights exactly equal the first `p`; all controls are finite and all weights positive.

Seam continuity is algebraic: seam multiplicity `m` exposes order `C^(p-m)`. Finite periodic parameters wrap to the half-open domain when their binary64 spacing resolves one period; under-resolved huge offsets fail closed. Nonperiodic axes keep neutral clamping and do not require exact period subtraction.

Control and weight edits address the unique net. Signed wrapped indices cross periodic seams, and every duplicated U/V/corner alias updates atomically. Invalid edits fail before mutation.

The topology is fixed: periodic knot insertion, splitting, degree change, and arbitrary conversion from geometrically closed surfaces are not claimed by this API.

Certified projection covers one complete period per periodic axis. Conversion adds `p` cyclic supports on both sides so seam boundaries become interior before outward interval knot refinement.

The existing deterministic work/depth cap covers cyclic extension, refinement, patch construction, subdivision, and retained minimizer boxes. Exhaustion or unsafe arithmetic fails closed.

## Consequences

Periodic U, V, and UV surfaces now have explicit evaluator, editor, and global projection semantics without reinterpreting neutral geometry.

Callers must provide canonical duplicated controls and knot extensions. Exact equality is intentional: tolerant seam recognition belongs to a future explicit conversion operation.

## Verification

Tests cover invalid cyclic knots/aliases, U/V wrapping, huge-offset refusal, inexact-width neutral axes, C-order reporting, rational corner edits, neutral compatibility, quadratic seam refinement, boundaries, seams, local minima, and dense references.

## Relation to code

- `crates/axiolid-nurbs/src/periodic_surface.rs`
- `crates/axiolid-nurbs/src/certified_surface_bezier.rs`
- `crates/axiolid-nurbs/src/certified_surface_projection.rs`
