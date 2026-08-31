# Globally certified surface projection and periodic semantics

## Goal

Add two additive NURBS capabilities without changing neutral/imported spline semantics:

1. globally certified closest-point projection for finite clamped tensor-product polynomial or strictly-positive rational B-spline surfaces;
2. verified closed-seam periodic curve evaluation and editing semantics.

## Constraints

- Pure Rust, finite/fallible allocation, bounded work, deterministic ordering.
- Existing local `project_surface` remains explicitly local.
- Existing scalar evaluators remain clamped; `closed` metadata alone never enables wrapping.
- Existing public values and generic edit APIs remain compatible.
- No uniqueness or exact-minimizer claim from finite interval subdivision.

## Projection contract

Public API:

- `CertifiedSurfaceProjectionOptions`: distance tolerance, independent native parameter tolerance, node budget, depth budget.
- `SurfaceParameterBox`: closed native U/V intervals.
- `CertifiedSurfaceProjection3::{Complete, Unresolved}`.
- `CertifiedSurfaceProjectionCertificate3`: deterministic representative scalar evaluation, outward global distance bounds, all possible minimizer boxes, visited-node count.
- `project_surface_certified(...)`.

`Complete` proves:

- `lower <= global minimum distance <= upper`;
- `upper - lower <= distance tolerance`;
- every global minimizer lies in the returned box union;
- every box meets both native parameter-width tolerances.

`Unresolved` retains conservative bounds/boxes when depth or floating-point no-progress prevents proof. Node/refinement/allocation exhaustion is `BudgetExceeded`. Invalid geometry remains `InvalidInput`.

Algorithm:

1. Validate/refine to positive-weight rational Bézier patches with existing `piecewise_bezier_patches` and `RefinementBudget`.
2. Seed an attained upper bound from scalar-oracle corners/midpoints, tie-broken lexicographically.
3. Bound each patch below by outward point-to-coordinate-box distance.
4. Best-first branch-and-bound over compact native parameter boxes; reconstruct restricted control nets only while processing.
5. Prune only when `lower > incumbent_upper`; equality retains ties.
6. Split the longer normalized parameter-width axis at a representable midpoint.
7. Complete only when distance and both parameter obligations hold for all retained cells.

## Periodic contract

Meaning selected: **verified closed-seam extension**.

- Add non-owning `PeriodicCurve2` and `PeriodicCurve3` views.
- Construction requires structurally valid scalar evaluation, `closed == true`, and verified C0 endpoint equality at caller tolerance.
- `evaluate` and `jet` wrap finite exterior parameters; an in-domain upper endpoint stays upper.
- `seam_continuity` reports C0/C1/C2 capability; only C0 is required.
- One-sided derivatives at the retained upper endpoint are documented unless continuity proves equality.
- Generic `insert_knot*` remains strict/native and rejects exterior parameters.
- Periodic-view insertion canonicalizes exterior parameters to the native interior, rejects seam-equivalent insertion, preserves the image and revalidates the seam.
- Periodic split canonicalizes the cut then deliberately returns two open curves; seam-equivalent cuts are rejected.
- Reverse preserves verified closure and continuity through existing exact reversal.
- No algebraic periodic knot/control convention is inferred from booleans.
- No surface-axis periodic API is claimed in this slice; that requires proof over entire rational boundary curves and transverse derivatives.

## TDD order

### Projection RED

1. Native-domain affine plane interior, edge, and corner global minima.
2. Positive rational patch and multispan competitor.
3. Equal/continuum minima retain possible boxes without uniqueness claims.
4. Invalid target/geometry/weights reject.
5. Node exhaustion fails closed; depth exhaustion returns unresolved.
6. Outward lower-bound property checks against dense scalar samples.

### Periodic RED

1. Exterior and large-offset rational-circle evaluation/jet wrapping.
2. Upper endpoint retained; C0-only seam accepted and derivative behavior explicit.
3. Open metadata/open seam/nonfinite parameters rejected.
4. Periodic insertion canonicalizes exterior parameter, preserves shape/positive weights/seam continuity.
5. Seam insertion rejected; periodic split returns open exact pieces; reverse retains continuity.

## Validation

- Focused new integration tests.
- Mutants: unsafe lower-bound rounding/prune equality, hidden unique tie, wrapped seam insertion, skipped seam validation.
- `cargo test -p axiolid-nurbs --tests --locked`.
- `cargo test --workspace --all-features --locked`.
- `cargo build --workspace --all-features --locked`.
- strict Clippy, field gate, canonical gate.
- exact-tree independent review before publication.

## Rollback

All APIs are additive. Revert the candidate commit; no schema or neutral evaluator behavior changes.
