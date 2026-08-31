# `axiolid-brep`

Owns the strict, analytic B-rep **result** contract: typed 3D-curve, 2D-pcurve,
and surface catalogs plus an owned `axiolid-topology` graph.

`axiolid-topology` remains generic and format-neutral. This crate binds it to
Axiolid's neutral `Curve2`, `Curve3`, and `Surface` values without a model,
compiler, mesh, or source-format dependency.

## Invariants

- An exact result has at least one face.
- Every edge has a finite, non-zero native 3D curve interval.
- Every edge use has a finite, non-zero pcurve interval.
- Every face has a support surface.
- All typed support handles resolve in their matching catalog.
- Structural topology must pass `audit_brep`; closure is required only by an
  eventual exact-solid result, not by an exact sheet.

Do not add evaluation, intersection, tessellation, or graph traversal here.
