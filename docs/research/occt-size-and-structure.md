# OpenCascade Technology (OCCT): Module Structure, Code Size, and Relevance to a BIM/IFC Geometry Pipeline

**Purpose:** Factual reference on what OCCT ships, how big it is, and which parts an IFC→triangles pipeline actually exercises.
**Report date:** 2026-08-19

## 0. Provenance of numbers in this report

Three classes of numbers appear below. They are labelled everywhere.

| Label | Meaning |
|---|---|
| **[MEASURED]** | Counted by me on this machine from a real checkout / real binary packages. Commands are reproducible; see §7. |
| **[CITED]** | Stated by an upstream/third-party source, with URL. |
| **[ESTIMATE]** | Derived/inferred. Explicitly marked, never presented as fact. |

**Measurement subjects:**

* **Source:** `github.com/Open-Cascade-SAS/OCCT` master, commit `7d2efad9c8a9a57ea96c4c8587134b34dd503cd8` (2026-08-10), version `OCC_VERSION 8.1.0` per `adm/cmake/version.cmake`. Source: https://github.com/Open-Cascade-SAS/OCCT
* **Binaries:** Debian trixie `libocct-*` **7.8.1+dfsg1-3**, amd64, stripped release shared libraries as shipped. Source: https://packages.debian.org/trixie/libocct-foundation-7.8
* Source counts are **physical lines** of `.cxx/.hxx/.h/.c/.lxx/.pxx` unless stated as "code-only" (pygount, comments+blanks excluded).
* Note the version skew: source is 8.1.0-dev, binaries are 7.8.1. Ratios between modules are stable across these; absolute numbers should not be mixed across the two.

---

## 1. Top-level module structure

OCCT's own module list is authoritative and machine-readable in the repo — `src/MODULES.cmake` lists exactly seven modules: **[MEASURED]**

```
FoundationClasses  ModelingData  ModelingAlgorithms  Visualization
ApplicationFramework  DataExchange  Draw
```

(Source: `src/MODULES.cmake`, https://github.com/Open-Cascade-SAS/OCCT/blob/master/src/MODULES.cmake)

Note that **"Shape Healing" is documented as a top-level component but is *not* a separate module** — it is the single toolkit `TKShHealing` inside ModelingAlgorithms. The official overview page lists it alongside the modules, which is a common source of confusion. (Docs: https://dev.opencascade.org/doc/overview/html/index.html)

Also note **`Deprecated/`** exists as an 8th source directory but produces no toolkit (contents: `NCollectionAliases` only). **[MEASURED]**

### 1.1 Measured size per module

GTests (in-tree unit tests) excluded from all rows. **[MEASURED]**

| Module | Toolkits (TK*) | Leaf packages | Files | Physical lines | Code-only LOC (pygount) |
|---|---:|---:|---:|---:|---:|
| **FoundationClasses** | 2 | 39 | 895 | 271,781 | ~145,600 |
| **ModelingData** | 4 | 51 | 1,306 | 491,831 | ~210,000 |
| **ModelingAlgorithms** | 14 | 109 | 3,011 | 847,409 | ~454,600 |
| **Visualization** | 7 | 29 | 1,130 | 279,409 | ~144,900 |
| **ApplicationFramework** (OCAF) | 13 | 42 | 832 | 121,969 | ~64,800 |
| **DataExchange** | 14 | 102 | 5,292 | 632,024 | ~293,800 |
| **Draw** (test harness) | 20 | 35 | 416 | 191,295 | ~103,700 |
| **Deprecated** | 0 | — | 972 | 34,459 | ~10,100 |
| **TOTAL** | **74** | **372** | **13,854** | **2,870,177** | — |

(Code-only column is from a separate pygount pass over the same trees; it includes GTests in some module dirs so treat it as approximate. The physical-line column is the exact, GTests-excluded figure.)

### 1.2 What each module does, and its toolkits

**FoundationClasses** — 2 toolkits. The base layer that everything links against.
* `TKernel` (19 pkgs, 513 files, 161k lines) — `Standard` (root object/RTTI/memory), `NCollection`/`TCollection`/`TColStd` (containers), `OSD` (OS abstraction), `Message`, `Quantity`, `Units`, `Resource`, `Storage`/`FSD` (persistence primitives), `Plugin`.
* `TKMath` (22 pkgs, 553 files, 184k lines) — `gp` (points/vectors/transforms), `math` + `Math*` (linear algebra, root finding, optimisation, integration), `BSplCLib`/`BSplSLib` (B-spline evaluation kernels), `PLib`, `ElCLib`/`ElSLib` (analytic curve/surface evaluation), `Poly` (triangulation containers), `Bnd` (bounding boxes), `BVH`, `Convert`, `TopLoc` (location/transform sharing).
* Docs: https://dev.opencascade.org/doc/overview/html/occt_user_guides__foundation_classes.html

**ModelingData** — 4 toolkits. Geometry and topology *representation* (no algorithms).
* `TKG2d` (135 files, 31k) — `Geom2d`, `Adaptor2d`.
* `TKG3d` (280 files, 88k) — `Geom` (3D curves/surfaces incl. NURBS), `GeomAdaptor`, `TopAbs`.
* `TKGeomBase` (750 files, **325k lines** — the single largest ModelingData toolkit) — `Approx`, `AppDef`, `AppCont`, `AdvApp2Var`, `Extrema`, `ProjLib`, `GeomConvert`, `GeomLib`, `GProp`, `IntAna`, `BndLib`, `GCPnts`, `FEmTool`, `GC`/`gce` construction helpers.
* `TKBRep` (334 files, 131k) — `TopoDS` (the B-Rep topology data structure), `BRep`, `BRepTools`, `BRepAdaptor`, `TopExp`, `TopTools`, `BinTools`.
* Docs: https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_data.html

**ModelingAlgorithms** — 14 toolkits, the largest module by lines. Everything that *computes*.
| Toolkit | Files | Lines | Role |
|---|---:|---:|---|
| `TKGeomAlgo` | 851 | 208,601 | curve/surface intersection (`IntPatch`, `IntSurf`, `IntWalk`, `IntCurve`, `GeomInt`, `ApproxInt`), `GeomFill`, `GeomAPI`, `Geom2dGcc`, `Plate`/`NLPlate`, hatching |
| `TKBool` | 423 | 146,329 | `TopOpeBRep*` legacy boolean machinery, `BRepFill`, `BRepAlgo` |
| `TKFillet` | 248 | 92,998 | fillets and chamfers (`ChFi3d`, `Blend`, `BRepFilletAPI`) |
| `TKShHealing` | 255 | 89,438 | **Shape Healing** — `ShapeFix`, `ShapeAnalysis`, `ShapeUpgrade`, `ShapeConstruct`, `ShapeCustom`, `ShapeExtend` |
| `TKTopAlgo` | 352 | 87,149 | `BRepBuilderAPI`, `BRepLib`, `BRepCheck`, `BRepClass`/`BRepClass3d` (classification), `BRepExtrema`, `BRepGProp`, `BRepBndLib`, `MAT`/`Bisector` (medial axis) |
| `TKBO` | 207 | 81,440 | modern Boolean Operations: `BOPAlgo` (34k lines), `IntTools` (18k), `BOPDS`, `BOPTools`, `BRepAlgoAPI` |
| `TKOffset` | 73 | 47,373 | offsets, thick solids, draft |
| `TKHLR` | 227 | 44,149 | hidden line removal (drawing generation) |
| `TKFeat` | 94 | 30,499 | feature modelling (holes, ribs, prisms on faces) |
| `TKMesh` | 163 | 21,212 | **tessellation**: `BRepMesh`, `IMeshData`, `IMeshTools`, `BRepMeshData` |
| `TKExpress` | 124 | 15,133 | EXPRESS schema tooling |
| `TKPrim` | 81 | 13,728 | primitive solids + sweeps (`BRepPrimAPI`, `BRepSweep`) |
| `TKHelix` | 20 | 3,640 | helical curves |
| `TKXMesh` | 2 | 100 | stub |
* Docs: https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_algos.html and Mesh guide https://dev.opencascade.org/doc/overview/html/occt_user_guides__mesh.html

**Visualization** — 7 toolkits, 279k lines. `TKV3d` (AIS interactive objects, 112k lines), `TKService` (graphic driver abstraction, fonts, images, 71k), `TKOpenGl` (81k), `TKOpenGles`, `TKMeshVS`, `TKIVtk` (VTK bridge), `TKD3DHost`. Rendering + interactive selection only. Docs: https://dev.opencascade.org/doc/overview/html/occt_user_guides__visualization.html

**ApplicationFramework (OCAF)** — 13 toolkits, 122k lines. Document/attribute/label tree, undo-redo, transactions, persistence drivers in three formats (`TKBin*`, `TKXml*`, `TKStd*`), `TKCAF`/`TKLCAF`/`TKVCAF`/`TKCDF`, plus `TKTObj`. Docs: https://dev.opencascade.org/doc/overview/html/occt_user_guides__ocaf.html

**DataExchange** — 14 toolkits, 632k lines, **the largest module by file count** (5,292 files).
| Toolkit | Files | Lines |
|---|---:|---:|
| `TKDESTEP` | 3,537 | 343,502 |
| `TKDEIGES` | 913 | 140,279 |
| `TKXSBase` | 371 | 71,660 |
| `TKXCAF` | 121 | 26,630 |
| `TKDEVRML` | 172 | 23,649 |
| `TKDEGLTF` | 35 | 11,566 |
| `TKDESTL`, `TKDEOBJ`, `TKDEPLY`, `TKRWMesh`, `TKDE`, `TKDECascade`, `TKBinXCAF`, `TKXmlXCAF` | small | ~26k combined |

**Note for BIM context:** OCCT's *open-source* DataExchange does **not** include IFC. IFC import is sold separately as a commercial add-on component by Open Cascade SAS, alongside ACIS SAT, Parasolid, DXF and JT. (https://occt3d.com/components/ifc-import-component/, referenced from https://dev.opencascade.org/doc/overview/html/index.html)

**Draw** — 20 toolkits, 191k lines. Tcl-based test harness. Never linked into an application.

---

## 2. Concrete size numbers

### 2.1 Source lines

| Measure | Value | Provenance |
|---|---:|---|
| All `src/`, physical lines, all file types | **3,088,835** over 14,384 files | **[MEASURED]** master @7d2efad |
| `src/` code-only LOC (pygount, C++ + C) | **1,426,935** code / 501,341 comment | **[MEASURED]** |
| `src/` excluding Draw + Deprecated + GTests | **2,643,305** physical lines / 12,465 files | **[MEASURED]** |
| In-tree GTests | 219,776 lines / 531 files | **[MEASURED]** |
| Whole repo on disk (incl. `data/`, `tests/`, `dox/`) | **347 MB**; `src/` alone **139 MB** | **[MEASURED]** |
| OCCT training material: "2.5 million lines of code" | 2.5M | **[CITED]** https://dev.opencascade.org/resources/trainings |
| Open Hub analysis (of the `oce` fork enlistment) | 3,649,772 LOC | **[CITED]** https://www.openhub.net/p/opencascade — ⚠️ tracks `github.com/tpaviot/oce`, a fork, not upstream. Treat as indicative only. |

The vendor's own "2.5 million lines" figure and my measured 2.64M (excl. Draw/Deprecated/GTests) agree closely.

### 2.2 Class count

| Measure | Value |
|---|---:|
| `.hxx` headers, excluding Draw/Deprecated/GTests (upper bound proxy for public classes) | **5,753** **[MEASURED]** |
| Distinct `class X : ...` / `class X {` declarations in `.hxx` | **3,333** **[MEASURED]** |
| Classes registered in OCCT's RTTI system (`DEFINE_STANDARD_RTTIEXT`) — i.e. `Standard_Transient` descendants | **2,418** **[MEASURED]** |

That last number is the important one architecturally: **~2,400 classes in OCCT are reference-counted, RTTI-registered heap objects**, not plain values.

### 2.3 Binary size — measured from real distro packages

Debian trixie `libocct-* 7.8.1+dfsg1-3`, amd64, **stripped** release `.so` files. **[MEASURED]** (downloaded and unpacked; `file` confirms "stripped")

| Runtime package | .deb download | Installed-Size (KiB) |
|---|---:|---:|
| `libocct-modeling-algorithms-7.8` | 8.38 MB | 29,243 |
| `libocct-data-exchange-7.8` | 4.70 MB | 20,623 |
| `libocct-modeling-data-7.8` | 2.81 MB | 8,883 |
| `libocct-visualization-7.8` | 2.15 MB | 7,455 |
| `libocct-ocaf-7.8` | 1.23 MB | 5,968 |
| `libocct-foundation-7.8` | 1.48 MB | 5,103 |
| **Sum of shipped `.so` files** | **75.2 MB** across **51 libraries** | ~77,275 KiB installed |

Source: https://packages.debian.org/trixie/libocct-modeling-algorithms-7.8 (and sibling package pages).

Same figures grouped by module, measured directly from the unpacked `.so` files (sums to 75.2 MB): **[MEASURED]**

| Module package | Libraries shipped | Stripped `.so` total |
|---|---:|---:|
| modeling-algorithms | 12 | **28.5 MB** |
| data-exchange | 14 | **20.1 MB** |
| modeling-data | 4 | **8.6 MB** |
| visualization | 5 | **7.2 MB** |
| ocaf | 13 | **5.8 MB** |
| foundation | 3 | **5.0 MB** |
| **Total** | **51** | **75.2 MB** |

(Debian's `visualization` package omits `TKIVtk`/`TKOpenGles`/`TKD3DHost`; a full upstream build of Visualization is larger.)

Largest individual stripped libraries: **[MEASURED]**

| Library | Size |
|---|---:|
| `libTKDESTEP.so` | 9.7 MB |
| `libTKGeomAlgo.so` | 6.2 MB |
| `libTKGeomBase.so` | 5.6 MB |
| `libTKDEIGES.so` | 4.5 MB |
| `libTKBool.so` | 4.3 MB |
| `libTKV3d.so` | 3.7 MB |
| `libTKTopAlgo.so` | 3.4 MB |
| `libTKFillet.so` | 3.0 MB |
| `libTKShHealing.so` | 2.9 MB |
| `libTKMath.so` | 2.8 MB |
| `libTKBO.so` | 2.7 MB |
| `libTKXSBase.so` | 2.3 MB |
| `libTKernel.so` | 2.0 MB |
| `libTKMesh.so` | 1.2 MB |
| `libTKBRep.so` | 1.3 MB |

Note `libTKMesh` — the entire tessellator — is only **1.2 MB / 21k lines**, while the intersection and boolean machinery it sits on top of is an order of magnitude larger.

### 2.4 Third-party dependency surface

External deps referenced across toolkit `EXTERNLIB.cmake` files: **[MEASURED]**
TBB (11 toolkits), FreeType, RapidJSON, Draco, VTK, FreeImage, FFmpeg, OpenVR, Tcl/Tk, X11/Xmu, OpenGL/GLES, D3D9, fontconfig, plus platform libs. Most are confined to Visualization / DataExchange / Draw; the modeling core's only optional third-party dep is **TBB** (parallelism).

---

## 3. What an IFC→triangles pipeline needs vs. what OCCT ships

### 3.1 Empirical baseline: what IfcOpenShell actually links

IfcOpenShell is the reference OCCT-based IFC geometry engine. Its `cmake/FindOpenCASCADE.cmake` declares the exact required library list: **[MEASURED]** from https://github.com/IfcOpenShell/IfcOpenShell/blob/master/cmake/FindOpenCASCADE.cmake (lines 91–124)

```
TKernel TKMath TKBRep TKGeomBase TKGeomAlgo TKG3d TKG2d
TKShHealing TKTopAlgo TKMesh TKPrim TKBool TKBO TKFillet
TKXSBase TKOffset TKHLR TKBin
+ (TKDESTEP TKDEIGES on OCCT >= 7.8, else TKIGES/TKSTEP*)
```

That is **20 of the 51 shipped libraries**.

The OCCT symbols IfcOpenShell's `src/ifcgeom` actually includes, by frequency: **[MEASURED]** — `TopoDS*`, `BRep_Tool`, `TopExp*`, `BRepBuilderAPI_Make{Face,Edge,Wire,Solid}`, `Geom_{Plane,Line,Circle,Surface,Curve,BSplineSurface,BSplineCurve}`, `gp_*`, `BRepPrimAPI_Make{Prism,Revol,HalfSpace}`, `BRepOffsetAPI_Sewing`, `BRepMesh_IncrementalMesh`, `ShapeFix_{Shape,Solid,ShapeTolerance}`, `ShapeAnalysis_Surface`, `BRepCheck_Analyzer`, `BRepGProp`, `Bnd_Box`, `NCollection_List`.

Note that `BRepPrimAPI_MakeHalfSpace` + `BRepAlgoAPI` usage is exactly the IFC `IfcHalfSpaceSolid` / `IfcBooleanClippingResult` path — i.e. **booleans are genuinely load-bearing for IFC**, not optional.

### 3.2 Measured size of nested subsets

Computed by summing measured `.so` sizes and source lines over each toolkit set (link dependencies closed transitively via each toolkit's `EXTERNLIB.cmake`). **[MEASURED]**

| Subset | Libs | Binary | % of 75.2 MB | Source files | Source lines |
|---|---:|---:|---:|---:|---:|
| Everything Debian ships | 51 | 75.2 MB | 100% | 12,408 | 2,636,741 |
| IfcOpenShell's full declared list | 20 | 57.4 MB | 76.3% | 9,809 | 2,115,051 |
| …minus STEP/IGES/XSBase/Bin (no foreign-CAD file IO) | 16 | 40.7 MB | 54.1% | 4,979 | 1,563,278 |
| Minimal IFC→triangles, link-closed, **with** booleans | 13 | 34.5 MB | 45.9% | 4,438 | 1,381,611 |
| Floor: build + evaluate + tessellate, **no** booleans | 10 | 24.5 MB | 32.6% | 3,582 | 1,079,082 |

The 13-library minimal set is: `TKernel TKMath TKG2d TKG3d TKGeomBase TKBRep TKGeomAlgo TKTopAlgo TKPrim TKMesh TKShHealing TKBO TKBool`.

**Key finding: even after stripping everything an IFC pipeline provably cannot use, you still carry ~34 MB of binary and ~1.38 M lines of source — 46% of OCCT.** The reduction is real but bounded, because the geometry core is where the mass actually is.

### 3.3 NOT needed at all

| Module / toolkit | Binary saved | Why an IFC→triangles pipeline doesn't need it |
|---|---:|---|
| **Draw** (20 toolkits) | not in runtime pkgs | Tcl test harness only |
| **ApplicationFramework / OCAF** (13 toolkits, 122k lines) | ~5.8 MB | Document/label/attribute tree with undo-redo and 3 persistence formats. IFC files are their own document model; OCAF is a parallel, redundant one. IfcOpenShell links only `TKBin` from this area (and its own comment says "@todo investigate the exact conditions when this is necessary"). |
| **Visualization** (7 toolkits, 279k lines) | 7.2 MB | AIS/OpenGL/D3D/VTK rendering + interactive picking. A pipeline that emits triangles hands them to a renderer; it isn't one. |
| **`TKDESTEP` + `TKDEIGES`** | 14.2 MB | STEP AP203/214/242 and IGES readers. 4,450 files, 484k lines. Irrelevant to IFC unless you specifically want STEP interop. IfcOpenShell links them for its optional STEP/IGES export path, not for IFC reading. |
| **`TKDEVRML`, `TKDEOBJ`, `TKDEPLY`, `TKDESTL`, `TKDEGLTF`, `TKRWMesh`** | ~1.7 MB | Mesh format IO — trivial to reimplement, not a kernel concern |
| **`TKXCAF`, `TKBinXCAF`, `TKXmlXCAF`, `TKXSBase`** | ~3.9 MB | XDE/XCAF colour-layer-name framework on top of OCAF; the whole point of an IFC library is that IFC already carries that metadata |
| **`TKHLR`** (227 files, 44k lines) | 1.4 MB | Hidden line removal for 2D drawing generation |
| **`TKFillet`** (248 files, 93k lines) | 3.0 MB | Fillets/chamfers. IFC has no general fillet entity; blends arrive already baked into swept/B-Rep geometry |
| **`TKOffset`** (73 files, 47k) | 1.9 MB | Thick-solid/draft. Used by IfcOpenShell only for edge cases (`BRepOffsetAPI_MakeThickSolid`), not on the main path |
| **`TKFeat`** (94 files, 30k) | 1.3 MB | Parametric feature modelling — a CAD-authoring concept absent from IFC |
| **`TKExpress`, `TKHelix`, `TKXMesh`, `TKMeshVS`, `TKIVtk`, `TKD3DHost`, `TKTObj*`** | ~1.5 MB | Not on any IFC path |

### 3.4 Unavoidable, and why

| Toolkit | Lines | Why it can't be dropped |
|---|---:|---|
| `TKernel` | 161k | Everything links it. `Standard_Transient`, `NCollection`, `OSD`, `Message`, `Quantity`, `Precision`. Every other toolkit's `EXTERNLIB.cmake` lists it. **[MEASURED]** |
| `TKMath` | 184k | `gp` transforms, `BSplCLib`/`BSplSLib` NURBS evaluation, `math` solvers, `Poly` triangulation containers, `Bnd` boxes |
| `TKG2d` / `TKG3d` | 31k / 88k | `Geom`/`Geom2d` curve & surface classes — IFC's `IfcBSplineSurface`, `IfcTrimmedCurve`, `IfcCircle`, `IfcSurfaceOfRevolution` map directly onto these |
| `TKGeomBase` | **325k** | The heaviest non-optional piece. `Extrema` (needed by everything), `ProjLib` (pcurve construction), `GeomConvert`, `Approx`/`AppDef` (approximation), `GCPnts` (curve discretisation — used directly by meshing), `BndLib`, `GProp` |
| `TKBRep` | 131k | `TopoDS` — the B-Rep data structure itself. `IfcFacetedBrep`, `IfcAdvancedBrep`, every swept solid becomes a `TopoDS_Shape` |
| `TKTopAlgo` | 87k | `BRepBuilderAPI_Make*` (how you construct anything), `BRepCheck`, `BRepClass3d` (point-in-solid), `BRepGProp` (volumes) |
| `TKGeomAlgo` | 209k | Surface-surface intersection (`IntPatch`, `IntSurf`, `IntWalk`, `GeomInt`, `ApproxInt`). **Booleans are built on this**; it is the deepest and mathematically hardest part of the kernel |
| `TKPrim` | 14k | `BRepPrimAPI_MakePrism/MakeRevol/MakeHalfSpace` — the direct implementation of `IfcExtrudedAreaSolid`, `IfcRevolvedAreaSolid`, `IfcHalfSpaceSolid`. Small but essential |
| `TKMesh` | 21k | `BRepMesh_IncrementalMesh` — the actual B-Rep→triangles step. Notably small |
| `TKBO` + `TKBool` | 81k + 146k | `BOPAlgo`/`BRepAlgoAPI` cut/fuse/common. **Required** by `IfcBooleanResult`, `IfcBooleanClippingResult`, `IfcOpeningElement` (every door and window in every IFC file is a boolean subtraction). `TKBool` is also pulled in transitively by `TKOffset`/`TKFillet`. |
| `TKShHealing` | 89k | `ShapeFix_Shape`, `ShapeFix_Solid`, `ShapeAnalysis_Surface`. IfcOpenShell calls these routinely because real-world IFC geometry is dirty: unclosed shells, self-intersecting profiles, bad tolerances. Also a hard link-dep of `TKBO` and `TKMesh`. **[MEASURED]** from `EXTERNLIB.cmake` |

**On "meshing of NURBS":** you cannot skip it. `TKMesh`/`BRepMesh` tessellates the *analytic* surface, so it depends on the full `Geom`+`GeomAdaptor`+`BSplSLib`+`Extrema`+`GCPnts` evaluation stack, and on `TKShHealing` and `TKTopAlgo`. The tessellator itself is cheap (21k lines); its dependency cone is not.

---

## 4. Architectural criticisms — what makes OCCT heavy

Each item below is grounded in something checkable in the source or in upstream statements.

### 4.1 CDL/WOK legacy (largely, but not entirely, resolved)

Until OCCT 7.0 (2016), OCCT was not written in C++ directly. Classes were declared in **CDL** (Cascade Definition Language) and mechanically translated into C++ by **WOK** (Workshop Organisation Kit). Upstream's own 7.0 preview post: *"All classes previously defined using CDL language have been converted into pure C++ format… This reduces number of source files in OCCT source archive by about 19,000, and saves ~22 MB of disk space"* — the `drv/` generated directory went from 15,458 files to 0. **[CITED]** https://dev.opencascade.org/content/open-cascade-technology-70-preview

The refactoring also eliminated CDL "generic classes": *"~130 unused generic classes are removed… about 120 generic classes will remain in OCCT 7.0 (compare to ~450 in OCCT 6.3.1)."* **[CITED]** https://dev.opencascade.org/content/current-progress-occt-refactoring-project

The *generator* is gone; the **shape of the API it produced remains** — that's why you still see `Handle(...)` macros, `DEFINE_STANDARD_RTTIEXT`, one-class-per-file with `Package_Class` naming, and 372 flat "packages".

### 4.2 `Standard_Transient` reference counting on ~2,400 classes

`Standard_Transient` is the root of OCCT's dynamic object system. Measured from `src/FoundationClasses/TKernel/Standard/Standard_Transient.hxx`: **[MEASURED]**

```cpp
class Standard_Transient { ...
  std::atomic_int myRefCount_;     // line 140
  void IncrementRefCounter() { myRefCount_.fetch_add(1, std::memory_order_relaxed); }
  int  DecrementRefCounter() { ... fetch_sub(1, std::memory_order_release) ... }
};
```

**2,418 classes** derive from it (`DEFINE_STANDARD_RTTIEXT` count). **[MEASURED]** Consequences:
* Nearly all geometry objects (`Geom_Surface`, `Geom_Curve`, `Poly_Triangulation`, `TopoDS_TShape`, …) are **heap-allocated, individually reference-counted, atomically**. This is the dominant allocation pattern in the kernel — no value-type geometry.
* `TopoDS_TShape : public Standard_Transient` (verified at `src/ModelingData/TKBRep/TopoDS/TopoDS_TShape.hxx:62`) — the topology graph is a web of atomically refcounted heap nodes with shared sub-shapes. **[MEASURED]**
* Refcount cycles are not collected; ownership is manual discipline.
* Custom RTTI (`DEFINE_STANDARD_RTTIEXT` + `DownCast`) exists in parallel to C++'s own — historically because CDL predated reliable cross-platform `dynamic_cast`.
* Docs describe this as a feature: "safe handling of dynamically created objects, ensuring automatic deletion of unreferenced objects" and "extended run-time type information (RTTI) mechanism". **[CITED]** https://dev.opencascade.org/doc/overview/html/occt_user_guides__foundation_classes.html

### 4.3 Every toolkit depends on TKernel, and the modeling toolkits form a near-clique

Measured from `EXTERNLIB.cmake` in each toolkit: **[MEASURED]**

```
TKMath      <- TKernel, TBB
TKG3d       <- TKMath TKernel TKG2d
TKBRep      <- TKMath TKernel TKG2d TKG3d TKGeomBase
TKGeomBase  <- TKernel TKMath TKG2d TKG3d TBB
TKGeomAlgo  <- TKernel TKMath TKG3d TKG2d TKGeomBase TKBRep
TKTopAlgo   <- TKMath TKernel TKG2d TKG3d TKGeomBase TKBRep TKGeomAlgo TBB
TKMesh      <- TKernel TKMath TKBRep TKTopAlgo TKShHealing TKGeomBase TKG3d TKG2d
TKBO        <- TKBRep TKTopAlgo TKMath TKernel TKG2d TKG3d TKGeomAlgo TKGeomBase TKPrim TKShHealing TBB
TKBool      <- TKBRep TKTopAlgo TKMath TKernel TKPrim TKG2d TKG3d TKShHealing TKGeomBase TKGeomAlgo TKBO
TKShHealing <- TKBRep TKernel TKMath TKG2d TKTopAlgo TKG3d TKGeomBase TKGeomAlgo
TKOffset    <- TKFillet TKBRep TKTopAlgo TKMath TKernel TKGeomBase TKG2d TKG3d TKGeomAlgo TKShHealing TKBO TKPrim TKBool
```

Observations:
* **`TKGeomBase` depends on `TKG3d` and `TKG3d`'s consumers depend back on `TKGeomBase` via `TKBRep`** — the modeling-data layer is not a clean DAG of small pieces.
* **`TKMesh` (just a tessellator) transitively requires 8 toolkits including shape healing.** You cannot take "the mesher" alone.
* **`TKBO` (booleans) requires 10 toolkits.** Booleans sit at the top of nearly the whole modeling stack.
* Symptom of this coupling in practice: IfcOpenShell's CMake had to wrap OCCT libs in `$<LINK_GROUP:RESCAN,...>` because *"Before 7.9.0 targets in OCCT cmake configs are not linked to each other leading to missing symbols on Unix"* — i.e. the dependency graph has enough cycles that a single-pass linker fails. **[MEASURED]** from `cmake/FindOpenCASCADE.cmake:51–55`, https://github.com/IfcOpenShell/IfcOpenShell/blob/master/cmake/FindOpenCASCADE.cmake

### 4.4 Data and algorithms are coupled through the topology data structure

`TopoDS_Shape` is not a passive value. It carries shared `TShape` nodes, per-sub-shape tolerances (`BRep_Tool::Tolerance`), locations (`TopLoc`), and cached representations (3D curve + per-face pcurves + `Poly_Triangulation`). Algorithms both read and *write* into it (healing mutates tolerances; meshing attaches triangulations to faces). Measured: `BRep_Tool.cxx` alone is 1,862 lines and `BRepTools.cxx` 1,506 lines of accessor/mutator logic over this structure. **[MEASURED]**

The practical effect is that "modeling data" is not separable from "modeling algorithms" — which is precisely why the minimal subset in §3.2 still comes to 1.38M lines.

### 4.5 The mathematical core is genuinely large, not just bloated

This is the honest counterweight to the "OCCT is legacy bloat" narrative. Of the 1.38M-line minimal subset: **[MEASURED]**
* `TKGeomBase` 325k + `TKGeomAlgo` 209k = **534k lines of approximation, extrema, projection and surface-surface intersection**.
* `TKBO`+`TKBool` = **228k lines of boolean topology**, of which `BOPAlgo` is 34k and `IntTools` 18k.

These are irreducibly hard problems (robust SSI on trimmed NURBS, tolerant boolean topology). A large fraction of OCCT's mass is problem difficulty, not architectural waste.

### 4.6 Other frequently-cited weight sources

* **Exception-based control flow.** 1,176 files reference `Standard_Failure`/`throw`. **[MEASURED]** OCCT exceptions can be compiled as either C++ exceptions or signal-based (`OSD::SetSignal`), a portability legacy that complicates embedding.
* **Per-class translation units.** 12,465 files for the non-Draw, non-test source. Build times and template instantiation across `NCollection` are a well-known cost; upstream added precompiled headers (`TKernel_pch.hxx`, `TKBRep_pch.hxx`, `TKBool_pch.hxx`, `TKMesh_pch.hxx`) specifically to mitigate this. **[MEASURED]**
* **Custom collections and strings.** `TCollection`/`TColStd`/`NCollection` duplicate STL functionality. OCCT 8.0 upgrade notes now advise: *"New code should prefer the explicit NCollection_*<T> form"* over the legacy `TColStd` instantiations — an in-progress migration. **[CITED]** https://dev.opencascade.org/doc/overview/html/occt_user_guides__foundation_classes.html
* **Custom memory allocator.** `Standard_MMgrOpt`/`CSF_MMGR`, an OCCT-specific allocator layered under the refcounted objects.
* **TBB coupling.** 11 toolkits list `CSF_TBB`, including `TKMath`, `TKGeomBase`, `TKTopAlgo` and `TKBO`. **[MEASURED]**
* **Licensing friction.** LGPL-2.1 **with an additional exception** (`OCCT_LGPL_EXCEPTION.txt` in the repo root), which is its own analysis burden for downstream projects. https://dev.opencascade.org/doc/overview/html/occt_public_license.html

---

## 5. Summary table

| Question | Measured answer |
|---|---|
| OCCT modules | 7 (`src/MODULES.cmake`), + a `Deprecated` stub dir |
| Toolkits (libraries) | 74 in source; 51 `.so` shipped by Debian 7.8.1 |
| Leaf packages | 372 (excl. Draw/Deprecated/GTests) |
| Total source | 3.09 M physical lines / 14,384 files; 1.43 M code-only LOC; 139 MB |
| Vendor's stated size | "2.5 million lines of code" |
| Refcounted (`Standard_Transient`) classes | 2,418 |
| Full binary set | 75.2 MB stripped, 51 libraries |
| What IfcOpenShell links | 20 libraries, 57.4 MB (76%) |
| Minimal IFC→triangles closure | 13 libraries, 34.5 MB (46%), 1.38 M source lines |
| Provably droppable | OCAF (5.8 MB), Visualization (7.2 MB), STEP+IGES (14.2 MB), HLR, Fillet, Feat, Offset, mesh-format IO, Draw |
| Irreducible core | TKernel, TKMath, TKG2d/G3d, TKGeomBase, TKBRep, TKGeomAlgo, TKTopAlgo, TKPrim, TKMesh, TKShHealing, TKBO, TKBool |
| Where the mass actually is | `TKGeomBase` (325k) + `TKGeomAlgo` (209k) + `TKBool`/`TKBO` (228k) = 55% of the minimal subset |

---

## 6. Sources

**Primary (upstream):**
* OCCT repository — https://github.com/Open-Cascade-SAS/OCCT (measured at commit `7d2efad9c8a9a57ea96c4c8587134b34dd503cd8`, version 8.1.0-dev)
* Module list — https://github.com/Open-Cascade-SAS/OCCT/blob/master/src/MODULES.cmake
* Overview / module descriptions — https://dev.opencascade.org/doc/overview/html/index.html
* Foundation Classes guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__foundation_classes.html
* Modeling Data guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_data.html
* Modeling Algorithms guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__modeling_algos.html
* Mesh guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__mesh.html
* Shape Healing guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__shape_healing.html
* Visualization guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__visualization.html
* OCAF guide — https://dev.opencascade.org/doc/overview/html/occt_user_guides__ocaf.html
* "2.5 million lines of code" — https://dev.opencascade.org/resources/trainings
* CDL/WOK removal in 7.0, file/disk savings table — https://dev.opencascade.org/content/open-cascade-technology-70-preview
* Generic-class elimination (450→120) — https://dev.opencascade.org/content/current-progress-occt-refactoring-project
* Commercial IFC import component (not in open-source OCCT) — https://occt3d.com/components/ifc-import-component/
* License — https://dev.opencascade.org/doc/overview/html/occt_public_license.html

**Binary sizes:**
* https://packages.debian.org/trixie/libocct-foundation-7.8
* https://packages.debian.org/trixie/libocct-modeling-data-7.8
* https://packages.debian.org/trixie/libocct-modeling-algorithms-7.8
* https://packages.debian.org/trixie/libocct-visualization-7.8
* https://packages.debian.org/trixie/libocct-data-exchange-7.8
* https://packages.debian.org/trixie/libocct-ocaf-7.8

**IFC-consumer evidence:**
* IfcOpenShell — https://github.com/IfcOpenShell/IfcOpenShell
* Required OCCT library list — https://github.com/IfcOpenShell/IfcOpenShell/blob/master/cmake/FindOpenCASCADE.cmake

**Third-party (use with caution):**
* Open Hub OCCT analysis (3,649,772 LOC) — https://www.openhub.net/p/opencascade — ⚠️ tracks the `tpaviot/oce` fork, not upstream.

---

## 7. Reproducing these measurements

```bash
git clone --depth 1 https://github.com/Open-Cascade-SAS/OCCT.git occt
cd occt
# module line counts (GTests excluded)
find src/ModelingAlgorithms -path '*/GTests' -prune -o -type f \
  \( -name '*.cxx' -o -name '*.hxx' -o -name '*.h' -o -name '*.c' \
     -o -name '*.lxx' -o -name '*.pxx' \) -print | xargs cat | wc -l
# toolkit list
find src -maxdepth 2 -type d -name 'TK*' | sort
# refcounted class count
grep -rc 'DEFINE_STANDARD_RTTIEXT' --include='*.hxx' src | awk -F: '{s+=$2} END {print s}'
# code-only LOC
pygount --format=summary --suffix=cxx,hxx,h,c,lxx,pxx src

# binary sizes (Debian/Ubuntu)
apt-get download libocct-foundation-7.8 libocct-modeling-data-7.8 \
  libocct-modeling-algorithms-7.8 libocct-visualization-7.8 \
  libocct-data-exchange-7.8 libocct-ocaf-7.8
for f in *.deb; do dpkg-deb -x $f ex; done
find ex -name '*.so.*' -type f -printf '%s %f\n' | sort -rn
```

Helper scripts used for this report are committed at
`scripts/research/occt_measure.sh` and `scripts/research/occt_measure_modules.py`.

### 7.1 Independent re-verification (2026-08-19)

The headline numbers were re-measured from a second, independent checkout
by a different agent. Results agree.

| Claim | Report | Re-measured | Delta |
|---|---:|---:|---:|
| 13-toolkit minimal subset | 1,381,611 | 1,355,006 | 1.9% |
| Total source (excl Draw/Deprecated) | 2,636,741 | 2,566,219 | 2.7% |
| `TKGeomBase` | 325k | 333,224 | ok |
| `TKGeomAlgo` | 209k | 225,236 | ok |
| `TKBO` + `TKBool` | 228k | 227,769 | ok |

Deltas come from the file-extension set. **`.gxx` and `.pxx` must be
included** — OCCT keeps template bodies in them, and `TKGeomBase` drops from
333k to 175k if they are omitted, which would understate the NURBS cost by
half. A first pass here made exactly that mistake.

Minimal subset as a share of source is **~53%** (1.36M / 2.57M). The 46%
figure in section 3.2 is the *binary* share (34.5 MB / 75.2 MB); the two are
not interchangeable.

```bash
# the 13-toolkit floor, re-derived
cd occt/src
for tk in TKernel TKMath TKG2d TKG3d TKGeomBase TKBRep TKGeomAlgo \
          TKTopAlgo TKPrim TKMesh TKShHealing TKBO TKBool; do
  find . -maxdepth 2 -type d -name "$tk"
done | xargs -I{} find {} -type f \
  \( -name '*.cxx' -o -name '*.hxx' -o -name '*.lxx' \
     -o -name '*.gxx' -o -name '*.pxx' \) | sort -u | xargs cat | wc -l
```
