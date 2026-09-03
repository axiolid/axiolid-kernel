# axiolid-oracle

Independent mapped-3D verification oracle for intersection and inversion
results (ADR 0037). Workspace-internal: it is a dev-dependency of the crates it
checks and is never published through the facade.

## What lives here

- `src/grid.rs` — deterministic closed-span sampling with an explicit density
  budget. Both endpoints are always visited, so a result claimed at a box
  corner is never missed by rounding.
- `src/contact.rs` — mapped-3D deviation between two operands over a claimed
  parameter box (curve/curve in 2D and 3D, curve/surface, surface/surface).
- `src/distance.rs` — sound refutation of a claimed global minimum distance.

## The one rule that matters

This crate must not depend on `axiolid-nurbs`. Its whole value is that it
shares no subdivision, interval, or root-isolation machinery with the
implementations it checks. Adding that dependency silently turns the oracle
into a second opinion from the same code.

Evaluation comes from `axiolid-evaluate` only.

## Falsifier, not prover

- A small `contact_witness` deviation is a witness that near-coincident
  geometry exists in the claimed box. A large one only means sampling did not
  find any.
- A `closer_point_refutation` hit is a sound disproof of the claimed minimum.
  No hit is not a proof of global minimality.

Tests assert on the reported deviation, so a failure says how far off the
result was, not merely that it disagreed.
