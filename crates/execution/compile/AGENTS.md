# axiolid-compile instructions

Purpose: the scalar reference `GeometryCompiler`. Turns `GeometryGraph` nodes
into `TriMesh`: profile flattening, triangulation, linear extrusion, transform
composition, and boolean dispatch.

## Invariants

Extrusion output must be **closed, edge-manifold, and outward-oriented**,
because that is exactly `axiolid-mesh-boolean-boolmesh`'s input precondition. Volume alone does
NOT verify this: a cap lying in the z = 0 plane contributes nothing to the
divergence integral, so a flipped bottom cap is invisible to a volume check.
Use the directed-edge parity gate in `tests/extrusion.rs` — every directed edge
exactly once, every edge with exactly one opposing half-edge.

Unsupported profile and solid families return `GeomError::Unsupported`, never a
silent approximation. A missing wall is cheap; a wrong wall corrupts every
downstream quantity.

No default tolerance or chord budget. The caller supplies both, because
acceptable error depends on source units and downstream use.

Curve flattening is **not owned here**. `segment_points`, `circle_rings`, and
`ellipse_rings` all delegate to `axiolid_reference::curve::flatten2` (ADR 0018),
which subdivides adaptively on measured sagitta. The old private
`circle_segments`/`circle_ring` pair is gone — do not reintroduce a
closed-form segment count, it only models circles and cannot express an
ellipse or a rational spline.

`tests/extrusion_volume.rs` pins the identity `volume == area * depth` for
every supported profile family, and asserts the chord budget actually bounds
the volume error (measured: error is O(chord), constant under 5). Volume and
area come from `axiolid-measure`, never a local divergence sum: that crate
audits closed-two-manifold first, so a hand-rolled integral would silently
measure a torn shell.

**Tolerance must scale with the chord budget.** `audit_mesh` calls a triangle
degenerate when `2A <= tolerance.linear()^2`. A cylinder flattened at chord
`c` has side quads about `sqrt(8*r*c)` wide and cap slivers far smaller, so a
fixed `Tolerance::MILLIMETRE` rejects perfectly correct geometry as soon as a
caller asks for sub-millimetre accuracy. Use `tolerance_for(chord)` in tests;
in production pass a tolerance derived from the same budget that drove
flattening. This is not a test artefact -- it is a real API contract.

## Adopted dependencies

`earcut` (ADR 0015) is named in `src/profile.rs` and nowhere else, and is not
re-exported. `axiolid_reference::triangulate_simple` audits it differentially on
hole-free polygons (`tests/oracle.rs`) — the adopted crate is verified, not
trusted.

## Layer

L3, an implementation crate alongside `axiolid-backend-cpu` and `axiolid-mesh-boolean-boolmesh`.
It may depend on representation and contract crates; nothing in L0–L2 may
depend on it.
