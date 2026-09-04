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

### G10. Production boolean constructs in f64; exactness is verification-only

Every gap above is about *which shapes* an operation accepts. This one is
about the *arithmetic*, and it cuts across all of them.

`axiolid-predicates` provides certified filtered `orient3d` (static filter,
then exact expansion, then dyadic fallback). In
`crates/providers/mesh/boolmesh/Cargo.toml` it is a **dev-dependency**: it
re-decides orientation in tests, and never runs in production dispatch.
Intersection coordinates are constructed and stored as f64. A reader who
finds certified predicates in the tree can reasonably infer an exactness the
production path does not provide.

Measured against CGAL (`Exact_predicates_exact_constructions_kernel`), OCCT
and Manifold on identical operands (`axiolid/benchmarks`):

- **Well-conditioned input costs nothing.** Chaining 64 rotated subtractions,
  axiolid's relative error against derived ground truth is 5.62e-16 — flat in
  chain length, and better than CGAL (1.12e-15) and OCCT (3.71e-15). f64
  construction is not a liability here.
- **Near-degenerate input does cost.** Intersecting two unit cubes overlapping
  by `d`, axiolid tracks CGAL down to `d = 1e-12` (4.73e-5 vs 3.34e-5) and
  never collapses, but at `d = 1e-15` CGAL is ~8x more accurate (1.12e-1 vs
  8.52e-1). That difference is exact rational constructions.
- OCCT collapses **earliest** (zero volume at `d = 1e-9`) because
  `BRepBuilderAPI_Sewing` runs at a fixed 1e-9 tolerance. An exact kernel is
  not automatically the robust one.
- CGAL's runtime cost is the price of that robustness, not slack: rebuilding
  the shim with `Exact_predicates_inexact_constructions_kernel` made it ~20%
  faster at n=1 and then abort at n=4.

The gap is not that f64 is wrong — it is a deliberate contract, like the `f32`
row below — but that the choice is undocumented, its conditioning threshold
unpublished, and its failure silent: at 1e-15 the boolean returns a badly
wrong volume with no evidence flag, which contradicts the fail-closed stance
of #16.

- Tracked as #81, milestone v0.7.
- Not #77: that widens operand *shapes* for the exact boolean; this is about
  construction arithmetic on the general path.

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

1. **v0.7 — Measurement and validity.** G1, G3, G10, and the diagnosis half
   of G2. Measurement is the oracle everything else is checked against, and
   diagnosis must exist before repair. G10 belongs here because it is
   measured, not built: quantifying the f64 floor is a measurement task, and
   the answer decides whether an exact tier is ever warranted.
2. **v0.8 — Mesh processing breadth.** G4, G5, the repair half of G2.
   Repair depends on v0.7's diagnosis; simplification needs measurement to
   prove it preserved volume within a stated bound.
3. **v0.9 — Solid modelling breadth.** G6, G7, G8, G9. The heaviest
   geometry, and the part that most needs the validity foundation beneath
   it.

This preserves the roadmap's stated principle: each milestone exists because
the next cannot be honestly attempted without it.

## Gaps per comparison library

The sections above are organised by capability. This section is the same
evidence organised by *which system Axiolid is behind*, because the four
compared systems are not peers: two are focused mesh engines, one is an
algorithm toolbox, one is a full CAD kernel. "Behind CGAL" and "behind
boolmesh" mean very different things.

Legend: **Gap** = the compared system does it and Axiolid does not.
**Parity** = Axiolid does it. **Out of scope** = a standing refusal in
`docs/ROADMAP.md`, not a backlog item.

### vs boolmesh (adopted upstream dependency, MPL-2.0)

boolmesh is not a rival kernel — it is the crate Axiolid already vendors as
its mesh-boolean provider (`boolmesh = "0.1.9"`). Its entire public API is
one function, `compute_boolean(&Manifold, &Manifold, OpType)`. Axiolid wraps
it and adds an orientation gate, typed refusals, evidence, and an analytic
box-subtraction fast path the upstream crate does not have.

So Axiolid is **ahead of boolmesh on contract**, and the only genuine gap is
capability the dependency ships that Axiolid does not switch on:

| Capability | boolmesh | Axiolid | Verdict |
| --- | --- | --- | --- |
| Robust mesh boolean | yes | yes, via this crate | Parity |
| Multi-threaded boolean (`rayon` feature) | optional feature | **not enabled** | **Gap — #80** |
| `f32` arithmetic (`f32` feature) | optional feature | not enabled (f64 by design) | Not a gap: f64 is a deliberate contract |
| Orientation validation on input | none | implemented | Axiolid ahead |
| Typed refusal / evidence | `Result<_, String>` | typed `GeomError` + `BooleanEvidence` | Axiolid ahead |
| Analytic box subtraction fast path | none | implemented (`edcc604`) | Axiolid ahead |

Probe: `grep -n 'boolmesh' crates/providers/mesh/boolmesh/Cargo.toml` shows
`boolmesh = "0.1.9"` with no `features` key, so the upstream `rayon` feature
is off.

### vs Manifold (focused triangle-mesh solid engine)

| Capability | Manifold | Axiolid | Verdict |
| --- | --- | --- | --- |
| Guaranteed-manifold boolean | yes | yes (via boolmesh) | Parity |
| Volume / surface area on results | yes | **no provider** | **Gap — #72** |
| Extrude / revolve | yes | yes, incl. exact revolution | Parity |
| Mesh simplification / refine | yes | **none** | **Gap — #75** |
| Convex hull | yes | test-only, no provider | **Gap — #76** |
| Curved B-rep / NURBS | deliberately none | yes | Axiolid ahead |
| Exact analytic results | none | yes | Axiolid ahead |

### vs CGAL (computational-geometry toolbox)

| Capability | CGAL | Axiolid | Verdict |
| --- | --- | --- | --- |
| Exact predicates | `orient2d/3d`, `incircle`, `insphere` | same set implemented | Parity |
| Polyhedral / Nef boolean over general solids | `Nef_3`, PMP corefinement | coaxial prisms only | **Gap — #77** |
| `volume` / `area` / `centroid` | PMP | **no provider** | **Gap — #72** |
| Self-intersection detection | `PMP::self_intersections` | **none in 3D** | **Gap — #73** |
| Mesh repair | PMP repair suite | vocabulary only | **Gap — #74** |
| Surface-mesh simplification | `Surface_mesh_simplification` | **none** | **Gap — #75** |
| Convex hull | `convex_hull_3` | test-only | **Gap — #76** |
| AABB tree / spatial queries | `AABB_tree` | `Bvh` implemented | Parity |
| Delaunay / Voronoi / alpha shapes | strong | none | Out of scope (no named consumer) |
| Minkowski sum | `Nef` Minkowski | none | Narrowed to sphere-offset in #78 |

### vs OpenCASCADE (full CAD kernel)

| Capability | OCCT | Axiolid | Verdict |
| --- | --- | --- | --- |
| General B-rep boolean | `BRepAlgoAPI_*` | coaxial prisms only | **Gap — #77** |
| Mass properties | `BRepGProp` | **no provider** | **Gap — #72** |
| Fillet / chamfer | `BRepFilletAPI_*` | chamfer on one straight edge | **Gap — #79** |
| Offset / shelling / thicken | `BRepOffsetAPI_*` | **none** | **Gap — #78** |
| Shape healing | `ShapeFix` suite | vocabulary only | **Gap — #74** |
| Analytic + spline geometry | strong | curves, surfaces, NURBS ops | Parity on the audited slice |
| Curved B-rep tessellation | `BRepMesh` | implemented | Parity |
| STEP / IGES exchange | strong | none | **Out of scope** — ROADMAP: "not a file parser" |
| Feature history / parametric CAD | strong | none | **Out of scope** — not a CAD authoring workstation |
| C++ dependency | is C++ | pure Rust | **Out of scope** — hard constraint |

### Summary count

| System | Real gaps | Where Axiolid is ahead | Excluded by standing refusal |
| --- | --- | --- | --- |
| boolmesh | 1 (#80) | contract, orientation gate, analytic path | f32 |
| Manifold | 3 (#72, #75, #76) | exact/analytic geometry, NURBS | none |
| CGAL | 6 (#72–#77) | format-neutral contracts, exact B-rep | Delaunay/Voronoi, Minkowski |
| OCCT | 5 (#72, #74, #77, #78, #79) | pure Rust, fail-closed refusals | STEP/IGES, feature history, C++ |

No gap is filed against a standing refusal. Eight issues cover the union of
the four columns.

## Milestones and issues as filed

The proposals above are live on GitHub. This table is the authoritative map
from gap to tracked work.

| Milestone | Issue | Gap | Behind |
| --- | --- | --- | --- |
| [v0.7 — Measurement and validity](https://github.com/axiolid/kernel/milestone/9) | [#72](https://github.com/axiolid/kernel/issues/72) Mass properties: volume, area, centroid, second moments | G1 | OCCT, CGAL, Manifold |
| v0.7 | [#73](https://github.com/axiolid/kernel/issues/73) Self-intersection detection and defect diagnosis | G3 | CGAL, OCCT |
| [v0.8 — Mesh processing breadth](https://github.com/axiolid/kernel/milestone/10) | [#74](https://github.com/axiolid/kernel/issues/74) Opt-in mesh repair with per-modification reports | G2 | CGAL, OCCT |
| v0.8 | [#75](https://github.com/axiolid/kernel/issues/75) Mesh decimation with a bounded, reported deviation | G4 | CGAL, Manifold |
| v0.8 | [#76](https://github.com/axiolid/kernel/issues/76) Convex hull provider | G5 | CGAL, Manifold |
| v0.8 | [#80](https://github.com/axiolid/kernel/issues/80) Enable boolmesh multi-threading behind a feature | — | boolmesh |
| [v0.9 — Solid modelling breadth](https://github.com/axiolid/kernel/milestone/11) | [#77](https://github.com/axiolid/kernel/issues/77) General exact boolean over planar-faced solids | G7 | CGAL, OCCT |
| v0.9 | [#78](https://github.com/axiolid/kernel/issues/78) Solid offset and shelling | G6, G8 | OCCT |
| v0.9 | [#79](https://github.com/axiolid/kernel/issues/79) Constant-radius fillet on a single straight edge | G9 | OCCT |

Nine issues across three milestones. Every issue carries a volume or topology
oracle and a mutation probe naming the specific weakening that must break its
test, matching the discipline used in v0.5 and v0.6.
