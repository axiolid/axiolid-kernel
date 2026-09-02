# Geometry capability comparison: IfcOpenShell kernels, OpenUSD, and Axiolid

**Date:** 2026-08-20

**Axiolid audit revision:** `be3ddce7f465`

**IfcOpenShell audit revision:** `1a6336bd207c`

## Executive conclusion

The compared systems are not interchangeable products at the same layer:

- **Open CASCADE Technology (OCCT)** is the broadest CAD modeling kernel here:
  analytic and spline geometry, B-rep topology, modeling operators, booleans,
  tessellation, healing, measurement, and CAD exchange.
- **CGAL** is the strongest computational-geometry toolbox: exact/adaptive
  predicates, polyhedral set operations, meshing, intersection, reconstruction,
  and spatial algorithms. It is a package collection, not an integrated
  feature-history CAD kernel.
- **Manifold** is the focused high-performance triangle-mesh solid engine:
  guaranteed-manifold results, fast booleans, extrude/revolve/refine, and batch
  execution. It deliberately avoids curved B-rep/NURBS breadth.
- **IfcOpenShell passthrough** is not a general kernel. It converts a small,
  linear-faced subset of IfcOpenShell's neutral taxonomy without booleans or
  opening subtraction. Its strength is minimal transformation, especially as a
  cheap first stage in a hybrid kernel.
- **OpenUSD / UsdGeom** is a scene description, composition, instancing, and
  interchange system. It can *represent* meshes, curves, points, subdivision,
  and NURBS schemas; it is not a CAD construction or robust geometric-computation
  kernel.
- **Axiolid today** has a good format-neutral architecture and real implementation
  in certified predicates, polygon/profile triangulation, extrusion, faceted
  B-rep tessellation, adopted mesh booleans, instancing/placement, and an IFC
  lowering pipeline. Its broad curve/surface/B-rep/spatial/healing APIs are
  mostly representations or contracts, not algorithms yet.

**Recommendation:** implement an adaptive curve-discretization and swept-disk
vertical slice next. It attacks three of the five real IfcOpenShell-only fixture
products and puts both remaining circular approximation mismatches under a
world-space error contract while creating the evaluator substrate needed by later
B-rep work. Follow with bounded half-space and primitive CSG, then shared-edge
curve/surface B-rep tessellation, then a real spatial index. Do not chase
OCCT-wide CAD authoring parity.

## Scope and evidence rules

This is a **source-capability audit**, not a product-marketing matrix. It
complements the broader adoption survey in
`docs/research/geometry-kernel-landscape.md`; this document is narrower and
scores what each named system actually executes.

1. An upstream library capability is credited only where its official manual,
   API documentation, or source exposes it.
2. An IfcOpenShell backend is scored by the operations implemented by its
   IfcOpenShell adapter at the pinned revision, not by everything the upstream
   dependency could theoretically do.
3. Axiolid receives **implemented** credit only for production code exercised by
   tests or the fixture/differential harness. Public structs and traits without a
   provider are marked **representation/contract only**.
4. Performance adjectives are used only where the project documents the design
   or Axiolid has a recorded benchmark. Numbers from differently scoped work are
   not turned into speedup claims.
5. Absence from OpenUSD is not treated as a defect where the operation is outside
   scene-description scope.

### Legend

| Mark | Meaning |
| --- | --- |
| **Strong** | Central, mature capability of the system |
| **Yes** | Implemented capability, but not the system's defining strength |
| **Limited** | Narrow subset or important restrictions |
| **R/C** | Axiolid representation or contract exists; no production provider |
| **Adopted** | Axiolid exposes the capability through a third-party Rust provider |
| **No** | No evidenced implementation in the audited scope |
| **N/A** | Outside the system's intended layer |

## First-order comparison

| System | Actual role | Strongest at | Structural trade-off |
| --- | --- | --- | --- |
| OCCT | Integrated CAD geometry/topology kernel | Curved B-rep modeling, CAD operators, repair and exchange | Very large C++ dependency and tolerance-heavy behavior |
| CGAL | Computational-geometry package collection | Exact predicates/constructions, meshing, polyhedral algorithms, spatial queries | Not one cohesive CAD B-rep authoring kernel |
| Manifold | Manifold triangle-mesh solid engine | Fast, predictable mesh booleans and batch solid operations | Mesh-only; no native analytic/NURBS B-rep |
| IfcOpenShell passthrough | Minimal taxonomy-to-polygon conversion backend | Cheap conversion of simple shells/extrusions; hybrid fallback | No booleans/openings; intentionally tiny input subset |
| OpenUSD / UsdGeom | Scene description, composition and interchange | Layering, references, variants, instancing, animation and renderer-neutral scene data | Represents geometry but does not construct/heal/boolean CAD solids |
| Axiolid | Pure-Rust format-neutral geometry stack plus IFC adapter | Auditable exact predicates, explicit capability seams, IFC lowering, portable mesh pipeline | Most advanced geometry families are represented but not evaluated |

## Capability matrix

The table compares the audited scope, not hypothetical extensions. “IfcOCC”,
“IfcCGAL”, and “IfcManifold” below mean the IfcOpenShell adapters over those
libraries.

| Capability | OCCT / IfcOCC | CGAL / IfcCGAL | Manifold / IfcManifold | Ifc passthrough | OpenUSD | Axiolid |
| --- | --- | --- | --- | --- | --- | --- |
| Scene composition, references, variants | Limited (OCAF/XDE assemblies) | No | Limited transforms/composition | Limited transform/style retention | **Strong** | Limited DAG instances/collections |
| Points, vectors, frames, transforms | **Strong** | **Strong** | Yes | Limited | **Strong** | Yes |
| Certified/exact predicates | No; tolerance-based CAD algorithms | **Strong** | Robust manifold contract, not an exact-predicate API | No | No | **Strong**: `orient2d/3d`, `incircle`, `insphere` |
| Analytic curves | **Strong** | Yes across packages | Limited 2D cross-sections | Lines only | Represented: basis/NURBS curves | **R/C** |
| Analytic and spline surfaces | **Strong** | Yes across specialized packages | No native curved surfaces | Planar polygon faces only | Represented: mesh/subdivision/NURBS patch | **R/C** |
| Curved B-rep topology | **Strong** | Limited; polyhedra/Nef are the core solid models | No; manifold triangle topology | Limited polygon shell | No CAD B-rep | Topology **R/C**; planar faceted tessellation implemented |
| Triangle/polygon mesh | Yes | **Strong** | **Strong** | Yes, narrow conversion | **Strong representation** | Yes |
| Boolean set operations | **Strong** | **Strong** for polyhedral/Nef/PMP domains | **Strong** | **No** | No | **Adopted** mesh provider; batch override implemented |
| Openings/void subtraction in IfcOpenShell | Yes | Yes except `cgal-simple` | Yes | **No** | N/A | Yes at product assembly layer |
| Extrusion | **Strong** | Adapter supports it | **Strong** | Limited polygon extrusion | Representation only | Yes: rectangle/circle/contour/derived profiles |
| Revolution / directrix sweep / loft | **Strong** | Not one integrated adapter path | Revolve yes; narrower than CAD sweep | No | No | **R/C**, no production provider |
| Adaptive curve/surface tessellation | **Strong** | **Strong** meshing ecosystem | N/A: already polygonal | Limited polygon triangulation | No construction algorithm | Local-space profile chord budget plus planar B-rep; no general or transform-aware curve/surface tessellator |
| Spatial indexing and intersection queries | Yes | **Strong**: AABB tree/search/intersections | Limited/internal acceleration | No | Bounds caches, not a geometry-query kernel | **R/C** only |
| Mesh repair / shape healing | **Strong** | **Strong** polygon-mesh processing tools | Limited validation/merge cleanup | No | No | **R/C** only |
| Mass properties / measurement | **Strong** | Yes | Yes | No | Extents/bounds, not solid mass properties | Limited internal gates; public `Measure` has no provider |
| Fillet, chamfer, offset, shell/thicken | **Strong** | Not a cohesive CAD feature set | Limited mesh offset/refine operations | No | No | No |
| CAD exchange | **Strong**: STEP/IGES and more | Mesh/geometry formats by package | Mesh formats/bindings | Through IfcOpenShell output path | **Strong** USD interchange | IFC input/lowering; no general CAD exchange kernel |
| Parallel/GPU execution | Partial/algorithm-specific | Package-specific | Optional TBB CPU parallelism; **no current GPU backend** | No | **Strong imaging**, not geometry construction | Execution seams only; no production GPU geometry algorithm |

### Deployment and licensing

| System | Implementation/dependency shape | License relevant to adoption |
| --- | --- | --- |
| CGAL | Header-heavy C++ package ecosystem; exact number types and packages expand build/link cost | Package-specific GPL/LGPL plus commercial licensing; audit the exact package set [CGAL license] |
| Manifold | Focused C++ library with language bindings and optional TBB CPU parallelism | Apache-2.0 [Manifold license] |
| OCCT | Large C++ toolkit graph and native runtime | LGPL-2.1 with the OCCT exception [OCCT license] |
| IfcOpenShell passthrough | C++ plugin inside IfcOpenShell; avoids a heavyweight modeling-kernel conversion but is not a standalone Rust dependency | Source header is LGPL-3.0-or-later [Ifc passthrough API] |
| OpenUSD | Large C++ scene/composition and imaging stack | Tomorrow Open Source Technology License 1.0 [USD license] |
| Axiolid | Rust workspace; no mandatory C++ kernel; adopted providers stay behind narrow traits | MPL-2.0 workspace license (`Cargo.toml`) |

## System profiles

### CGAL

**What it covers.** CGAL's package manual spans arithmetic/algebra, 2D and 3D
kernels, arrangements, polygons, polyhedra and surface meshes, Nef polyhedra,
polygon-mesh processing, surface/volume mesh generation, intersections,
reconstruction, shape detection, AABB trees, spatial searching, and many
specialized geometry algorithms.[CGAL packages][CGAL Mesh 3][CGAL AABB] The
kernel manual explicitly
separates predicates from constructions and offers inexact, exact-predicate, and
exact-construction kernel choices.[CGAL kernel]

Its strongest fit in this comparison is **certified computational geometry**:
algorithms can select exact predicates without forcing every stored coordinate
through one heavy exact representation. `Nef_polyhedron_3` provides closed set
operations over polyhedra, while Polygon Mesh Processing supplies corefinement,
booleans, remeshing, repair, orientation, measurement, and intersection
operations.[CGAL Nef][CGAL PMP]

The IfcOpenShell adapter is much narrower than all of CGAL. At the pinned
revision it converts shells, extrusions, solids, and boolean results; the full
variant uses `Nef_polyhedron_3` and supports booleans, while `cgal-simple`
explicitly does not.[IfcCGAL]

**Strengths**

- Best exact/adaptive predicate and construction ecosystem in this set.
- Unusually broad meshing, spatial, intersection, and reconstruction toolbox.
- Good source of algorithm designs and independent correctness oracles.

**Trade-offs for Axiolid**

- It is a large C++ template ecosystem, contrary to the pure-Rust dependency
  premise.
- Package breadth is not the same as one coherent CAD B-rep authoring model;
  CGAL is not a drop-in OCCT replacement for fillets, offsets, parametric
  curved faces, healing, and STEP-oriented topology.
- CGAL licensing is package-specific GPL/LGPL with commercial licensing
  available; adoption requires package-by-package review.[CGAL license]

### Manifold

**What it covers.** Manifold owns a manifold triangle-mesh solid representation,
constructors, booleans, transforms, composition/decomposition, extrude,
revolve, smoothing/refinement, level-set construction, surface area/volume, and
language/mesh-I/O bindings. Its project contract is that operations produce
manifold output when given valid manifold input.[Manifold README][Manifold API]

IfcOpenShell's adapter converts extrusions, shells, solids, and boolean results,
and explicitly advertises both openings and boolean support.[IfcManifold]
That is very close to Axiolid's current executed solid path, though Manifold has
broader built-in mesh construction and refinement.

**Strengths**

- Focused, modern and fast C++ solid engine rather than a general CAD framework.
- Manifoldness is a first-class invariant, not an after-the-fact cleanup option.
- Batch-friendly execution and optional TBB CPU parallelism; the project
  documents data-layout and batching constraints explicitly.[Manifold performance]
  The current build has no CUDA/GPU backend.[Manifold build]
- Apache-2.0 licensing is straightforward for consumption.[Manifold license]

**Trade-offs for Axiolid**

- Triangle mesh is the canonical solid. Analytic curves, NURBS surfaces, p-curves,
  and curved B-rep semantics are not preserved as kernel-native objects.
- It cannot by itself close Axiolid's advanced IFC curve/surface lowering gap.
- Axiolid already adopted `boolmesh`; replacing it is justified only by a corpus
  benchmark and failure-rate improvement, not by Manifold's broader reputation.

### Open CASCADE Technology

**What it covers.** OCCT is the only system here that behaves like a traditional
full CAD kernel. Its modeling-data layer joins analytic and B-spline geometry to
oriented B-rep topology. Modeling algorithms cover primitive construction,
sweeps, booleans, fillets/chamfers, offsets/thickening, local modification,
feature operations, and mass properties. Separate toolkits cover triangulation,
shape healing, visualization, application/document data, and STEP/IGES and other
exchange.[OCCT data][OCCT algorithms][OCCT healing][OCCT exchange]

IfcOpenShell's OCCT adapter exposes the broadest taxonomy conversion in its
kernel set: edges, loops, faces, shells, solids, extrusion, revolution, boolean
results, lofts, curve sweeps, B-spline surfaces, opening subtraction, shape
unification, curve conversion, and surface conversion.[IfcOCC]

**Strengths**

- Mature curved B-rep data model and the deepest CAD operation coverage here.
- Practical interoperability and repair tooling accumulated over decades.
- The reference for “can this IFC geometric family be represented without
  flattening it immediately?”

**Trade-offs for Axiolid**

- Large C++ dependency graph, complex ownership/runtime behavior, and a much
  broader product scope than Axiolid needs.
- Robustness is predominantly tolerance-management plus repair, not CGAL-style
  certified predicates throughout.
- LGPL-2.1 with the OCCT exception is permissive enough for many applications,
  but still violates Axiolid's no-C++ architecture.[OCCT license]

**Use as prior art, not a parity target.** Copy the separation of geometry,
topology, modeling algorithms, tessellation, healing, and exchange. Do not copy
its total feature surface or make every IFC mesh request pay for a full CAD
kernel.

### IfcOpenShell passthrough

“Passthrough” is an IfcOpenShell backend listed beside `opencascade`, `cgal`,
`cgal-simple`, and `manifold`; it is not a separate upstream geometry library.
[Ifc kernel list] At revision `1a6336bd207c` its header states that booleans are
unsupported and exposes only shell, solid, and extrusion conversion; opening
conversion returns `false`.[Ifc passthrough API]

The implementation is deliberately narrower still:

- shell edges must be linear;
- every face must have exactly one loop;
- a shell face may contain only three or four edges;
- extrusion bases must be one polygon loop;
- it triangulates polygon caps and emits polygonal side faces;
- it neither computes boolean results nor subtracts openings.

Those restrictions are executable source conditions, not inferred from the
name.[Ifc passthrough implementation]

**Strengths**

- Very small conversion surface and little semantic transformation.
- Useful as a cheap path for already-faceted/simple extruded input.
- Useful in `hybrid-passthrough-opencascade`: IfcOpenShell can try the cheap
  representation first and fall back for unsupported shapes. The hybrid layer
  deliberately skips passthrough when an element has openings.[Ifc hybrid]

**Comparison with Axiolid.** Axiolid already exceeds passthrough in profile
coverage, arbitrary contour triangulation, opening subtraction, booleans,
instances, certified predicates, and faceted B-rep face sizes. Passthrough is
useful architectural evidence for tiered fallback, but it is not a capability
ceiling worth targeting.

### OpenUSD / UsdGeom

OpenUSD describes itself as a system for encoding, composing, and reading
scalable 3D scenes. Its defining mechanisms are layers, references, payloads,
variants, inheritance/specialization, time-sampled properties, and native/point
instancing.[USD introduction][USD instancing]

`UsdGeom` supplies schemas for transforms, meshes, points, basis curves, NURBS
curves, NURBS patches, cameras, primitive shapes, extents, and related scene
properties.[UsdGeom][UsdGeom basis curves][UsdGeom NURBS curves]
[UsdGeom NURBS patch] `UsdGeomMesh` carries polygon topology, orientation,
normals, primvars, creases/corners, and subdivision scheme metadata; it does not
perform solid boolean or B-rep construction.[UsdGeom mesh]

`UsdPhysics` adds schemas and descriptors for rigid bodies, collision shapes,
joints, articulations, and simulation ownership. Those records are intended for
downstream physics implementations; they do not turn OpenUSD into a collision or
solid-modeling algorithm provider.[USD physics]

**Strengths**

- Best composition and instancing model in the comparison.
- Excellent boundary format for rendering, DCC, asset pipelines, animation, and
  large scenes assembled from independently authored layers.
- Renderer-neutral imaging through Hydra and efficient cached bound evaluation.
  [USD bounds]

**Why it is not Axiolid's missing kernel.** NURBS schemas describe data; they do
not supply the evaluator, intersection, trimming, watertight shared-edge
triangulation, booleans, or healing Axiolid lacks. Likewise, Hydra is an imaging
architecture, not a CAD construction backend. OpenUSD would be a valuable
**adapter/output target** after geometry is compiled, not the implementation of
that compilation.

Current OpenUSD source is distributed under the Tomorrow Open Source Technology
License 1.0, an Apache-2.0-derived license with a different trademark section;
it should not be described simply as Apache-2.0.[USD license]

### Axiolid: historical implementation snapshot

The following classification was checked against production source and tests at
`be3ddce7f465`. It is retained as point-in-time research evidence; package names
and capability status below are historical. For current ownership and status,
see the [implemented crate map](/architecture/current-target-crate-map) and
[capability matrix](/capabilities).

#### Implemented and exercised

- Format-neutral scalar/vector/frame/transform/bounds values and explicit
  tolerance/capability contracts (`axiolid-core` and the then-current aggregate `axiolid-kernel`).
- Immutable typed geometry DAG with instances, collections, validation, profile,
  primitive, curve/surface, B-rep, mesh, and construction-intent node families
  (`axiolid-model`). Representation breadth here is not algorithm breadth.
- Certified `orient2d`, `orient3d`, `incircle`, and `insphere`, static filters,
  arbitrary-length expansion arithmetic, exact-path differential gates, and
  degeneracy-controlled benchmarks (`axiolid-reference`).
- Simple polygon triangulation and ring orientation, plus adopted `earcut`
  differential coverage for polygon/profile work.
- Rectangle, circle, arbitrary contour, holes, and derived-profile extrusion to
  triangle meshes (`axiolid-mesh-compile`).
- Planar faceted B-rep tessellation with shared topology (`axiolid-mesh-compile::brep`).
- Triangle-mesh union/intersection/difference through the adopted `boolmesh`
  provider, including a measured disjoint-cutter `subtract_many` override
  (`axiolid-mesh-boolean-boolmesh`).
- DAG compilation with memoization, deep-graph iteration, instance transforms,
  mirrored-winding repair, collections, booleans, extrusions, and faceted B-reps.
- IFC lowering, mapped instances, placement-chain application, relationship-level
  void subtraction, and an end-to-end fixture/differential harness.

#### Represented or contracted, but not implemented

- Lines, circles, ellipses, polylines, and B-spline curves are value types, but
  no production `CurveEvaluator` exists.
- Planes, cylinders, cones, spheres, tori, and B-spline surfaces are value types,
  but no production `SurfaceEvaluator` exists.
- Trimmed/composite/offset curves, swept/offset/bounded surfaces, p-curves, and
  point-on-curve/surface relations exist in the graph; there is no general
  evaluator/intersector for them.
- Exact B-rep topology exists, but curved-edge/curved-face tessellation with
  shared edge sampling does not.
- Primitive and half-space values exist, but the compiler does not execute them.
- Revolution, directrix sweep, and sectioned/lofted-solid instructions exist;
  only extrusion is compiled.
- `SpatialIndex`, `Measure`, `Diagnose`, `Repair`, generic `Tessellator`, and
  `Sweeper` are contracts without production providers.
- CPU runtime/ISA selection and GPU graph adapters exist, but they do not yet
  provide optimized geometry algorithms. These are seams, not acceleration.

#### Measured current position

The committed IfcOpenShell differential report compares 42 common products:
28 agree in volume at the report tolerance; six reference records are absent
from Axiolid, one of those being an intentionally cyclic mapped item that also
fails in IfcOpenShell. The five real missing products are:

| Family | Products | Current cause |
| --- | ---: | --- |
| Swept disk / mapped swept disk | 3 | No directrix evaluator or swept-disk provider |
| Primitive CSG (`IfcBlock` minus extrusion) | 1 | Primitive node is not compiled |
| Half-space clipping | 1 | Half-space node is not compiled/bounded for mesh boolean |

Two already-produced circular/extruded products differ by 29% and 1.6%. The
first is a scaled mapped circle and the second a direct circle extrusion; together
they show that the current local-space profile chord policy is not sufficient under
all transforms, but they do not yet prove one shared root cause. Twelve repeated
faceted B-rep products differ by about 1.2%; both pipelines report the source as
non-manifold/inside-out, so these are a separate topology/placement/tessellation
investigation rather than evidence for silently relaxing the tolerance.

Recorded local strengths are narrow but real:

- `orient3d`: about 69 M predicates/s at 0% degeneracy and 30 M/s at 10%; exact
  escalation tracks injected degeneracy instead of collapsing unpredictably.
- disjoint `subtract_many`: 9.20x faster than the sequential path at 64 cutters,
  with equal volume and approximately neutral genuine worst-case behavior.
- the two-opening wall volume agrees with IfcOpenShell to floating-point noise;
  the larger placed wall agrees to about `1.55e-10` relative after applying
  object placement and centering the volume calculation.

#### Current strengths

- **Auditability:** contracts, exact predicates, fallbacks, tolerances, and
  capability failures are explicit and mutation-verified.
- **Portability:** pure Rust in the core path, no mandatory C++ runtime, scalar
  correctness oracle, and runtime CPU architecture selection.
- **Layering:** IFC interpretation stays above the format-neutral geometry
  packages; representations, execution contexts, and operation providers remain
  distinct.
- **Selective adoption:** earcut and boolmesh are used where they beat writing a
  weaker local replacement, while independent differential oracles remain ours.

#### Current limits

Axiolid is not yet a general CAD kernel, a complete computational-geometry
library, or a rendering scene system. Its public vocabulary is substantially
broader than its executable algorithms. That is acceptable only if the next
work deepens existing representations instead of adding more empty contracts.

## Recommendation: what to implement next

### Priority 0 — scalar curve evaluation, adaptive discretization, and swept disk

This is the next implementation. It has the highest observed return:

- resolves **three of the five** valid products currently absent from the
  differential corpus;
- provides the missing foundation for later curved B-rep tessellation;
- is the right vertical slice to diagnose and resolve the two existing circular
  extrusion discrepancies, because the sampling budget can finally be enforced
  after the complete mapped transform;
- deepens value types and contracts that already exist instead of adding another
  subsystem shell.

Deliver it as three vertical slices, each executable before the next:

1. Implement scalar `CurveEvaluator` providers for line, polyline, circle,
   ellipse, and polynomial/rational B-spline curves, including domains and first
   derivatives.
2. Implement adaptive curve sampling bounded by chord error and tangent-angle
   error in **world space after the effective transform**. Sampling a circle
   before a non-uniform map and then scaling it is not enough: the major axis
   determines the required segmentation.
3. Implement swept disk along line/polyline/composite/trimmed directrices with a
   deterministic transported frame, explicit behavior at tangent discontinuities,
   and caps. Then lower `IfcSweptDiskSolid` into that neutral instruction.

Required gates:

- differential position/derivative tests against independent analytic formulas;
- knot-end, repeated-knot, rational-weight, reversed-domain, near-zero tangent,
  arc-junction, and 180-degree-turn cases;
- sampled curve stays within declared world-space chord error;
- the three missing swept-disk products become common records and agree with
  IfcOpenShell in volume/manifoldness within a declared tessellation tolerance;
- circular products `#74` and `#99` improve from 29%/1.6% disagreement to below
  `1e-3` relative without regressing any currently agreeing record;
- timing table at 1, 100, and 10,000 directrices; no performance claim without
  same-run baselines;
- mutation probes must kill “ignore map scale”, “fixed segment count”, “wrong
  rational denominator”, and “reset frame at each composite segment”.

### Priority 1 — execute primitives and bounded half-space clipping

This closes the remaining two valid missing products with representations that
already exist:

- tessellate `Primitive` variants through the same adaptive policy;
- lower primitive CSG operands into the neutral graph;
- turn an unbounded half-space into a finite cutter using the subject bounds plus
  an explicit, overflow-checked clip margin;
- route the result through the existing mesh-boolean provider.

Do **not** manufacture a global “large cube.” The half-space fixture's 79-million
cubic-metre reference volume demonstrates why a magic world size is unsafe.
The finite cutter must derive from the subject's bounds and operation tolerance.

Gate: all 47 valid IfcOpenShell reference products become common records; bath
CSG and half-space volumes agree independently; translating/scaling the same
case cannot change the answer except by the expected determinant.

### Priority 2 — shared-edge curved B-rep tessellation

Build the first real `Tessellator` provider after Priority 0 gives it curve
evaluation:

1. scalar analytic/B-spline surface evaluation and normals;
2. edge discretization once per topological edge;
3. reuse the identical sample sequence in both adjacent face trims;
4. p-curve/curve-on-surface trimming;
5. adaptive face interior tessellation;
6. seam, pole, periodic-domain, reversed-face, and singular-normal handling.

This is the shortest path toward the useful part of OCCT parity: consuming
advanced curved B-reps without adopting OCCT's entire modeling stack. Per-face
independent tessellation is explicitly rejected because it creates cracks even
when each face individually meets chord tolerance.

### Priority 3 — a real static spatial index and query provider

Every broad system in the comparison has spatial acceleration; Axiolid has only
the zero-allocation `SpatialIndex` trait. Implement or adopt a static BVH/AABB
tree with AABB overlap and ordered ray queries, then add nearest-point only when
a consumer requires it.

Before writing one, run a focused Rust dependency survey. The selection gates are
build weight, unsafe footprint, f64 support, caller-owned keys, deterministic
build/query order, callback queries, AArch64/wasm portability, and measurable
performance against brute force. Integrate it into cutter grouping only above a
measured crossover; the current `n <= 64` linear scan is intentionally cheaper
than building an index.

### Priority 4 — measurement, diagnosis, and explicit repair

Implement `Measure<TriMesh>` and topological diagnosis before automatic healing:

- centred/expansion-backed signed volume and centroid;
- surface area and closed-shell mass properties;
- boundary, non-manifold, duplicate, degenerate, inconsistent-winding, and
  self-intersection diagnostics;
- structured failure for open or invalid solids rather than plausible numbers.

Then add narrow, opt-in repairs whose reports say exactly what changed. OCCT's
shape-healing breadth is useful prior art; silently applying an OCCT-style repair
pipeline inside compilation is not.

### Priority 5 — only then broaden CAD construction

Revolution, loft/sectioned solid, shelling/offset, fillet, chamfer, and curved
B-rep booleans are valuable, but they should follow the evaluator/tessellator and
validity foundations. Implement them when the IFC corpus or a named application
provides fixtures and an oracle. “OCCT has it” alone is not a product requirement.

## What not to implement next

| Temptation | Decision |
| --- | --- |
| Replace boolmesh with Manifold | **No**, unless a same-corpus benchmark shows materially better correctness or throughput. The current provider is load-bearing and measured. |
| Reimplement all CGAL packages | **No.** Borrow algorithms and use CGAL as an oracle; preserve the focused pure-Rust surface. |
| Full OCCT parity | **No.** Prioritize IFC ingestion and neutral geometry execution, not a generic CAD authoring workstation. |
| OpenUSD “kernel” | **No.** Add a USD export/scene adapter later; do not expect schemas or Hydra to evaluate B-reps or compute booleans. |
| GPU mesh boolean | **No.** Current workload is branchy, topological, precision-sensitive, and too small to amortize transfers; ADR 0002 already records this. |
| New contracts/crates without consumers | **No.** The repository has enough seams. The next changes must execute existing curve, sweep, primitive, half-space, tessellation, spatial, or measure contracts. |
| Implicit global healing | **No.** Diagnose first; repair only under an explicit plan with an audit report. |

## Decision summary

Axiolid should not chase the broadest row count. Its defensible position is:

1. **IfcOpenShell-like IFC interpretation breadth** above the kernel;
2. **Manifold-like fast mesh-solid execution** where discretization is acceptable;
3. **CGAL-like certified predicates and explicit robustness** at algorithm
   boundaries;
4. **OCCT-like separation of geometry, topology, tessellation, and healing**, but
   only the curved B-rep consumption features IFC actually needs;
5. **OpenUSD-like instancing/export interoperability** at the scene boundary,
   without confusing composition with geometry computation;
6. a smaller, pure-Rust, measurable implementation with structured unsupported
   results instead of silent fallback.

The immediate next vertical slice is therefore **curve evaluation -> adaptive
world-space sampling -> swept disk -> corpus differential result**. It retires a
measured coverage gap, unlocks curved B-rep work, and exercises existing
architecture. Primitive/half-space execution follows; only then should the work
move to general curved B-rep tessellation and spatial queries.

## Sources

All web sources were accessed 2026-08-20. IfcOpenShell links are pinned to the
locally audited revision rather than a moving branch.

### CGAL

- [CGAL packages]: <https://doc.cgal.org/latest/Manual/packages.html>
- [CGAL kernel]: <https://doc.cgal.org/latest/Kernel_23/index.html>
- [CGAL Nef]: <https://doc.cgal.org/latest/Nef_3/index.html>
- [CGAL PMP]: <https://doc.cgal.org/latest/Polygon_mesh_processing/index.html>
- [CGAL Mesh 3]: <https://doc.cgal.org/latest/Mesh_3/index.html>
- [CGAL AABB]: <https://doc.cgal.org/latest/AABB_tree/index.html>
- [CGAL license]: <https://www.cgal.org/license.html>

### Manifold

- [Manifold README]: <https://github.com/elalish/manifold>
- [Manifold API]: <https://manifoldcad.org/docs/html/classmanifold_1_1_manifold.html>
- [Manifold performance]: <https://github.com/elalish/manifold/wiki/Performance-Considerations>
- [Manifold build]: <https://github.com/elalish/manifold/blob/master/CMakeLists.txt>
- [Manifold license]: <https://github.com/elalish/manifold/blob/master/LICENSE>

### Open CASCADE Technology

- [OCCT data]: <https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_data.html>
- [OCCT algorithms]: <https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_algos.html>
- [OCCT healing]: <https://dev.opencascade.org/doc/overview/html/occt_user_guides__shape_healing.html>
- [OCCT exchange]: <https://dev.opencascade.org/doc/overview/html/occt_user_guides__de_wrapper.html>
- [OCCT license]: <https://dev.opencascade.org/resources/licensing>

### IfcOpenShell kernel adapters

- [Ifc kernel list]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcconvert/IfcConvert.cpp#L340-L345>
- [IfcOCC]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/kernels/opencascade/opencascade_kernel.h#L123-L152>
- [IfcCGAL]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/kernels/cgal/cgal_kernel.h#L104-L128>
- [IfcManifold]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/kernels/manifold/manifold_kernel.h#L23-L34>
- [Ifc passthrough API]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/kernels/passthrough/passthrough_kernel.h#L12-L28>
- [Ifc passthrough implementation]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/kernels/passthrough/passthrough_kernel.cpp>
- [Ifc hybrid]: <https://github.com/IfcOpenShell/IfcOpenShell/blob/1a6336bd207c8cdd2dc0013263a00b7eccb2a349/src/ifcgeom/hybrid_kernel.h#L55-L70>

### OpenUSD

- [USD introduction]: <https://openusd.org/release/intro.html>
- [UsdGeom]: <https://openusd.org/release/api/usd_axiolid_page_front.html>
- [UsdGeom mesh]: <https://openusd.org/release/api/class_usd_axiolid_mesh.html>
- [UsdGeom basis curves]: <https://openusd.org/release/api/class_usd_axiolid_basis_curves.html>
- [UsdGeom NURBS curves]: <https://openusd.org/release/api/class_usd_axiolid_nurbs_curves.html>
- [UsdGeom NURBS patch]: <https://openusd.org/release/api/class_usd_axiolid_nurbs_patch.html>
- [USD instancing]: <https://openusd.org/release/api/_usd__page__scenegraph_instancing.html>
- [USD bounds]: <https://openusd.org/release/api/class_usd_axiolid_b_box_cache.html>
- [USD physics]: <https://openusd.org/release/api/usd_physics_page_front.html>
- [USD license]: <https://github.com/PixarAnimationStudios/OpenUSD/blob/release/LICENSE.txt>

### Axiolid evidence

- Representation breadth: `crates/representations/modeling/graph/src/node.rs`
- Curve/surface values and unimplemented evaluator contracts:
  `crates/representations/analytic/curve/src/` and
  `crates/representations/analytic/surface/src/`
- Exact predicates and scalar polygon work: `crates/algorithms/reference/src/`
- Executed solid generation: `crates/algorithms/construction/construct/src/`
- Executed compiler families: `crates/execution/compile/src/compiler.rs`
- Faceted B-rep path: `crates/execution/compile/src/brep.rs`
- Adopted booleans and batching: `crates/providers/mesh/boolmesh/src/`
- Geometry provider contracts: `crates/contracts/common/src/`
- Focused operation/provider seams:
  `crates/algorithms/query/spatial/src/`,
  `crates/algorithms/query/measure/src/`,
  `crates/algorithms/repair/heal/src/`, and
  `crates/contracts/operations/tessellate/src/`
- Differential results: `docs/benchmarks/differential-ifcopenshell.md`
- Predicate ownership and limits: `docs/adr/0016-predicate-ownership-and-adopted-implementations.md`
- Deferred optimization triggers: `docs/adr/0013-deferred-performance-techniques.md`
