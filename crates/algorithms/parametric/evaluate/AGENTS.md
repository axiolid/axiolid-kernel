# axiolid-evaluate instructions

Purpose: the scalar evaluation oracle for parametric geometry (ADR 0012, ADR 0036).

Allowed internal dependencies: `axiolid-core`, `axiolid-contracts`,
`axiolid-curve`, `axiolid-surface`. Do not add mesh, spatial, measure, or
provider dependencies — the point of this package is that a parametric consumer
(NURBS, CAD) acquires evaluation without the `axiolid-reference` umbrella graph.

## Module ownership

`curve.rs` native-domain evaluation, derivatives, jets, adaptive flattening;
`surface.rs` evaluation, partials, normals, jets, elementary inversion;
`nurbs.rs` shared private spline-axis machinery.

## Invariants

- No feature gates. An oracle that varies by feature is not an oracle.
- No intrinsics or threading; stay obviously correct in preference to fast.
- `axiolid-reference` re-exports `curve` and `surface` unchanged. Renaming or
  reshaping a public item here silently breaks `axiolid_reference::curve::*`
  callers, so treat those paths as part of this package's public surface.
