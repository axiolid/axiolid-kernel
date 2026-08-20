# 0003 — Pure-Rust mesh boolean instead of OpenCascade

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** —

## Context

This is the decision the project's viability rests on. IfcOpenShell needs
OpenCascade largely because IFC requires robust constructive solid geometry:

- `IfcRelVoidsElement` — cut door/window openings out of walls (ubiquitous)
- `IfcBooleanClippingResult` — clip solids with half-spaces
- `IfcBooleanResult` — general union/difference/intersection trees

If we cannot do robust CSG without a heavy C++ kernel, "lightweight
IfcOpenShell alternative" is not achievable and the premise fails.

Evidence gathered before deciding:

- The sibling `solibri-rs` workspace uses `manifold3d` (C++ Manifold) for its
  3D boolean, measured at ~256 MB of its debug build dir and requiring a C++
  toolchain — inherited by every dependent crate. It feature-gates it for
  exactly this reason.
- Two pure-Rust robust mesh booleans exist: `boolmesh` (MPL-2.0, from-scratch
  Manifold-inspired, hard dependency `glam` only, optional `rayon`) and
  `manifold-rust` (a port targeting numerical parity with Manifold v3.5.0).

So a robust boolean in pure Rust is **demonstrated, not hypothetical**.

## Decision

Mesh boolean is expressed as the `axiolid_kernel::MeshBoolean` trait. No C++
geometry kernel enters the dependency graph.

Behind that trait we may either implement our own or adopt a pure-Rust crate;
the IFC layer cannot tell the difference, so this choice can be deferred and
revisited on evidence without an API break.

The contract is explicit: **manifold input yields manifold output**, and an
implementation that cannot uphold it returns `GeomError::NotManifold` rather
than silently emitting a broken mesh. A corrupt solid propagates into every
downstream area, volume, and clash result — an explicit failure is far cheaper.

The trait includes `batch_difference` because IFC's dominant pattern is one wall
minus many openings; handing the backend all tools at once lets it parallelize,
where a caller-side loop would serialize and pay dispatch per opening.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Bind OpenCascade | Exactly the weight we exist to remove. |
| Bind C++ Manifold (`manifold3d`) | Much lighter than OCCT and genuinely robust, but still a C++ toolchain requirement for every consumer. Kept as a fallback if pure Rust proves insufficient. |
| Voxel/SDF approximation | Cheap and robust, but lossy: wrong volumes and quantities, unacceptable for BIM takeoffs. |
| Only 2D coplanar cuts | What `solibri-rs` shipped first; insufficient for general IFC (angled cuts, solids dipping into terrain). |

## Consequences

**Positive**

- Pure-Rust dependency graph: `cargo build` with no C++ toolchain.
- The heaviest algorithm is isolated behind one trait, so it can be swapped or
  upgraded without touching the IFC layer.

**Negative / costs**

- A robust boolean is genuinely hard. If our own implementation stalls, the
  schedule depends on adopting an external crate — an accepted, identified risk.
- Pure-Rust implementations are younger than OCCT and less battle-tested; the
  fixture corpus in `test/fixtures/` exists to pressure them on real edge cases
  (`bath_csg_solid`, `issue_1155_halfspace_flyaway`,
  `issue_2019_wall_two_overlapping_openings`).

**Follow-ups / risks to watch**

- **RESOLVED by [0014](0014-adopt-boolmesh-mesh-boolean.md):** `boolmesh`
  0.1.9 passed both hard fixtures (exact volume conservation, cross-checked
  against Monte-Carlo; no halfspace flyaway) and is adopted as a dependency.
  `manifold-rust` is Apache-2.0 (not MPL as implied above) and remains the
  fallback; it is not yet fixture-tested.
- Licensing: `boolmesh` is MPL-2.0 (file-level copyleft). **Checked in 0014:**
  depending imposes no obligation on our MIT code; vendoring-and-patching does.
  It is therefore an unmodified dependency, never vendored.

## Relation to existing code

- `axiolid/kernel/src/boolean.rs` — the trait, contract, and this rationale.
- `axiolid/cpu/src/lib.rs` — currently returns `Unsupported` and reports
  `mesh_boolean: false`; it fails honestly rather than returning a wrong mesh.
- `ifc/shape/src/lib.rs` — `apply_openings`, the `IfcRelVoidsElement` call site.
