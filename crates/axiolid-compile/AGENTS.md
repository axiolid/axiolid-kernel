# axiolid-compile instructions

Purpose: the scalar reference `GeometryCompiler`. Turns `GeometryGraph` nodes
into `TriMesh`: profile flattening, triangulation, linear extrusion, transform
composition, and boolean dispatch.

## Invariants

Extrusion output must be **closed, edge-manifold, and outward-oriented**,
because that is exactly `axiolid-boolmesh`'s input precondition. Volume alone does
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
`ellipse_rings` all delegate to `axiolid_scalar::curve::flatten2` (ADR 0018),
which subdivides adaptively on measured sagitta. The old private
`circle_segments`/`circle_ring` pair is gone — do not reintroduce a
closed-form segment count, it only models circles and cannot express an
ellipse or a rational spline.

`tests/extrusion_volume.rs` pins the identity `volume == area * depth` for
every supported profile family, and asserts the chord budget actually bounds
the volume error (measured: error is O(chord), constant under 5).

## Adopted dependencies

`earcut` (ADR 0015) is named in `src/profile.rs` and nowhere else, and is not
re-exported. `axiolid_scalar::triangulate_simple` audits it differentially on
hole-free polygons (`tests/oracle.rs`) — the adopted crate is verified, not
trusted.

## Layer

L3, an implementation crate alongside `axiolid-backend-cpu` and `axiolid-boolmesh`.
It may depend on representation and contract crates; nothing in L0–L2 may
depend on it.
