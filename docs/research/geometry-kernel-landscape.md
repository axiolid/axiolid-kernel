# Geometry Kernel Architecture & The Rust Geometry/CAD Ecosystem

**Research report for the `axiolid` pure-Rust IFC geometry kernel.**
Compiled 2026-08-19. All crate versions, licenses, download counts and repo
activity figures below were pulled directly from the crates.io API
(`https://crates.io/api/v1/crates/<name>`) and the GitHub REST API
(`https://api.github.com/repos/<owner>/<repo>`) on 2026-08-19, not from
secondary sources.

Throughout, **[MEASURED]** marks a figure read from an API or from source code
I fetched; **[IMPRESSION]** marks my own judgement.

---

## 0. Executive summary of the licensing situation

`axiolid` is MIT. The relevant constraint is *inbound* license compatibility.

| License of dependency | Can an MIT project depend on it? | Note |
|---|---|---|
| MIT, `MIT OR Apache-2.0` | Yes, freely | Ideal |
| Apache-2.0 (alone) | Yes | Permissive, but adds a patent-grant + NOTICE obligation and is one-way incompatible with GPLv2. Your *own* license stays MIT; you just ship attribution. |
| 0BSD | Yes, freely | Most permissive of all — no attribution required |
| MPL-2.0 | Yes, with care | **File-level copyleft.** Linking/using is fine and does not infect your MIT code. But modifications *to MPL-licensed files* must stay MPL and be published. Must be noted in NOTICE. |
| LGPL-2.1 | Problematic | Dynamic linking only; static linking (Rust's default) triggers relinking obligations. Effectively unusable for a statically-linked Rust lib. |
| GPL | No | Excluded by the brief |

**[MEASURED]** Key license facts relevant to the candidate set:

- **truck** (all crates): `Apache-2.0` — *not* dual MIT/Apache. Usable, attribution required.
- **fornjot** (`fj`, `fj-core`, `fj-math`): `0BSD` — maximally permissive; code can be lifted verbatim with no attribution obligation.
- **opencascade-rs**: `LGPL-2.1` **and** binds C++ — excluded on both counts.
- **csgrs**: `MIT`.
- **manifold3d / manifold-csg**: `Apache-2.0 OR MIT`, but binds the C++ manifold library — excluded on the "no C++" rule, not on license.
- **boolmesh**: `MPL-2.0` — pure Rust, but file-level copyleft.
- **parry / rapier / nalgebra / cgmath**: `Apache-2.0`.
- **glam, spade, earcut, lyon, i_overlay, geo, robust, cavalier_contours, rstar**: `MIT OR Apache-2.0`.
- **i_float**: `MIT`.

---

## 1. The Rust geometry/CAD ecosystem, crate by crate

### 1.1 Summary table

**[MEASURED]** — crates.io + GitHub API, 2026-08-19. "Recent" = recent downloads
as reported by crates.io.

| Crate / repo | Latest ver | License | Pure Rust? | Last release | Last commit | Stars | Downloads (all / recent) |
|---|---|---|---|---|---|---|---|
| `truck-*` (ricosjp/truck) | 0.4–0.6 | Apache-2.0 | **Yes** | 2024-09-20 | 2026-08-10 | 1,531 | truck-base 71k / 29k |
| `fj*` (hannobraun/fornjot) | 0.49.0 | **0BSD** | **Yes** | 2024-03-21 | 2026-06-19 | 2,556 | fj 82k / 770 |
| `opencascade` / `-sys` | 0.2.0 | LGPL-2.1 | **No — C++ OCCT** | 2023-08-16 | 2026-08-03 | 261 | 4.9k / 830 |
| `csgrs` | 0.20.1 | MIT | **Yes** | 2025-07-24 | 2026-07-31 | 248 | 41k / 5.7k |
| `manifold3d` / `manifold-csg` | 0.4.0 | Apache-2.0 OR MIT | **No — C++ manifold** | 2026-08-08 | 2026-08-08 | 19 (binding) / 2,234 (upstream) | 19k / 15.6k |
| `boolmesh` | 0.1.9 | **MPL-2.0** | **Yes** | 2026-02-12 | 2026-05-07 | 31 | 9.9k / 7.5k |
| `parry3d` | 0.30.2 | Apache-2.0 | **Yes** | 2026-08-08 | 2026-08-07 | 855 | 2.26M / 607k |
| `rapier3d` | 0.35.2 | Apache-2.0 | **Yes** | 2026-08-15 | — | (same repo) | 1.49M / 364k |
| `nalgebra` | 0.35.0 | Apache-2.0 | **Yes** | 2026-05-24 | 2026-06-30 | 4,775 | 84.4M / 15.0M |
| `glam` | 0.33.4 | MIT OR Apache-2.0 | **Yes** | 2026-08-18 | 2026-08-18 | 2,033 | 124.2M / 46.6M |
| `cgmath` | 0.18.0 | Apache-2.0 | **Yes** | **2021-01-03** | — | — | 11.6M / 1.4M |
| `spade` | 2.15.1 | MIT OR Apache-2.0 | **Yes** | 2026-03-24 | 2026-03-24 | 336 | 17.6M / 4.5M |
| `earcut` (georust) | 0.4.11 | MIT OR Apache-2.0 | **Yes** | 2026-07-26 | 2026-07-26 | 57 | 1.33M / 852k |
| `lyon_tessellation` | 1.0.20 | MIT OR Apache-2.0 | **Yes** | 2026-03-21 | 2026-05-03 | 2,596 | 6.26M / 1.58M |
| `i_overlay` | 8.1.0 | MIT OR Apache-2.0 | **Yes** | 2026-08-16 | 2026-08-16 | 203 | 6.93M / 3.28M |
| `geo` | 0.33.1 | MIT OR Apache-2.0 | **Yes** | 2026-04-20 | 2026-08-12 | 1,914 | 20.9M / 4.19M |
| `robust` (georust) | 1.2.0 | MIT OR Apache-2.0 | **Yes** | 2025-05-10 | 2025-05-10 | 107 | 22.1M / 5.0M |
| `cavalier_contours` | 0.8.0 | MIT OR Apache-2.0 | **Yes** | 2026-08-10 | 2026-08-16 | 228 | 47.5k / 28k |
| `rstar` | 0.13.0 | MIT OR Apache-2.0 | **Yes** | 2026-05-24 | — | — | 39.2M / 12.6M |

### 1.2 Per-crate notes

#### truck — https://github.com/ricosjp/truck
**[MEASURED]** Apache-2.0, pure Rust, 1,531 stars, actively committed
(last commit 2026-08-10) but **crates.io releases are stale — last publish
2024-09-20**, roughly two years behind `master`. This gap matters: the
published crates do not contain the newer `truck-assembly`, `truck-drafting`
or the fillet/healing work visible in the repo tree.

Covers: NURBS B-rep modelling, topology, mesh algorithms, boolean ops
(`truck-shapeops`), STEP I/O (`truck-stepio`), and wgpu rendering.
Detailed architecture in §3.

**[MEASURED]** Dependency graph of the core crates is clean and small:
`truck-base` → `cgmath`, `matext4cgmath`, `rustc-hash`, `serde`.
`truck-topology` → `parking_lot`, `rayon`, `rustc-hash`, `serde`, `thiserror`.
No C++ anywhere. **[IMPRESSION]** The `cgmath` dependency is the notable
liability — cgmath's last release was 2021-01-03, i.e. it is effectively
unmaintained, and truck's entire type vocabulary (`Point3`, `Vector3`,
`Matrix4`) is re-exported from it via `truck-base/src/cgmath64.rs`.

#### fornjot — https://github.com/hannobraun/fornjot
**[MEASURED] The repository is ARCHIVED as of 2026-06-19.** GitHub reports
`archived: true` and the description reads *"Early-stage b-rep CAD kernel,
written in the Rust programming language. **No longer in development.**"*
Last crates.io release 0.49.0 on 2024-03-21. 2,556 stars.

**[MEASURED]** License is **0BSD** (crates.io `license` field for `fj`,
`fj-core`, `fj-math`), which is the most permissive option in the whole
candidate set — no attribution requirement at all. For a dead-but-instructive
codebase this is significant: its ideas *and its code* can be adopted freely.

Detailed architecture and the author's published post-mortem in §2.

#### opencascade-rs — https://github.com/bschwind/opencascade-rs
**[MEASURED]** LGPL-2.1, 261 stars, last crates.io release 0.2.0 on
2023-08-16 (nearly 3 years stale), repo still receiving commits (2026-08-03),
66 open issues. It is a `cxx`-based binding to the C++ OpenCASCADE kernel.

**Excluded from axiolid on two independent grounds:** it puts C++ (and a large
OCCT build) in the dependency graph, and LGPL-2.1 is incompatible with static
Rust linking in an MIT project.

#### csgrs — https://github.com/timschmidt/csgrs
**[MEASURED]** MIT, 248 stars, v0.20.1 (2025-07-24), last commit 2026-07-31,
only 5 open issues. MSRV 1.85.1. Self-described "multi-modal constructive
solid geometry kernel in Rust".

**[MEASURED]** Required dependencies: `core2`, `doc-image-embed`, `either`,
`geo`, `hashbrown`, `nalgebra`, `robust`, `thiserror`. Large set of *optional*
deps behind features (`parry3d`, `rapier3d`, `chull`, `dxf`, `svg`,
`fast-surface-nets`, `stl_io`, `wgpu-types`, `bevy_*`, …). Pure Rust in its
default configuration.

**[IMPRESSION]** This is a *mesh/BSP* CSG kernel, not a B-rep kernel — it is
the closest Rust analogue to OpenSCAD's model, not to Parasolid's. Its
license (MIT) and its use of `geo` + `robust` make it the most
license-friendly reference implementation in the set. Notably it already
composes exactly the pieces (`geo`, `nalgebra`, `robust`) that a new kernel
would reach for.

#### manifold3d / manifold-csg — https://github.com/zmerlynn/manifold-csg
**[MEASURED]** `Apache-2.0 OR MIT`, v0.4.0 published 2026-08-08, very active.
But its README states plainly: *"Safe Rust bindings to the manifold3d
geometry kernel… manifold3d is a fast, robust **C++ library**."* The upstream
`elalish/manifold` repo is `language: C++`, Apache-2.0, 2,234 stars.

Provides: `Manifold` solids (booleans, hull, Minkowski, SDF level sets,
warp), `CrossSection` 2D regions (Clipper2-based offsetting), `MeshGL`/
`MeshGL64`, constrained Delaunay triangulation, extrude/revolve/slice.

**Excluded from axiolid: C++ in the dependency graph.** It is however the
functional benchmark for what "robust mesh booleans" means.

#### boolmesh — https://github.com/komietty/boolmesh
**[MEASURED]** **MPL-2.0**, pure Rust, 31 stars, v0.1.9 (2026-02-12), last
commit 2026-05-07. **[MEASURED]** Dependencies are minimal: required `glam`;
optional `rayon`, `bevy`, `bevy_panorbit_camera`, `tobj`.

**[MEASURED]** From its README: *"a pure Rust library for performing robust
and efficient mesh boolean operations. It is a full-from-scratch Rust
implementation inspired by Elalish's Manifold."* Supports f32 and f64. The
author reports a depth-4 Menger sponge in ~8 s single-threaded / ~4 s
multi-threaded on an Apple M4. **API surface is deliberately tiny** — one
entry point, `compute_boolean(&Manifold, &Manifold, OpType)`; as of v0.1.9 the
author removed primitive generators and transforms to keep it "lean and
specialized". Inputs must be closed manifolds (no boundaries, no
self-overlap).

**[IMPRESSION]** This is the single most interesting crate in the set for a
pure-Rust kernel: it is the only pure-Rust implementation of Manifold-class
robust mesh booleans. The MPL-2.0 file-level copyleft is a real but bounded
cost — using it as a dependency is fine and does not affect axiolid's MIT
licensing; only edits to boolmesh's own files would need to be published
under MPL.

#### parry / rapier — https://github.com/dimforge/parry
**[MEASURED]** Apache-2.0, pure Rust, `parry3d` 0.30.2 (2026-08-08),
2.26M all-time downloads. 855 stars.

**[MEASURED] Major architectural news:** parry **migrated off nalgebra onto
glam** (via a new `glamx` crate) in v0.26.0, commit `72f842d`, dated
2026-01-09, +13,378/−13,759 across 300 files. The changelog states this was
done *"for future compatibility with `rust-gpu`"* and is *"a major breaking
change affecting almost all public APIs."* This is confirmed by the current
dependency list of `parry3d` 0.30.2, which includes `glamx` and no longer
`nalgebra`. Discussed by Dimforge at
https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/.

Covers: AABB/BVH/QBVH broad phase, convex hulls, point/ray queries,
distance, time-of-impact, contact manifolds, shape primitives, trimesh
queries. **[IMPRESSION]** Relevant to axiolid for spatial indexing
(`axiolid-spatial`) and proximity queries, *not* for B-rep — parry is a
collision library and its "geometry" is approximate by design.

#### nalgebra vs glam vs cgmath
**[MEASURED]**

| | nalgebra 0.35.0 | glam 0.33.4 | cgmath 0.18.0 |
|---|---|---|---|
| License | Apache-2.0 | MIT OR Apache-2.0 | Apache-2.0 |
| Last release | 2026-05-24 | **2026-08-18** | **2021-01-03** |
| All-time downloads | 84.4M | **124.2M** | 11.6M |
| Recent downloads | 15.0M | **46.6M** | 1.4M |
| Open issues | 429 | 16 | — |
| MSRV | 1.89.0 | 1.68.2 | — |

**[MEASURED]** glam now exceeds nalgebra on both total and recent downloads,
has 16 open issues against nalgebra's 429, and shipped a release the day
before this report. cgmath is effectively abandoned (no release in 5½ years)
yet truck depends on it.

**[IMPRESSION]** The trade-off is real and well understood:
- **nalgebra**: generic over scalar and dimension (`Const<N>`/`Dyn`),
  full linear algebra (decompositions, solvers), needed if you want e.g. SVD
  for fitting, least-squares for surface approximation, or dimension-generic
  code. Heavier compile times; the type signatures are famously verbose.
- **glam**: concrete `Vec3`/`DVec3`/`Mat4`/`DMat4` types, SIMD-accelerated,
  no genericity, fast to compile, ergonomic. No decompositions.
- The `f64` question matters for CAD/IFC: glam does provide `DVec3`/`DMat4`
  f64 types, so glam is not disqualified on precision grounds. But glam's
  SIMD advantage is largely an f32 story.
- The ecosystem signal is mixed but moving: parry (a *geometry* library, not
  a game engine) chose glam in 2026; boolmesh chose glam; `geo`/`spade`/
  `robust` are agnostic and use their own point traits; truck uses cgmath;
  csgrs uses nalgebra.

#### spade — https://github.com/Stoeoef/spade
**[MEASURED]** `MIT OR Apache-2.0`, pure Rust, v2.15.1 (2026-03-24),
17.6M downloads, 336 stars, 14 open issues.
**[MEASURED]** Dependencies: `hashbrown`, `num-traits`, `robust`, `smallvec`
(+ optional `mint`, `serde`) — so it builds on Shewchuk-style exact
predicates via `robust`.

Provides: 2D Delaunay triangulation, **constrained Delaunay triangulation
(CDT)**, Voronoi (dual), vertex removal, bulk and incremental loading,
optional hierarchy structure for fast nearest-neighbour/insertion, natural
neighbour interpolation.

**[MEASURED]** truck's own `truck-meshalgo` takes `spade` as an optional
dependency — i.e. a real B-rep kernel already uses spade for its trimming
triangulation.

#### earcut (georust) & lyon
**[MEASURED]** `earcut` 0.4.11, `MIT OR Apache-2.0`, 1.33M downloads,
last release 2026-07-26. Its only dependency is `num-traits` — the leanest
possible triangulator. It is a Rust port of Mapbox's earcut ear-clipping
algorithm.

**[MEASURED]** `lyon` 1.0.19 / `lyon_tessellation` 1.0.20, `MIT OR Apache-2.0`
(both LICENSE-MIT and LICENSE-APACHE present in-repo), 2,596 stars, 6.26M
downloads for `lyon_tessellation`. Split into `lyon_geom` (9.66M dl — Bézier/
arc/line math, flattening, offsetting), `lyon_path` (8.82M dl — path
representation and iterators), `lyon_tessellation` (fill and stroke
tessellation with a scanline fill tessellator).

**[IMPRESSION]** The `lyon_geom` / `lyon_path` / `lyon_tessellation` split is
itself a good model of layering: pure math → data structure → algorithm, each
usable standalone. `lyon_geom`'s download count being ~50% higher than
`lyon_tessellation`'s is evidence people do consume the math layer alone.
Earcut is faster and simpler but produces lower-quality triangles and does not
handle self-intersection; lyon's fill tessellator is more robust for arbitrary
paths.

#### i_overlay / i_float — https://github.com/iShape-Rust/iOverlay
**[MEASURED]** `i_overlay` 8.1.0 (`MIT OR Apache-2.0`), `i_float` 4.1.0
(`MIT`). 6.93M / 6.74M downloads. Last releases 2026-08-16 and 2026-08-15 —
very actively maintained. 203 stars, only 2 open issues. MSRV 1.88/1.85.
**[MEASURED]** Deps: `i_float`, `i_key_sort`, `i_shape`, `i_tree` (+ optional
`rayon`) — an entirely self-owned, pure-Rust stack.

Covers: 2D polygon boolean (intersection, union, difference, xor),
self-intersection resolution, and (per the repo description) related overlay
operations.

**[MEASURED]** Crucially, `geo` 0.33.1 depends on `i_overlay` — georust
adopted it as the engine behind its boolean ops.

**[IMPRESSION]** The `i_float` split is the interesting design point: it is a
separate fixed-point/integer arithmetic crate. i_overlay achieves robustness
by snapping to an integer grid rather than by adaptive-precision floating
point — a fundamentally different robustness strategy from Shewchuk
predicates, and one worth understanding before choosing.

#### geo (georust) — https://github.com/georust/geo
**[MEASURED]** `MIT OR Apache-2.0` (confirmed from `geo/Cargo.toml`:
`license = "MIT OR Apache-2.0"`; the GitHub API reports `NOASSERTION` only
because of the dual-license file layout). 20.9M downloads, 1,914 stars,
117 open issues, v0.33.1 (2026-04-20), commits through 2026-08-12.
`geo-types` 0.7.20 has 24.9M downloads and MSRV 1.75.

**[MEASURED]** Dependencies: `float_next_after`, `geo-types`,
`geographiclib-rs`, `i_overlay`, `log`, `num-traits`, `rand`, `rand_pcg`,
`robust`, `rstar`, `sif-itree` (+ optional `earcut`, `proj`, `serde`,
`spade`).

**[IMPRESSION]** `geo` is geospatial-first: its primitives are 2D, its
algorithms assume planar or geodetic coordinates, and it carries geodesy
baggage (`geographiclib-rs`) irrelevant to a CAD kernel. But `geo-types`
alone is a lightweight, extremely widely-used 2D primitive vocabulary, and
`geo`'s composition (`i_overlay` for booleans, `robust` for predicates,
`rstar` for indexing, `earcut`/`spade` for triangulation) is a validated
recipe for the 2D layer.

#### robust (georust) — https://github.com/georust/robust
**[MEASURED]** `MIT OR Apache-2.0`, pure Rust, v1.2.0 (2025-05-10),
**22.1M all-time / 5.0M recent downloads**, 107 stars. No release in
~15 months, but this is a *port of a finished algorithm*, so staleness is
weak evidence of neglect. **[IMPRESSION]** Its adoption is the key fact:
`spade`, `geo`, and `csgrs` all depend on it. It is the de-facto standard
Rust implementation of Shewchuk's adaptive-precision predicates.

Note there is also an unrelated `robust-predicates` crate
(https://crates.io/crates/robust-predicates, https://github.com/hporro/robust-predicates)
— **[IMPRESSION]** far less adopted; `robust` is the ecosystem default.

#### Also worth knowing: cavalier_contours
**[MEASURED]** https://github.com/jbuckmccready/cavalier_contours —
`MIT OR Apache-2.0`, pure Rust, v0.8.0 (2026-08-10), commits through
2026-08-16, 228 stars, MSRV 1.88. "2D polyline/shape library for offsetting,
combining, etc." **[IMPRESSION]** Directly relevant to IFC profile handling
(`axiolid-profile`): it handles **arc-segment polylines** (bulge-encoded), which
is exactly what IFC `IfcCompositeProfileDef` / `IfcArbitraryClosedProfileDef`
with arc segments needs, and which `geo`/`i_overlay` (straight-segment only)
do not.

---

## 2. Fornjot's architecture and its published lessons

### 2.1 Status
**[MEASURED]** Archived 2026-06-19. The author, Hanno Braun, published a
post-mortem: **"Shutting Down Fornjot"**,
https://archive.hannobraun.com/fornjot/blog/shutting-down-fornjot/

Key measured facts from that post: first commit **2020-07-30**; roughly
**6 years** of development; sponsor-funded; ended without shipping a useful
tool. The author's own framing: *"Fornjot is ending; unfinished,
incomplete."*

### 2.2 Crate split
**[MEASURED]** From the repository tree at the final state:

```
crates/
  fj           — top-level convenience/facade crate
  fj-core      — the kernel proper
  fj-math      — math primitives (no CAD semantics)
  fj-interop   — interchange types between kernel and consumers
  fj-export    — export to mesh/CAD file formats
  fj-viewer    — rendering
  fj-window    — windowing/app shell
experiments/   — dated prototype directories (see §2.4)
models/
tools/
```

**[MEASURED]** `fj-math` contents: `aabb`, `arc`, `circle`, `coordinates`,
`line`, `plane`, `point`, `poly_chain`, `scalar`, `segment`, `transform`,
`triangle`, `vector`. Note `scalar.rs` — a dedicated scalar wrapper type,
and `coordinates.rs` — explicit coordinate-system handling.

**[MEASURED]** `fj-core` internal module split — this is the interesting part:

```
fj-core/src/
  core.rs
  layers/          layer.rs, layers.rs, objects.rs, geometry.rs,
                   presentation.rs, validation.rs
  objects/         any_object.rs, is_object.rs, object_set.rs, stores.rs,
                   kinds/{vertex, half_edge, cycle, region, face, shell,
                          sketch, solid, curve, surface}.rs
  geometry/        boundary.rs, geometry.rs, half_edge.rs, path.rs, surface.rs
  storage/         blocks.rs, handle.rs, store.rs
  operations/      build/, insert/, update/, replace/, reverse/, split/,
                   join/, merge/, sweep/, transform/, geometry/, holes.rs,
                   derive.rs, presentation.rs
  algorithms/      approx/ (per-object approximation + tolerance.rs),
                   bounding_volume/, intersect/, triangulate/ (delaunay.rs,
                   polygon.rs)
  queries/         all_half_edges_with_surface.rs,
                   bounding_vertices_of_half_edge.rs,
                   sibling_of_half_edge.rs
  validate/        per-object validation
  validation/      checks/, config.rs, error.rs, validation_check.rs
```

**[IMPRESSION]** Several design decisions are legible from this tree and are
worth naming explicitly, because they are the most transferable part of
Fornjot:

1. **Topology is half-edge based.** `objects/kinds/half_edge.rs`,
   `queries/sibling_of_half_edge.rs`, `validation/checks/half_edge_connection.rs`.
2. **Objects live in a centralised store with handles**, not in an ownership
   tree (`storage/{store,handle,blocks}.rs`, `objects/stores.rs`). Objects are
   referred to by `Handle<T>`, enabling sharing and identity-based comparison.
3. **A "layers" abstraction** (`layers/`) separates concerns that most kernels
   entangle: object storage, geometry, presentation, and validation are each a
   *layer* over the same object graph. This is how geometry got separated from
   topology.
4. **Operations are traits, organised by verb** (`build`, `update`, `insert`,
   `sweep`, `split`, `join`, `reverse`, `replace`, `transform`), each with one
   file per object kind. Extremely regular; makes the API surface predictable.
5. **Validation is first-class and pluggable** — a `ValidationCheck` trait,
   a `ValidationConfig`, and a separate `validation/checks/` directory.
6. **Approximation (tessellation) is a per-object trait with an explicit
   `Tolerance` type** (`algorithms/approx/tolerance.rs`), not a global constant.

### 2.3 Published design lessons — geometry vs topology

**[MEASURED]** The dedicated issue *"Separate geometry from topology"*,
https://github.com/hannobraun/fornjot/issues/2116, drove a large rework late
in the project's life. A follow-on issue,
https://github.com/hannobraun/fornjot/issues/2266 — *"Validation is no longer
reliable in the presence of the geometry layer"* — records that the
separation **broke validation**, i.e. the rework had real, unresolved costs.

**[MEASURED]** Fornjot also published a rationale for choosing B-rep over
alternatives: https://fornjot.app/blog/why-fornjot-is-using-boundary-representation/

**[MEASURED]** `fj-kernel` was renamed to `fj-core` in commit `0d2f1c4`
(https://github.com/hannobraun/fornjot/commit/0d2f1c47c7e7885f45a98b98f722fc8898b95134)
— **[IMPRESSION]** a small but telling signal that "the kernel" stopped being
one crate's job and became the whole workspace's.

### 2.4 Local vs global geometry — the most concrete published lesson

**[MEASURED]** In `experiments/2025-03-18/README.md`
(https://github.com/hannobraun/fornjot/blob/main/experiments/2025-03-18/README.md),
Braun evaluates storing geometry **globally in 3D** versus **locally in
curve/surface parameter space**, after having tried both. Direct quotes:

> "This experiment went with the ostensibly simpler approach of storing all
> geometry in 3D, making object graph constructions *much* simpler… However,
> this came at a cost."

> "These local definitions are still needed in some situations, and getting
> them has turned out to be a problem. It requires additional infrastructure,
> namely the capability of projecting into a curve/surface."

> "The bigger problem is conceptual. By constructing all geometry in 3D, we're
> throwing away information about local coordinates that would be available at
> the time of construction. **This information can't be reconstructed
> reliably, as there are degenerate cases where a 3D coordinate maps to
> multiple 2D ones.**"

> "I think that throwing away information that you can't reliably reconstruct
> later is a very foundational problem. My instincts tell me that this will
> keep causing problems down the line… The problem of redundant, local
> geometry seems more manageable."

His stated conclusion is that **locally-defined geometry (parameter-space) is
worth another try**, despite making the object graph more complex, because
global-3D-only discards information irrecoverably.

**[IMPRESSION]** This is arguably the single most valuable published artefact
from the entire Fornjot project for anyone designing a new kernel: an
empirical, both-ways-tried comparison of the central representational choice.

### 2.5 Process lessons from the post-mortem

**[MEASURED]** Braun's own enumerated mistakes, from
https://archive.hannobraun.com/fornjot/blog/shutting-down-fornjot/:

- **"Extrapolating from Early Success"** — switching from SDF to B-rep showed
  promise; he expected linear progress. *"That linear progress never
  materialized. Instead, I ran into a cliff. Discovering in so many different
  ways why CAD kernels, specifically b-rep kernels, are considered hard."*
- **"Sticking to Incremental Improvements"** — *"Instead of convincing myself
  that a new idea was promising, then spending a long time implementing it in
  small steps, I should have been prototyping. All the time. Only then
  integrating, incrementally or not, what proved valuable."*
- **"Prototyping Came Too Late"** — *"I had maneuvered the codebase into a
  transitionary state, halfway between an old approach and a new one, when I
  realized that the new approach was unlikely to pan out."* He then spent
  **over a year** on the `experiments/` prototypes, arrived at *"a much
  simpler architecture, which I still think shows promise"*, but ran out of
  steam before integrating it.
- **"Allowing My Vision to Become Muddled"** — dropping the application to
  focus on the kernel alone was, in his assessment, a mistake:
  *"An application can be focused. A CAD kernel is a generic piece of
  infrastructure, with many use cases to consider."* And: *"Something can be
  a kernel, a library, but still focus on specific use cases. Be a tool
  instead of a building block."*

**[IMPRESSION]** For axiolid the fourth point is the load-bearing one and cuts
in axiolid's favour: axiolid's kernel is *not* a general-purpose CAD kernel,
it is scoped to what IFC geometry actually requires. That is exactly the
"focused kernel" Braun says he should have built. The scoping constraint is
an asset, and should be defended.

---

## 3. Truck's architecture

### 3.1 Crate split
**[MEASURED]** Workspace members from the repo root (2026-08-19):

| Crate | Role (from truck's README) |
|---|---|
| `truck-base` | "basic structs and traits: importing cgmath, curve and surface traits, tolerance, etc." |
| `truck-geotrait` | "Defines geometric traits: `ParametricCurve`, `ParametricSurface`, and so on." |
| `truck-geometry` | "geometrical structs: knot vector, B-spline and NURBS" |
| `truck-topology` | "topological structs: vertex, edge, wire, face, shell, and solid" |
| `truck-polymesh` | "defines polygon data structure and some algorithms handling mesh" |
| `truck-meshalgo` | "Mesh algorithms, include tessellations of the shape." |
| `truck-modeling` | "integrated modeling algorithms by geometry and topology" |
| `truck-shapeops` | "Provides boolean operations to Solid" |
| `truck-stepio` | STEP import/export |
| `truck-derivers` | proc-macro derives |
| `truck-assembly` | assembly/DAG (repo only, unpublished) |
| `truck-drafting` | 2D drafting (repo only, unpublished) |
| `truck-platform`, `truck-rendimpl` | wgpu rendering |
| `truck-js` | wasm bindings |

**[MEASURED]** The published dependency edges (from the crates.io dependency
API) are strictly layered:

```
truck-base      → cgmath, matext4cgmath, rustc-hash, serde
truck-geotrait  → (truck-base)
truck-geometry  → truck-base, truck-geotrait, serde, thiserror
truck-topology  → truck-base, truck-geotrait, parking_lot, rayon, rustc-hash,
                  serde, thiserror  [opt: rclite]
truck-polymesh  → (base/geometry layer)
truck-meshalgo  → truck-base, truck-geometry, truck-polymesh, truck-topology,
                  array-macro, itertools, rayon, rustc-hash  [opt: spade, vtkio]
truck-modeling  → truck-base, truck-geometry, truck-geotrait, truck-polymesh,
                  truck-topology, derive_more, rustc-hash, serde, thiserror
truck-shapeops  → truck-base, truck-geometry, truck-geotrait, truck-meshalgo,
                  truck-topology, derive_more, itertools, rustc-hash
```

The critical observation: **`truck-topology` depends on `truck-geotrait`
(traits) but NOT on `truck-geometry` (concrete NURBS types).**

### 3.2 How truck separates topology from geometry

**[MEASURED]** This is done by **generic parameters, not by indirection**.
From `truck-topology/src/lib.rs`:

```rust
pub struct Vertex<P>            { /* … */ }
pub struct Edge<P, C>           { /* … */ }
pub struct Wire<P, C>           { /* VecDeque<Edge<P,C>>, via Deref */ }
pub struct Face<P, C, S>        { /* … */ }
pub struct Shell<P, C, S>       { /* Vec<Face<P,C,S>>, via Deref */ }
pub struct Solid<P, C, S>       { /* attached to closed shells */ }
```

`P` = point type, `C` = curve type, `S` = surface type. The topology crate is
*entirely agnostic* about what geometry is — you can (and truck's own tests
do) instantiate it with `P = ()` or `P = usize` purely to test topological
invariants with no geometry at all. **[IMPRESSION]** This is a cleaner
separation than Fornjot's layer approach and costs nothing at runtime; the
price is generic-parameter noise throughout every downstream signature.

**[MEASURED]** **Identity is explicit and separate from value.** truck defines:

```rust
pub type VertexID<P> = ID<Mutex<P>>;
pub type EdgeID<C>   = ID<Mutex<C>>;
pub type FaceID<S>   = ID<Mutex<S>>;
```

with documented semantics, quoted from the source:

- `Vertex::new()` *"creates a different vertex each time"* — two vertices with
  the same point are **not** equal.
- `Edge::new()` *"create a different edge each time, even if the end vertices
  are the same one."*
- **The ID is orientation-independent**: `edge0.inverse().id() == edge0.id()`
  but `edge0.inverse() != edge0`. Same for faces.
- **The ID is stable under geometric mutation**: the doc test shows
  `v.set_point(1)` changes the point but `v.id()` is unchanged.
- IDs are `Copy`, explicitly so they can be used as hash-map keys without
  cloning the entity.

**[IMPRESSION]** This "identity is `Copy`, orientation-free, and stable across
geometry edits" contract is a very well-considered design point and is exactly
what B-rep boolean algorithms need for bookkeeping.

**[MEASURED]** Geometry is stored behind `Mutex` (`ID<Mutex<P>>`,
`parking_lot` dependency), i.e. shared mutable geometry with interior
mutability, so that a curve shared by two edges updates for both.

### 3.3 The geometric trait vocabulary (`truck-geotrait`)

**[MEASURED]** Verbatim from
`truck-geotrait/src/traits/{mod,curve,surface}.rs`:

**Core orientation/transform traits** (`mod.rs`):
```rust
pub trait Invertible: Clone {
    fn invert(&mut self);
    fn inverse(&self) -> Self { /* default via clone+invert */ }
}
pub trait Transformed<T>: Clone {
    fn transform_by(&mut self, trans: T);
    fn transformed(&self, trans: T) -> Self { /* default */ }
}
pub trait ToSameGeometry<T> { fn to_same_geometry(&self) -> T; }
pub type ParameterRange = (Bound<f64>, Bound<f64>);
```

**Curves** (`curve.rs`):
```rust
pub trait ParametricCurve: Clone {
    type Point;
    type Vector: Zero + Copy;
    fn subs(&self, t: f64) -> Self::Point;      // evaluate
    fn der(&self, t: f64) -> Self::Vector;      // 1st derivative
    fn der2(&self, t: f64) -> Self::Vector;     // 2nd derivative
    fn der_n(&self, n: usize, t: f64) -> Self::Vector;   // nth derivative
    fn ders(&self, n: usize, t: f64) -> CurveDers<Self::Vector>;
    fn parameter_range(&self) -> ParameterRange { (Unbounded, Unbounded) }
    fn try_range_tuple(&self) -> Option<(f64, f64)>;
    fn period(&self) -> Option<f64> { None }    // periodicity is first-class
}

pub trait BoundedCurve: ParametricCurve {
    fn range_tuple(&self) -> (f64, f64);
    fn front(&self) -> Self::Point;
    fn back(&self) -> Self::Point;
}

pub trait ParametricCurve2D: ParametricCurve<Point = Point2, Vector = Vector2> {}
pub trait ParametricCurve3D: ParametricCurve<Point = Point3, Vector = Vector3> {}

pub trait ParameterDivision1D {
    type Point;
    /// Creates the curve division (parameters, corresponding points).
    fn parameter_division(&self, range: (f64, f64), tol: f64)
        -> (Vec<f64>, Vec<Self::Point>);
}

pub trait ParameterTransform: BoundedCurve {
    fn parameter_transform(&mut self, scalar: f64, r#move: f64) -> &mut Self;
    fn parameter_normalization(&mut self) -> &mut Self;   // → range (0,1)
}

pub trait Concat<Rhs>: BoundedCurve {
    type Output: BoundedCurve<...>;
    fn try_concat(&self, rhs: &Rhs) -> Result<Self::Output, ConcatError<Self::Point>>;
}
```

**Surfaces** (`surface.rs`):
```rust
pub trait ParametricSurface: Clone {
    type Point;
    type Vector: Zero + Copy;
    fn subs(&self, u: f64, v: f64) -> Self::Point;
    fn uder(&self, u: f64, v: f64) -> Self::Vector;
    fn vder(&self, u: f64, v: f64) -> Self::Vector;
    fn uuder(&self, u: f64, v: f64) -> Self::Vector;
    fn uvder(&self, u: f64, v: f64) -> Self::Vector;
    fn vvder(&self, u: f64, v: f64) -> Self::Vector;
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector;
    fn ders(&self, max_order: usize, u: f64, v: f64) -> SurfaceDers<Self::Vector>;
    fn parameter_range(&self) -> (ParameterRange, ParameterRange);
    fn u_period(&self) -> Option<f64> { None }
    fn v_period(&self) -> Option<f64> { None }
}

pub trait ParametricSurface3D: ParametricSurface<Point = Point3, Vector = Vector3> {
    fn normal(&self, u: f64, v: f64) -> Vector3;          // default: uder × vder
    fn normal_uder(&self, u: f64, v: f64) -> Vector3;
    fn normal_vder(&self, u: f64, v: f64) -> Vector3;
}

pub trait BoundedSurface: ParametricSurface {
    fn range_tuple(&self) -> ((f64, f64), (f64, f64));
}

pub trait IncludeCurve<C: ParametricCurve> {
    /// Returns whether the curve `curve` is included in the surface `self`.
    fn include(&self, curve: &C) -> bool;
}

pub trait ParameterDivision2D {
    fn parameter_division(&self, range: ((f64,f64),(f64,f64)), tol: f64) -> ...;
}

pub trait SearchParameter { /* in search_parameter.rs — inverse evaluation */ }
```

**[IMPRESSION]** Notable design choices worth calling out:
- **Bounded vs unbounded is a trait distinction**, not a runtime flag.
  `ParametricCurve` may be unbounded (an infinite line); `BoundedCurve` is the
  refinement that guarantees endpoints. This is exactly right for IFC, which
  has both `IfcLine` (unbounded) and trimmed curves.
- **Periodicity is first-class** (`period()`, `u_period()`, `v_period()`),
  which is essential for cylinders, cones, tori, and closed B-splines.
- **Arbitrary-order derivatives** (`der_n`, `der_mn`, `ders`) rather than a
  fixed 0/1/2 set — needed for curvature, offsets, and blends.
- **Tessellation is a trait on geometry, parameterised by tolerance**
  (`ParameterDivision1D/2D`), so each curve/surface kind decides how to
  subdivide itself. Tessellation is *not* a monolithic external algorithm.
- **Blanket impls for `&C` and `Box<C>`** are provided throughout, so trait
  objects and references compose without friction.

### 3.4 Geometry organisation (`truck-geometry`)
**[MEASURED]** Three-way split:
- `nurbs/` — `knot_vec`, `bspcurve`, `bspsurface`, `nurbscurve`, `nurbssurface`
- `specifieds/` — `line`, `circle`, `plane`, `sphere`, `torus`, `parabola`,
  `hyperbola` (analytic primitives kept exact rather than converted to NURBS)
- `decorators/` — `pcurve`, `processor`, `trimmied_curve`, `extruded_curve`,
  `revolved_curve`, `intersection_curve`, `offset/{curve,surface}`,
  `homotopy`, `edge_blend`, `rbf_surface`, `af_surface`, `scalar_function`

**[IMPRESSION]** The `decorators/` pattern is the most reusable idea here and
is a form of **lazy geometry evaluation**: an extruded curve is stored as
"this curve + this direction", not baked into a NURBS surface. `Processor` is
a decorator that carries a transform + orientation flag, so transforming a
surface does not rewrite its control points. `PCurve` represents a curve
defined *in the parameter space of a surface* — i.e. truck does support
Fornjot's "local geometry", as a decorator. `IntersectionCurve` represents a
surface–surface intersection implicitly rather than approximating it eagerly.
This directly addresses the exactness/laziness concern in §4.6.

### 3.5 Tolerance model
**[MEASURED]** `truck-base/src/tolerance.rs`, verbatim:

```rust
pub const TOLERANCE: f64 = 1.0e-6;
pub const TOLERANCE2: f64 = TOLERANCE * TOLERANCE;

pub trait Tolerance: AbsDiffEq<Epsilon = f64> + Debug {
    fn near(&self, other: &Self)  -> bool { self.abs_diff_eq(other, TOLERANCE) }
    fn near2(&self, other: &Self) -> bool { self.abs_diff_eq(other, TOLERANCE2) }
}
impl<T: AbsDiffEq<Epsilon = f64> + Debug> Tolerance for T {}
```
plus `assert_near!` / `assert_near2!` / `prop_assert_near!` macros (the last
for proptest integration).

**[IMPRESSION]** This is a **single global absolute tolerance**, blanket-
implemented over everything that is `AbsDiffEq`. It is simple and pervasive,
but it is *not* a tolerant-modelling scheme: there is no per-entity tolerance,
no relative/model-size scaling, and no way to widen tolerance on a specific
imported edge. Contrast with Parasolid (§4.3). For IFC — which routinely
carries models in millimetres with coordinates in the 10^6 range, and
imported/degenerate geometry — **[IMPRESSION]** a single 1e-6 absolute
constant is likely to be a genuine limitation.

---

## 4. Modern geometry kernel design patterns in the literature

### 4.1 Separation of topology from geometry
The classical statement is that a B-rep model has two orthogonal parts:
*topology* (the incidence graph: which vertices bound which edges, which
edges bound which faces) and *geometry* (the point/curve/surface data
attached to those topological cells).

**[MEASURED]** Three approaches are visible in the wild:
- **Generic parameters** — truck: `Face<P, C, S>`. Topology crate has zero
  dependency on concrete geometry. (§3.2)
- **Layers over a shared object graph** — Fornjot's `layers/` module, with
  geometry as one layer and objects as another, driven by issue
  https://github.com/hannobraun/fornjot/issues/2116. **[MEASURED]** This
  approach caused a regression tracked at
  https://github.com/hannobraun/fornjot/issues/2266 ("Validation is no longer
  reliable in the presence of the geometry layer").
- **Attribute/pointer on the topological entity** — the classical OCCT/ACIS/
  Parasolid arrangement, where a `TopoDS_Edge` references a `Geom_Curve`.

**[IMPRESSION]** The Fornjot experience is a useful caution: separation is
correct in principle but the *validation* story must be designed alongside it,
because validity conditions are inherently mixed (a face is valid only if its
boundary curves actually lie on its surface — a joint geometric/topological
predicate).

### 4.2 The geometric predicates problem
Floating-point evaluation of sign predicates (orientation, in-circle,
in-sphere) can return the *wrong sign*, which in a combinatorial algorithm
does not produce a slightly-wrong answer — it produces an inconsistent
combinatorial structure, infinite loops, or crashes.

**[MEASURED]** The canonical reference is Jonathan Shewchuk,
*"Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric
Predicates"*, Discrete & Computational Geometry 18(3):305–363, 1997:
- Paper: https://people.eecs.berkeley.edu/~jrs/papers/robustr.pdf
- Short version: https://people.eecs.berkeley.edu/~jrs/papers/robust-predicates.pdf
- Code/overview: https://www.cs.cmu.edu/~quake/robust.html
- Journal: https://link.springer.com/article/10.1007/PL00009321

The technique is *adaptive*: compute a fast floating-point estimate with a
rigorous error bound; only if the estimate's magnitude is within the error
bound, escalate to exact expansion arithmetic. The common case costs almost
nothing; correctness is unconditional.

**[MEASURED]** In Rust this is the `robust` crate
(https://github.com/georust/robust, `MIT OR Apache-2.0`, 22.1M downloads),
depended on by `spade`, `geo`, and `csgrs`. A separate `robust-predicates`
crate also exists (https://github.com/hporro/robust-predicates) —
**[IMPRESSION]** much less adopted.

**[MEASURED]** An alternative robustness strategy is visible in
`i_overlay`/`i_float`: **snap to an integer grid and compute exactly in
fixed-point**, rather than adaptively in floating point. `i_float` is a
standalone `MIT` crate for this purpose. **[IMPRESSION]** This trades
input fidelity (coordinates are quantised) for unconditional exactness and
simpler code, and is the reason i_overlay's boolean ops are fast and robust.
It is a legitimate and different point in the design space, but quantisation
is a poor fit for CAD/IFC where absolute coordinate ranges vary hugely.

**[IMPRESSION]** The key architectural consequence: **predicates must be a
separate, tiny, dependency-light module used by everything else**, exactly as
`robust` is in the georust stack. Predicates are *sign* questions, distinct
from *constructions* (computing an intersection point), and only the former
can be made exact cheaply. Kernels get into trouble when they conflate the two.

### 4.3 Tolerance models
Three broad models appear in practice:

1. **Single global epsilon.** truck's `TOLERANCE = 1e-6` (§3.5). Simple,
   fails on models whose coordinate magnitudes vary.
2. **Session precision + per-entity local precision ("tolerant modelling").**
   **[MEASURED]** Parasolid's documented model: session precision is
   **1.0e-8 units**, and *"Distances less than this are considered zero"*
   (http://www.q-solid.com/Parasolid_Docs_V35/chapters/ov_chap.04.html,
   http://www.q-solid.com/Parasolid_Docs/headers/pk_session_set_precision.html).
   Parasolid additionally allows raising the tolerance **on individual edges
   and vertices** via `PK_EDGE_set_precision` / tolerant modelling
   (http://www.q-solid.com/Parasolid_Docs_V35/chapters/fd_chap.017.html) —
   explicitly *"to use Tolerant Modelling by setting local precision on edges
   and vertices in parts created by importing models from non-Parasolid
   powered applications."*
   ACIS documents the equivalent concept in its *Tolerant Modeling* chapter:
   http://www-isl.ece.arizona.edu/ACIS-docs/PDF/KERN/06TMOD.PDF
3. **Exact/algebraic**, avoiding tolerance altogether — academically clean,
   generally impractical for NURBS intersections.

**[IMPRESSION]** For an **IFC** kernel, model (2) is the relevant target,
because IFC data is by definition imported from heterogeneous authoring tools
and arrives with exactly the sloppy-edge problem that tolerant modelling was
invented for. The Parasolid documentation is the clearest public specification
of how that is structured.

### 4.4 Topological data structures

| Structure | Manifold-ness | Notes |
|---|---|---|
| **Winged-edge** (Baumgart, 1972) | 2-manifold, orientable | Each edge stores 2 vertices, 2 faces, 4 wing edges. Traversal requires case analysis on orientation — the classic annoyance. |
| **Half-edge / DCEL** | 2-manifold, orientable | Each edge split into two opposite half-edges; each half-edge knows next, twin, origin vertex, incident face. Traversal is branch-free. The de-facto modern default. |
| **Radial-edge** (Weiler, 1985/1986) | **Non-manifold** | Adds a radial cycle of edge-uses around an edge, so an edge can bound *N* faces (N≠2). Required for non-manifold models: dangling faces, wire edges, multiple solids meeting at an edge. |
| **BMesh** (Blender) | Non-manifold | Practical modern non-manifold design; documented at https://developer.blender.org/docs/features/objects/mesh/bmesh/ |

**[MEASURED]** References:
- Weiler, *"Edge-Based Data Structures for Solid Modeling in Curved-Surface
  Environments"*, IEEE CG&A, 1985 — https://dl.acm.org/doi/10.1109/MCG.1985.276271
- Weiler, *"Topological Structures for Geometric Modeling"* —
  https://papers.cumincad.org/cgi-bin/works/paper/47c5
- MIT 2.158J Computational Geometry lecture notes on B-rep structures —
  https://ocw.mit.edu/courses/2-158j-computational-geometry-spring-2003/f04f923ac8e0af56d19095b4de8dea3c_lecnotes14_fixed.pdf
- Muuss & Butler, *"Combinatorial Solid Geometry, Boundary Representations,
  and Non-Manifold Geometry"* (BRL-CAD) —
  https://ftp.arl.army.mil/~mike/papers/90nmg/joined.html
- Survey: *"An Overview on Boundary Representation Data Structures for 3D
  Models Representation"* —
  https://www.academia.edu/4898337/An_Overview_on_Boundary_Representation_Data_Structures_for_3D_Models_Representation
- NVIDIA SMLib topology documentation —
  https://docs.nvidia.com/smlib/manual/smlib/topology/index.html

**[MEASURED]** In Rust: Fornjot chose **half-edge** (`objects/kinds/half_edge.rs`,
`queries/sibling_of_half_edge.rs`). truck chose a **face/wire/edge boundary
representation without explicit half-edges** — orientation is carried by
`Edge`'s `inverse()` and an orientation-independent `EdgeID`, and a `Wire` is
an ordered `VecDeque<Edge>`.

**[IMPRESSION]** The manifold/non-manifold question is a hard fork in the road
and is *not* purely academic for IFC: IFC models legitimately contain
non-manifold configurations (shells that are not closed, faces meeting at an
edge, wire geometry). Choosing half-edge closes off non-manifold cases;
choosing radial-edge pays complexity everywhere. truck's approach (no explicit
edge-adjacency structure at all; adjacency is derived when needed) sidesteps
the choice at the cost of slower adjacency queries.

### 4.5 B-rep vs mesh
The two representations answer different questions and most real systems carry
both:
- **B-rep**: exact analytic/NURBS surfaces, exact topology, supports
  parametric edit, offsetting, filleting, exact mass properties. Booleans are
  hard (surface–surface intersection is the crux) and non-robust.
- **Mesh**: triangles only, approximate, but booleans are tractable and can be
  made robust (manifold, boolmesh). Required for rendering and for most
  downstream consumers.

**[MEASURED]** Fornjot published its rationale for choosing B-rep:
https://fornjot.app/blog/why-fornjot-is-using-boundary-representation/

**[MEASURED]** Every kernel in this survey maintains an explicit
**B-rep → mesh** direction as a first-class, tolerance-parameterised operation
and none supports the reverse:
- truck: `truck-meshalgo/src/tessellation/` + `ParameterDivision1D/2D` traits
  taking a `tol: f64`.
- Fornjot: `fj-core/src/algorithms/approx/` with one module per object kind
  and an explicit `tolerance.rs`.

**[IMPRESSION]** The consistent pattern is that tessellation is
*per-entity and tolerance-driven*, defined next to the geometry that knows how
to subdivide itself, rather than being a single monolithic mesher. Both
kernels independently arrived at this.

### 4.6 Lazy / deferred evaluation of geometry
**[MEASURED]** truck's `truck-geometry/src/decorators/` is a concrete
implementation of this: `ExtrudedCurve`, `RevolvedCurve`, `TrimmedCurve`,
`OffsetCurve`/`OffsetSurface`, `IntersectionCurve`, `PCurve`, `Processor`, and
`Homotopy` all *wrap* other geometry and evaluate on demand, rather than
converting eagerly to NURBS control points.

**[IMPRESSION]** The benefits are: (a) exactness is preserved — a revolved
line stays an exact cylinder rather than becoming an approximating NURBS
patch; (b) transforms are O(1) (`Processor` stores a matrix); (c) memory is
proportional to the construction history, not to the sampled result. The costs
are: deep decorator stacks make evaluation slow, and every algorithm must go
through the trait interface rather than pattern-matching on concrete types
(which in turn is why the "specifieds" — plane, sphere, torus — are kept as
distinct concrete types that algorithms *can* special-case).

**[IMPRESSION]** This also connects back to §2.4: `PCurve` (a curve in a
surface's parameter space) is truck's answer to exactly the local-vs-global
geometry dilemma Braun described, implemented as an opt-in decorator rather
than a global representational commitment.

---

## 5. Cross-cutting observations

**[MEASURED]** Points established by direct evidence:

1. **Fornjot is dead** (archived 2026-06-19) but **0BSD**, so its code and
   design are freely reusable with no attribution obligation, and its author
   published an unusually candid post-mortem.
2. **truck is the only live, pure-Rust, permissively-licensed B-rep kernel**
   in the set — but its crates.io releases have been frozen since 2024-09-20
   while `master` moves, and it is Apache-2.0 (not MIT), and it depends on the
   abandoned `cgmath`.
3. **Every C++-binding option is excluded** by the pure-Rust constraint
   (opencascade-rs, manifold3d/manifold-csg), and opencascade-rs is
   additionally LGPL-2.1.
4. **`boolmesh` is the only pure-Rust robust mesh boolean implementation**,
   and it is MPL-2.0 (usable, file-level copyleft, must be noted).
5. **`robust` is the ecosystem-standard predicates crate** — `spade`, `geo`,
   and `csgrs` all depend on it; 22.1M downloads; `MIT OR Apache-2.0`.
6. **The 2D stack is mature and MIT-friendly**: `i_overlay` (booleans),
   `spade` (CDT), `earcut`/`lyon_tessellation` (triangulation),
   `cavalier_contours` (arc-aware polyline offsetting), all
   `MIT OR Apache-2.0` and all actively released within the last 5 months.
7. **The math-library centre of gravity is shifting toward glam**: glam has
   overtaken nalgebra on downloads, and parry — a geometry library — migrated
   nalgebra → glam in v0.26.0 (2026-01-09) explicitly for rust-gpu
   compatibility.

**[IMPRESSION]** The two kernels converge on more than they diverge on, which
is itself informative. Both separate a pure-math crate from the kernel; both
make tessellation a per-entity tolerance-driven operation; both keep analytic
primitives distinct from NURBS; both treat validation as a distinct concern.
They differ on: topology representation (half-edge vs ordered-wire), how
geometry is bound to topology (layers vs generic parameters), and whether
geometry lives in local parameter space or global 3D — and it is precisely
these three that Braun identifies as the sources of his difficulty.

---

## 6. Source index

**Repositories / crates**
- truck — https://github.com/ricosjp/truck
- fornjot — https://github.com/hannobraun/fornjot
- opencascade-rs — https://github.com/bschwind/opencascade-rs
- csgrs — https://github.com/timschmidt/csgrs
- manifold (C++) — https://github.com/elalish/manifold
- manifold-csg (bindings) — https://github.com/zmerlynn/manifold-csg
- boolmesh — https://github.com/komietty/boolmesh
- parry — https://github.com/dimforge/parry
- nalgebra — https://github.com/dimforge/nalgebra
- glam — https://github.com/bitshifter/glam-rs
- spade — https://github.com/Stoeoef/spade
- lyon — https://github.com/nical/lyon
- i_overlay — https://github.com/iShape-Rust/iOverlay
- geo — https://github.com/georust/geo
- robust — https://github.com/georust/robust
- earcut — https://github.com/georust/earcut
- cavalier_contours — https://github.com/jbuckmccready/cavalier_contours

**Fornjot design writing**
- Post-mortem — https://archive.hannobraun.com/fornjot/blog/shutting-down-fornjot/
- Why B-rep — https://fornjot.app/blog/why-fornjot-is-using-boundary-representation/
- Separate geometry from topology — https://github.com/hannobraun/fornjot/issues/2116
- Validation regression after geometry layer — https://github.com/hannobraun/fornjot/issues/2266
- Local vs global geometry experiment — https://github.com/hannobraun/fornjot/blob/main/experiments/2025-03-18/README.md
- `fj-kernel` → `fj-core` rename — https://github.com/hannobraun/fornjot/commit/0d2f1c47c7e7885f45a98b98f722fc8898b95134
- Future evolution of object storage — https://github.com/hannobraun/fornjot/discussions/1454

**truck source consulted**
- `truck-geotrait/src/traits/mod.rs`, `curve.rs`, `surface.rs`
- `truck-topology/src/lib.rs`
- `truck-base/src/tolerance.rs`
- README — https://github.com/ricosjp/truck/blob/master/README.md

**Predicates & robustness**
- Shewchuk 1997 (full) — https://people.eecs.berkeley.edu/~jrs/papers/robustr.pdf
- Shewchuk (short) — https://people.eecs.berkeley.edu/~jrs/papers/robust-predicates.pdf
- CMU Quake predicates page — https://www.cs.cmu.edu/~quake/robust.html
- Springer DCG — https://link.springer.com/article/10.1007/PL00009321

**Tolerance models**
- Parasolid model structure (session precision 1e-8) — http://www.q-solid.com/Parasolid_Docs_V35/chapters/ov_chap.04.html
- Parasolid session & local precision — http://www.q-solid.com/Parasolid_Docs_V35/chapters/fd_chap.017.html
- `PK_SESSION_set_precision` — http://www.q-solid.com/Parasolid_Docs/headers/pk_session_set_precision.html
- `PK_EDGE_set_precision` — http://www.q-solid.com/Parasolid_Docs/headers/pk_edge_set_precision.html
- ACIS Tolerant Modeling — http://www-isl.ece.arizona.edu/ACIS-docs/PDF/KERN/06TMOD.PDF

**Topological data structures**
- Weiler, Edge-Based Data Structures (IEEE CG&A 1985) — https://dl.acm.org/doi/10.1109/MCG.1985.276271
- Weiler, Topological Structures for Geometric Modeling — https://papers.cumincad.org/cgi-bin/works/paper/47c5
- MIT 2.158J notes — https://ocw.mit.edu/courses/2-158j-computational-geometry-spring-2003/f04f923ac8e0af56d19095b4de8dea3c_lecnotes14_fixed.pdf
- BRL-CAD non-manifold geometry — https://ftp.arl.army.mil/~mike/papers/90nmg/joined.html
- Blender BMesh — https://developer.blender.org/docs/features/objects/mesh/bmesh/
- B-rep data structure survey — https://www.academia.edu/4898337/An_Overview_on_Boundary_Representation_Data_Structures_for_3D_Models_Representation
- Mesh data structures (Utah) — https://www.sci.utah.edu/~jmk/papers/cs6620_02_datastructs.pdf

**Ecosystem**
- parry glam migration commit — https://github.com/dimforge/parry/commit/72f842d9dab8c3729d45ee9b26efe86fc744d1c1
- parry CHANGELOG — https://github.com/dimforge/parry/blob/master/CHANGELOG.md
- Dimforge 2025 review / 2026 goals — https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/
- spade CDT docs — https://docs.rs/spade/latest/spade/struct.ConstrainedDelaunayTriangulation.html
