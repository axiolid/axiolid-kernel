# Consumer-driven capability audit: rule-checking archetype

A rule-checking application is one of the archetypes this kernel is meant to
support. This audit compares the base-geometry surface such an application
needs against what the kernel currently provides, and records where each
capability belongs.

The kernel does not name or model any specific end-user product. Capabilities
below are stated as neutral geometry.

## Method

A downstream consumer geometry layer of 16,084 non-comment lines and 378
public items was inventoried item by item, then classified against this
kernel's 603 public symbols and the rows in `capabilities.md`.

Every item was placed in exactly one of three owning layers:

- **Kernel** — neutral geometry with no domain semantics. Owned here.
- **Format layer** — lowering from a source schema into neutral geometry.
  Owned upstream in the format library, not here.
- **Application** — building semantics, regulatory thresholds, verdicts.
  Owned by the consuming product.

Declaration scans were used only for navigation. Classification came from
reading bodies, because module names are unreliable: 24 files in the consumer
layer are doc-only stubs whose names promise algorithms they do not contain.

## Gaps — kernel-owned, now tracked

| Capability | Evidence of need | Kernel status | Owner | Issue | Milestone |
| --- | --- | --- | --- | --- | --- |
| Ray/mesh narrow phase | Consumer implements nearest-hit ray casting over triangle meshes | Broad phase only: `SpatialIndex::visit_ray`, `ray_aabb_entry` | Kernel | #41 | v0.3 |
| Planar offset | Polygon inset/outset and polyline stroke offset used for clearance envelopes | Absent; backend already vendored via `axiolid-overlay` | Kernel | #42 | v0.4 |
| Mesh/mesh distance with witnesses | Separation distance plus witness points and proximity components | `closest_points_on_triangles` exists; mesh-level composition does not | Kernel | #43 | v0.4 |
| Planar projection of meshes | Projected triangle union and vertical prism intersection | 3D and 2D halves exist; the projection between them does not | Kernel | #44 | v0.4 |
| Exact planar shortest path | Visibility-graph routing with typed unreachable reasons | Sampled routing exists in `axiolid-field`; exact planar counterpart does not | Kernel | #46 | v0.4 |
| Planar region algebra | Persistent region with set ops, erosion/dilation, components, area | `overlay()` is stateless; no region type, no component count | Kernel | #47 | v0.4 |

### Milestone decision

No new milestone. Every gap completes a seam the kernel already half-owns:
ray broad phase without narrow phase, an overlay backend whose offset entry
points are unused, triangle-level proximity without mesh-level composition,
sampled routing without its exact counterpart. Grouping them into a separate
milestone would split one coherent "finish what we started" story across two
release buckets.

#41 lands in v0.3 because it is an intersection capability. The rest land in
v0.4, whose subject is trustworthy discrete geometry.

## Already supported — no action

| Capability | Kernel provision |
| --- | --- |
| Mesh surface/volume measures | `axiolid-measure::{surface_properties, volume_properties}` |
| Point-in-mesh containment | `axiolid-measure::WindingMesh` |
| Triangle/triangle intersection | `axiolid-reference::triangle_triangle` |
| Broad-phase pair finding | `axiolid-spatial::Bvh` overlap and distance pairs |
| Mesh health and validation | `axiolid-mesh::audit_mesh`, `MeshHealth` |
| Convex hull, min-area rectangle | `axiolid-reference::convex_hull` |
| Polygon triangulation with holes | `axiolid-construct::profile` |
| 2D boolean overlay | `axiolid-overlay::overlay` |
| 3D mesh boolean | `axiolid-boolmesh` provider |
| Sampled 2.5D traversal | `axiolid-field::navigate` behind `navigation` |

## Not kernel-owned — deliberately refused

Roughly 5,600 lines of the consumer layer encode building semantics: stair
flights, landings, going and rise ratios, handrail extensions, ramp gradients,
head clearance, tactile warning surfaces, vertical access classification.

These stay in the application. The kernel must not learn what a stair is.

They are not a burden on the kernel: they already call neutral primitives
(minimum-area rectangle, convex hull, projected triangle shapes) and it is
exactly those primitives that #41-#47 generalise. Improving the substrate
serves them without importing their vocabulary.

Also refused:

- **Source-schema lowering** — placement math, profile interpretation, unit
  policy, entity identity, and schema-mandated void cuts. Owned by the format
  layer.

  This was checked rather than assumed. The audited consumer geometry layer
  contains 45 source-schema mentions and **zero** schema types in code — all
  45 are comments explaining motivation, so that layer is already neutral. The
  lowering itself lives one crate above it (~4,300 lines) and consumes only
  `Mat4`, `Vec3` and a triangle mesh from the geometry layer below.

  That upper layer hand-rolls swept-solid construction the kernel already
  provides — `extrude`, `revolve`, `fixed_reference_sweep`, `swept_disk`,
  `surface_curve_sweep`. That is duplication for the format layer to retire by
  adopting the kernel; it is not a kernel gap and no issue is filed for it.
- **Verdicts** — the kernel may report "no route exists under this envelope";
  it may never report "non-compliant". `axiolid-field::navigate` already
  states this rule in its module docs and it is upheld here.
- **Native C++ boolean backends** — the consumer uses one. The kernel ships a
  pure-Rust provider; the consumer should migrate to it rather than the kernel
  adopting a C++ toolchain dependency.

## Adoption blocker

Capability gaps are not the binding constraint. A consumer pinning this kernel
by git revision from before the crate reorganisation cannot upgrade at all:
crates were renamed and moved with no recorded mapping, so the upgrade fails
to resolve rather than failing to compile.

Until that is fixed, no consumer can measure which of #41-#47 already landed.
Tracked as #45, and it is the precondition for the rest of this audit being
actionable.

## Ownership summary

The full audited surface, by owning layer:

| Owner | Surface | Disposition |
| --- | --- | --- |
| Kernel | 6 missing capabilities | Tracked as #41-#44, #46, #47 |
| Kernel | 10 capabilities already provided | No action |
| Format layer | ~4,300 lines of schema lowering | Not tracked here; upstream concern |
| Application | ~5,600 lines of building semantics | Not tracked here; product concern |

No capability was left unclassified. Nothing owned by the format layer or the
application layer has been proposed as kernel work.
