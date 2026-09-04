# Competitive capability gaps: Axiolid vs Manifold, boolmesh, CGAL, OCCT

**Date:** 2026-09-04
**Axiolid revision audited:** `4b23bc9` (post-v0.6)
**Method:** every row below was checked against the working tree at that
revision, not inherited from the 2026-08-20 comparison document (which is
pinned at `be3ddce7f465` and is now stale in several rows).

## How this differs from the existing comparison doc

`docs/research/geometry-kernel-capability-comparison.md` remains the broader
audit. It predates v0.2-v0.6, so several of its "representation/contract only"
rows are now implemented:

| Row it marks R/C | Actual state at `4b23bc9` |
| --- | --- |
| Spatial indexing | **Implemented**: `axiolid-spatial` has a real `Bvh` with overlap, pair, nearest-candidate and ray traversal |
| Curved B-rep tessellation | **Implemented**: `compile/brep.rs` trims cylinders, cones, spheres and tori by constant-u/v |
| Revolution / sweep | **Implemented (narrow)**: exact revolution for the rectangle family, exact straight fixed-reference sweep |
| Fillet/chamfer | **Partially implemented**: constant-distance chamfer on one straight edge; fillet refuses by name |
| Boolean breadth | **Widened**: exact boolean over coaxial prisms via certified 2D overlay |

The gaps below are what remains true *after* crediting that work.

## Verified gaps

Each gap states the probe that established it.

### G1. No mass properties provider

`MassProperties` (area, signed volume, centroid, second moments) is a struct,
and `Measure<T>` is a trait. **No type implements it.**

- Probe: `grep -rn 'impl Measure' crates/` returns nothing.
- OCCT: `BRepGProp`/`GProp_GProps` is a core, heavily used facility.
- CGAL: `Polygon_mesh_processing::volume`, `area`, `centroid`.
- Manifold: exposes volume and surface area on every manifold.

This is the single most conspicuous gap: every compared system computes
volume and area, and Axiolid already has closed manifolds to compute them on.
Several Axiolid tests hand-roll divergence-theorem volume sums because no
public provider exists.

### G2. Healing is a scaffold with no algorithms

`crates/algorithms/repair/heal` has `Defect`, `Diagnosis`, `is_clean()` and
`blocks_boolean()` — vocabulary only.

- Probe: the crate's own `PLAN.md` says "architecture scaffold; algorithms
  incomplete", and no `pub fn` performs diagnosis or repair.
- OCCT: `ShapeFix`/`ShapeAnalysis` shape healing is a major subsystem.
- CGAL: PMP repair — `remove_degenerate_faces`, `stitch_borders`,
  `orient_polygon_soup`, `remove_self_intersections`.
- Manifold: merge/validation cleanup on construction.

Note the ordering constraint the existing audit already recorded: diagnosis
must precede repair, and repairs must report exactly what changed. Silent
healing inside compilation is explicitly rejected.

### G3. No mesh self-intersection detection

- Probe: the only `self_intersects` in the tree is a 2D ring check inside
  `axiolid-overlay`; there is no 3D triangle-mesh self-intersection query.
- CGAL: `PMP::does_self_intersect` / `self_intersections`.
- OCCT: `BOPAlgo_CheckerSI`.

This blocks G2 (it is the hardest defect class) and is a precondition for
trusting boolean inputs.

### G4. No mesh simplification, decimation, or remeshing

- Probe: `decimate`, `remesh` match nothing in `crates/`.
- CGAL: `Surface_mesh_simplification` (edge collapse), isotropic remeshing.
- Manifold: `Simplify`, `Refine`.
- OCCT: mesh decimation in visualization services.

A kernel producing tessellations with no way to reduce them forces every
consumer to carry its own reducer.

### G5. No convex hull provider

- Probe: `convex_hull` appears only in a test file under
  `crates/algorithms/reference/tests/`; no `fn convex_hull` outside tests.
- CGAL: `convex_hull_3` is a flagship package.
- OCCT / Manifold: hull available.

Small, well-specified, exactly certifiable with the existing predicates, and
a building block for collision and fitting work.

### G6. No Minkowski sum / general mesh offset

- Probe: `minkowski` and `offset_mesh` match nothing.
- CGAL: `minkowski_sum_3`, polygon offsets.
- Manifold: offset/refine operations.
- OCCT: `BRepOffsetAPI_MakeOffsetShape`, `MakeThickSolid`.

Note `axiolid-overlay` already does exact **2D** offset (landed in v0.5), so
the planar half is done; the solid half is missing.

### G7. Exact boolean is still narrow

v0.6 widened exact boolean to coaxial prisms via 2D overlay. Still refused:
non-coaxial operands, differing z-spans on union, and curved-surface
operands (the last is explicitly out of scope in #66 and depends on general
curved B-rep intersection).

- CGAL: `Nef_polyhedron_3` handles arbitrary polyhedra exactly.
- OCCT: `BRepAlgoAPI_*` over general B-rep.

### G8. No shelling / thickening of solids

- Probe: no `shell`/`thicken` producing a hollowed solid.
- OCCT: `MakeThickSolid` is a standard modelling operation.

### G9. Fillet exists only as a refusal

v0.6 landed constant-distance chamfer on a single straight edge. Constant-
radius fillet — which needs a cylindrical blend surface and tangent trimming
— refuses by name. Variable radius and edge networks are untouched.

## Explicitly NOT proposed

`docs/ROADMAP.md` records standing refusals; none of the following belong in
a milestone, and they are listed here so a future reader does not re-derive
them as "gaps":

- **CAD exchange (STEP/IGES).** The roadmap states the kernel is not a file
  parser; format semantics belong in adapter projects.
- **Replacing boolmesh with Manifold.** Rejected unless a same-corpus
  benchmark shows materially better correctness or throughput.
- **OCCT parity or any C++ dependency.** Pure Rust is a hard constraint.
- **GPU mesh boolean.** ADR 0002 already records the workload as unsuitable.
- **Full CGAL package reimplementation.** CGAL is an oracle, not a template.

## Proposed sequencing

The ordering is a dependency chain, not a ranking:

1. **v0.7 — Measurement and validity.** G1, G3, and the diagnosis half of
   G2. Measurement is the oracle everything else is checked against, and
   diagnosis must exist before repair.
2. **v0.8 — Mesh processing breadth.** G4, G5, the repair half of G2.
   Repair depends on v0.7's diagnosis; simplification needs measurement to
   prove it preserved volume within a stated bound.
3. **v0.9 — Solid modelling breadth.** G6, G7, G8, G9. The heaviest
   geometry, and the part that most needs the validity foundation beneath
   it.

This preserves the roadmap's stated principle: each milestone exists because
the next cannot be honestly attempted without it.
