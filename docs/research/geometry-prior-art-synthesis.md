# Geometry prior-art synthesis

Date: 2026-08-19

This records architectural evidence used by ADR 0009 and
`crates/PLAN.md`. It is not a claim that the scaffold implements the
algorithms named here.

## Reviewed revisions

| System | Revision | Local source |
| --- | --- | --- |
| IfcOpenShell | `1a6336bd207c` | `/mnt/backup/references/ifcopenshell` |
| IFC-Lite | `35594eeb99bd` | `/mnt/backup/references/ifc-lite` |
| That Open web-ifc | `38281501ca6c` | `/mnt/backup/references/thatopen-web-ifc` |
| That Open Fragments | `c4d48d7a5a11` | `/mnt/backup/references/thatopen-fragments` |
| Solibri extraction | `9aa004b94bd6` | `../vendor/solibri/crates/geometry` |

## Decisions, not imitation

| Evidence | Adopt | Avoid |
| --- | --- | --- |
| IfcOpenShell | IFC interpretation before neutral geometry; broad shape taxonomy; reference-oracle corpus; batch openings | OpenCascade/C++ as a mandatory dependency; format semantics in the kernel; implicit global tolerance |
| IFC-Lite | pure-Rust exact predicates; mapped-item instancing; content-based deduplication; RTC rebasing; tiered CSG fallback; invariant/property/differential tests | one IFC-coupled geometry crate; unconditional Rayon/serialization; unbounded caches; treating every documented fallback as implemented |
| That Open | reusable geometry plus placed instance split; lazy derived buffers; batched worker messages; compact transferable buffers; explicit disposal | parser, IFC IDs, colors, Three.js, and geometry construction in one layer; early mesh-only collapse; global epsilon/settings; one god backend |
| Solibri | one operation trait with multiple providers; common 2D subtract path before expensive 3D CSG; invariant validation; cross-process determinism tests | BIM query vocabulary in generic geometry; optional C++ path as default; documentation that is not checked against source/features |

## Source observations that shaped the scaffold

### IFC-Lite

- `rust/geometry/src/lib.rs` exposes one very broad geometry crate and imports
  IFC parser/schema types directly. It proves pure-Rust breadth is possible, but
  is not the dependency boundary Axiolid wants.
- `router/caching.rs`, `content_hash.rs`, and `instancing.rs` show why caches and
  instancing matter for repeated mapped geometry. Axiolid keeps identity in an
  immutable DAG and will put derived caches outside the values, with byte
  budgets and option/provider-sensitive keys.
- `router/rtc_offset.rs` treats large-coordinate rebasing as a pipeline concern.
  Axiolid preserves f64 model coordinates and leaves f32/GPU rebasing to an
  explicit operation or binding boundary.
- `docs/architecture/geometry-pipeline.md` documents tiered boolean fallback,
  batch union of cutters, clamping unbounded tools, and strict final mesh
  validation. Those are implementation waves, not hidden behavior in lowering.

### That Open

- web-ifc's `IfcGeometryProcessor.h` combines schema dispatch, geometry caches,
  construction, and output instances. Its geometry/instance split is useful;
  the combined ownership is not.
- `representation/geometry.h` and `IfcGeometry.h` mix mesh values with colors,
  Express IDs, transforms, and active mode flags. Axiolid keeps rendering and
  source IDs outside generic representations and uses enums/typed nodes instead
  of several mutually exclusive booleans.
- `operations/bim-geometry/epsilons.h` contains multiple file-scope epsilons.
  Axiolid requires explicit `Tolerance` in every sensitive operation.
- Fragments' `item-geometry.ts` and multithreading layer demonstrate compact
  typed buffers, buffer transfer, lazy derivation, instancing, and worker
  isolation. These belong at bindings/runtime boundaries; they do not justify
  storing the canonical model as f32 render buffers.

### Solibri extraction

The detailed review is in `docs/research/solibri-geometry-review.md`. The most
transferable seam is `MeshSubtractor`: consumers depend on one operation while
portable, 2D-specialized, and heavy 3D providers remain replaceable. Axiolid
uses narrow Rust traits and executable per-operation registries rather than a
catalog of boolean capability claims.

### IfcOpenShell

IfcOpenShell remains the compatibility oracle and coverage benchmark. Its
pipeline validates the need for an IFC-specific interpretation layer before the
kernel, but its OpenCascade dependency graph is exactly what Axiolid must not
inherit. Python oracle generation stays a test tool, never a linked dependency.

## Resulting Axiolid rules

1. One exact, format-neutral immutable DAG; source adapters retain their own
   enums only until explicit conversion.
2. Operation traits are capability truth. Metadata never claims an operation.
3. Execution contexts (CPU pool/ISA), operation providers, and representations
   remain separate concerns.
4. Core-only and default facade builds stay measured and small; advanced,
   parallel, SIMD, and GPU features are additive and isolated.
5. Portable scalar behavior must exist before an optimized provider is exposed.
6. CPU ISA selection is runtime. AArch64 builds must never inherit x86 flags.
7. GPU seams are API-neutral, operation-specific, batch-oriented, and precise
   about f32/f64 requirements. No concrete GPU algorithm is claimed yet.
8. Derived caches are external, byte-budgeted, observable, and keyed by content,
   tolerance/options, and provider identity.
9. Architecture and schema-coverage tests must be mutation-verified.
10. Performance claims require benchmarks; this scaffold makes no such claim.
