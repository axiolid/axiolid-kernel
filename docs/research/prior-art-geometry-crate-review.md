# Review: a prior-art rule-checking geometry crate

Read 2026-08-19 from a local prior-art workspace, commit `9aa004b`. Every claim
was checked against source or by running the crate's own tests. Figures marked
MEASURED were produced on this machine.

## What it is

MEASURED: 6,815 lines across 51 files, one crate, no internal dependencies --
it is the root of the workspace graph. Deps: `glam`, `smallvec`, `thiserror`,
`rayon`, `i_overlay`, `earcutr`, and optionally `manifold3d` (C++).

This is real working code, not scaffold. 99 tests pass with default features.

## What it does well

### 1. Split by stability and role, not by primitive type

Module set: `scalar`, `predicates`, `primitives`, `profile`, `mesh`, `solid`,
`spatial`, `query`, `diagnostics`. The stated rationale is that `primitives`
changes almost never while `query` changes constantly, so keeping them apart
means a new rule never recompiles the vector math.

This is a better axis than the obvious one (a module per shape kind). It is
the same insight behind our L0/L1/L2 tiering, arrived at independently.

### 2. Invariants written down with the scar that produced them

The crate doc lists six invariants, each citing the specific failure it
prevents:

- No rendering types -- the prior product's entity base class had
  `getColorBuffer()` and `getPresentationType()` on the base class of all
  geometry.
- No serialization derives -- 34 of 66 of its geometry classes carried a
  `serialVersionUID`, which made the kernel unrefactorable without breaking
  every saved file.
- Tolerance is a parameter -- cites IfcOpenShell's `AbstractKernel.h` opening
  with a file-scope `static const double ALMOST_ZERO = 1.e-9;`, which is
  wrong for either a millimetre or a metre model.
- Exact predicates decide, f64 constructs.
- Dirty geometry is a state, not an error -- with the measurement: ~45% of
  real meshes are not watertight (windows 15% closed, doors 13%).
- One clash kernel for both IFC and `.smc`.

A rule with its originating incident attached survives review pressure. A
bare "don't do X" does not.

### 3. The subtractor trait -- a genuinely good seam

`MeshSubtractor::subtract(subject, tools) -> TriMesh` with three
implementations: `PlanarFootprintSubtractor` (2D coplanar cut via
`i_overlay`), `FacePlaneClipSubtractor`, and `Csg3dSubtractor` (3D CSG,
feature-gated behind the C++ dependency).

The fast, dependency-free path handles the common real case (flat paving on
terrain); the heavy path is opt-in and behind the same trait. That is
precisely the structure we want for Axiolid's operation-specific contracts and
replaceable providers, and it is validated here by three independent
implementations rather than one plus an aspiration.

### 4. Validation by invariant, not by golden output

The planar cut is checked by triangulation-invariant quantities: cut top-face
area equals `original_top_area - union(tool_footprints AND top)`, and no cut
triangle's centroid lands inside any tool footprint. Not an index-buffer
snapshot. This survives a legitimate change in triangulation strategy, which
a golden mesh would not.

### 5. Cross-process determinism testing

`tests/planar_subtract_determinism.rs` re-runs the subtraction in a
subprocess loop. The reason is stated exactly: the subtractor groups faces
into a `HashMap<i64, ...>`, Rust's default hasher is seeded randomly per
process, so vertex/triangle order varied across runs. A single in-process run
cannot catch this because the seed is fixed for the life of the process.

That is a bug class most projects never test for, and the test is designed
around the actual mechanism rather than hoping repetition finds it.

### 6. Oracle testing against IfcOpenShell

`scripts/gen_clash_oracle.py` drives ifcopenshell's own collision engine
(`axiolid.tree`, `clash_intersection_many`) and emits JSONL keyed by GlobalId,
sorted so the set compares regardless of pair order. Same criterion the Rust
narrow phase implements, so the two must agree. Similar generators exist for
AABB, mesh, and placement oracles.

Differential testing against the reference implementation is the strongest
correctness evidence available for a geometry kernel, and it is cheap here
because the oracle is a Python script rather than a linked dependency.

## What it does not do well

### 1. The module doc contradicts the code

MEASURED. The crate doc says, verbatim, there is "**no mesh validator**
(`mesh::validate`), **no acceleration structure** (`spatial` -- the broad
phase is all-pairs AABB in `query::clash`), and **no 2D boolean**
(`profile::boolean2d`)."

Checked all three:

| Claim in doc | Reality |
|---|---|
| no mesh validator | `src/mesh/validate.rs` is 72 lines and `validate_mesh` is re-exported at the crate root |
| broad phase is all-pairs | `query/clash.rs` builds `Bvh<usize>`; `spatial/bvh.rs` is 350 lines with a real BVH |
| no 2D boolean | correct -- `profile/boolean2d.rs` is a stub, but the 2D difference is done inline in `solid/subtract.rs` via `i_overlay` |

Two of three warnings are stale. The doc was accurate when written and the
code outgrew it. This is worse than no doc: a reader trusts the warning,
writes an all-pairs workaround, and duplicates a BVH that already exists.

Lesson for axiolid: our own AGENTS.md and module docs make the same kind of
status claims ("Scaffold", "Not yet implemented"). Those need to be
mechanically checked or they will drift identically. A doc assertion that
code contradicts is a defect, not a stylistic issue.

### 2. `--no-default-features` is broken

MEASURED:

```
cargo test -p geometry                      99 passed, 0 failed
cargo test -p geometry --no-default-features  55 passed, 2 FAILED
```

Both failures are `called Option::unwrap() on a None value` in
`query/solid_intersection.rs`.

Root cause: `solid_intersection_metrics` has two definitions --
`#[cfg(feature = "csg3d")]` returns real metrics,
`#[cfg(not(feature = "csg3d"))]` returns `None` unconditionally. The
`#[cfg(test)]` module is not feature-gated, so without `csg3d` the tests call
the stub and unwrap `None`.

This matters more than a normal broken test. The `csg3d` feature exists
specifically so consumers can drop the C++ toolchain -- the manifest says
`manifold3d` accounts for ~256 MB of a 422 MB debug build directory that
every dependent crate inherited. The escape hatch from the C++ dependency is
the one configuration that is not tested, so it rotted.

Directly relevant to axiolid: our premise is no C++ in the graph. This is
what happens when the pure path is nominally supported but not gated. Our
`gate.sh` runs per-feature-combo builds for exactly this reason, and this is
the evidence that it earns its runtime.

### 3. The stated epsilon policy is not implemented

`predicates/orientation.rs` declares itself "the ONE epsilon policy for the
crate" -- Shewchuk exact predicates, `orient2d`/`orient3d`/`incircle` -- and
then ends with "Not yet ported." It is 11 lines.

MEASURED: 60 hardcoded epsilon sites across the crate, including
`1e-9` literals in `query/containment.rs` and `query/footprint.rs`. So the
scattered `< 1e-9` comparisons the doc warns against are the actual
implementation, and `f64::EPSILON` is used in `footprint.rs`,
`slab_contact.rs`, and `distance.rs`.

The invariant is right and the code has not caught up. Worth noting that
`scalar.rs` correctly criticises IfcOpenShell's global `ALMOST_ZERO` while
the crate carries 60 local equivalents -- a global constant is at least
greppable and changeable in one place.

### 4. Half the crate is placeholder

MEASURED: 24 of 51 files contain "Not yet ported."; the doc admits "26 of its
47 files" (itself now stale by file count).

The crate is explicit about this and argues the case: reserving the shape
records where work belongs so a capability is scheduled rather than silently
dropped, and a documented placeholder beats an empty file that renders as a
real module with nothing in it.

I partly agree -- our own scaffold does the same thing deliberately. But the
cost is visible here: `primitives/vec.rs`, `matrix.rs`, `plane.rs`, `ray.rs`,
`segment.rs`, `triangle.rs` are all stubs while the real math lives in
`glam` re-exports and inline code elsewhere. A reader looking for the plane
type finds an empty `plane.rs` and has to discover it is really in
`glam`/`footprint.rs`. Placeholders help when they mark future work; they
mislead when the behaviour already exists somewhere else.

### 5. Enormous flat re-export surface

The crate root re-exports ~45 names across 12 `pub use` groups, including
things like `expanded_short_axis_footprint_intersects` and
`uncovered_footprint_segment_lengths`. The stated reason is backward
compatibility: `sol-geometry` exposed these at the root and every caller
imports them from there.

The result is that the crate's public API is both the module tree and a flat
alias of it, so there are two ways to import everything and no enforced
canonical path. Migration shims are legitimate, but this one has no
deprecation and no end date.

### 6. Query names encode domain semantics

`query/` contains `native_external_wall_footprint_indices`,
`horizontal_cap_contact`, `dominant_plane_contact`, `slab_contact`,
`portal`, `RouteBlockageReason`, `MobilityProfile`.

The crate's own first rule is that it "does not know IFC, the host product,
rules, or pixels" and that a type mentioning a schema or product entity name
belongs elsewhere.
These names honour the letter (no `Ifc` prefix) while carrying building
semantics: an external wall, a slab, a portal, and route mobility are BIM
concepts, not geometry ones. A CAD or GIS consumer would find
`native_external_wall_footprint_indices` meaningless.

The generic core is real -- `Bvh<T>` is genuinely payload-agnostic. But the
boundary leaks at the query layer, which is exactly where our layering gate
would fire if these lived in `crates/`.

### 7. No architecture gate

MEASURED: `crates/geometry/tests/` holds 7 integration tests, all behavioural
(bvh candidates, clash, distance, free space, mesh diagnostics, determinism,
portal). None check the crate boundary. Other crates in the workspace do have
architecture tests (`checker/tests/architecture_doc.rs`,
`codec/tests/architecture_structure_coverage_typed_decode.rs`), so the
practice exists -- just not for geometry.

The "no IFC knowledge" rule is therefore enforced by code review only, which
is consistent with finding domain vocabulary at the query layer and stale
capability claims in the doc.

## What axiolid should take

1. **Adopt the subtractor pattern at an operation-specific contract/provider
   boundary.** One trait, a cheap dependency-free default, heavy providers
   feature-gated. Validated here by three implementations.
2. **Adopt oracle testing against IfcOpenShell** via Python scripts. No
   linked dependency, strongest available correctness evidence.
3. **Adopt cross-process determinism tests** wherever a `HashMap` orders
   output.
4. **Adopt invariant-based validation** (areas, centroids) over golden
   meshes.
5. **Keep our layering gate.** Items 1, 2, 6 and 7 of the criticisms above
   are all things a mechanical boundary check would have caught or prevented.
6. **Add a doc-drift check.** The single most useful thing this review
   found is that prose capability claims rot silently. If a module doc says
   "not implemented", something should verify that.
7. **Test the pure-Rust path explicitly.** `--no-default-features` is the
   configuration that protects the project premise, and it is the one that
   broke here.
