# ADR 0024: Exact B-rep result contracts own analytic support and trims

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Axiolid maintainers
- **Related:** [ADR 0020](./0020-exact-brep-kernel-model.md), [ADR 0022](./0022-general-nurbs-kernel-capability.md), [ADR 0023](./0023-solid-generation-is-an-l2-crate.md)

## Context

ADR 0020 makes analytic B-rep geometry and topology the intended kernel model. The
existing `axiolid-topology::BRep<G>` was intentionally a generic, partial topology
container for graph and import use. A single `G` can stand for a curve, pcurve, or
surface, and its optional supports carry no bounded native parameter intervals.
That is useful for incomplete imported data but cannot itself be an exact result:
a line, circle, or NURBS support alone does not identify the bounded edge it
contributes to a face.

`axiolid-construct` currently returns `TriMesh`. It must retain that working,
explicitly discrete path while gaining a future analytic construction path without
silently sampling, inverting, or triangulating an exact request.

## Decision

Introduce `axiolid-brep` as an L1 representation crate. It composes neutral
`axiolid-curve`, `axiolid-surface`, and `axiolid-topology` values, but implements
no solving, intersections, projection, or tessellation.

### Typed topology roles

`axiolid-topology::BRep` is now `BRep<Curve3, Curve2 = Curve3, Surface = Curve3>`.
The defaults preserve graph callers such as `BRep<NodeId>`, while exact callers
cannot accidentally provide a surface handle where an edge requires a 3D curve or
where a face use requires a 2D pcurve.

### Strict exact result

`axiolid_brep::ExactBRep` owns:

- an `ExactTopology = BRep<Curve3Id, Curve2Id, SurfaceId>`;
- separate owned `Curve3`, `Curve2`, and `Surface` catalogs with typed handles;
- one finite, non-zero native interval for every exact edge support;
- one finite, non-zero native interval for every face-loop pcurve use.

The edge interval is oriented from the edge start vertex to its end vertex. The
pcurve interval is oriented in the owning loop use traversal. This retains the
orientation needed for periodic supports, seams, and reverse uses without
coordinate inference.

`ExactBRepBuilder::finish` refuses an empty result, invalid generic topology,
missing curve/pcurve/surface supports, dangling catalog handles, and zero or
non-finite trim intervals. It does **not** assert geometric coincidence,
intersection completeness, shell closure, or manifoldness beyond the generic
structural audit. Those assertions require the curve/surface inverse and
intersection capabilities identified by ADR 0020 and must be added as certified
validators, never guessed from tessellation.

A generic `BRep` may remain partial. It is not an `ExactBRep`, and cannot be
upgraded by assigning invented intervals or approximating missing pcurves.

### Normative topology, ownership, and adjacency

The following terms are normative for every future exact construction API.

- A **vertex** is one owned `VertexId` and its model-space `Point3`. Vertex
  identity is handle identity, **not** coordinate equality; constructors MUST NOT
  weld independently created vertices merely because their coordinates are close.
- An **edge** is an ordered `(start, end)` pair of `VertexId`s, one mandatory
  `Curve3Id`, and one mandatory finite, positive native curve interval. That
  interval runs from `start` to `end` for `Orientation::Forward`; a user reverses
  traversal through an `EdgeUse`, not by mutating the carrier interval. A
  non-zero span with `start == end` is permitted for a periodic seam edge.
- An **edge use** is a particular traversal of an edge in one loop. It MUST own a
  `Curve2Id` and finite, positive native pcurve interval in the parameter chart of
  the face that consumes the loop. Two faces using one edge normally have distinct
  pcurves. A pcurve is never recovered by sampling or inversion as an implicit
  fallback.
- A **loop** is a cyclic ordered sequence of oriented edge uses. Adjacent uses
  MUST meet by the same vertex handle after their orientations are applied. A
  **face bound** selects a loop and its sense; every face has exactly one outer
  bound and zero or more inner bounds.
- A **face** owns one `SurfaceId`; its `Orientation` selects whether its material
  side agrees with or opposes the support surface normal. Its bounds trim that
  surface only through their face-local pcurves and intervals.
- A **shell** is an oriented collection of face uses. Its `closed` flag is an
  assertion that must be checked by `audit_brep`, not trusted from input. A
  **solid** owns one outer shell and zero or more void shells. The outer-shell
  material orientation is outward; void-shell material orientation is inward.

`ExactBRep` owns all three support catalogs and the topology that references
those catalogs. Catalog handles, topology IDs, and trim-map keys are stable only
inside that result; a result is neither a global geometry registry nor a source
format identity map. Shared topological boundaries are represented by shared
`EdgeId`s and edge uses, never coordinate welding or independently split sampled
polylines.

`audit_brep` supplies the current exact, coordinate-free adjacency policy. An
open exact sheet is valid when `BRepHealth::is_tessellable()` holds. A generator
MUST call a result a **closed-manifold exact solid** only when it selects a
`Solid` root whose outer/void shells are the only shells in the result and the
result satisfies `BRepHealth::is_closed_manifold()`: no dangling references,
open/empty loops,
missing or duplicate outer bounds, unpaired edge uses, overused edges, or false
closure claims. Geometric containment of voids, self-intersection, orientability
at singularities, and curve/pcurve/surface coincidence are deliberately not
inferred by this structural audit; they require a future certified geometric
validator.

### Tolerance and certification policy

`ExactBRep` preserves analytic/topological intent. It does not claim symbolic
real arithmetic or attach an unstated global epsilon to the model. Native trim
interval validity is exact as a representational invariant: endpoints must be
finite and the interval length positive. Any operation that tests endpoint
residuals, pcurve-on-surface agreement, closure, containment, self-intersection,
or manifoldness beyond handle adjacency MUST take an explicit
`axiolid_core::Tolerance` and report the policy in its diagnostic/result.

A tessellation tolerance is not an exact-B-rep tolerance. It only controls an
explicit `TessellationRequest` and must never change support identity, trim
intervals, topology adjacency, or the outcome of an exact request.

### Explicit output request

`axiolid-construct` exposes a capability-explicit result boundary:

```rust
pub enum GenerationRequest {
    ExactBRep,
    Tessellation(TessellationRequest),
}

pub enum GeneratedGeometry {
    ExactBRep(ExactBRep),
    Tessellation(TriMesh),
}
```

`TessellationRequest` requires an explicit `Tolerance`. A generator matching
`ExactBRep` must either return `GeneratedGeometry::ExactBRep` or report its
structured unsupported/invalid error. It must not return a mesh fallback. Existing
mesh-returning generation functions remain their own documented discrete APIs
until equivalent exact constructors exist.

The facade exposes the value contract through the opt-in `brep` feature and makes
`generate` depend on it. Leaf consumers can depend directly on `axiolid-brep`.

### Result and failure states

The request/result relation is total with respect to representation choice:

| Request | Allowed successful result | Forbidden successful result |
| --- | --- | --- |
| `ExactBRep` | `GeneratedGeometry::ExactBRep` | `GeneratedGeometry::Tessellation` |
| `Tessellation(request)` | `GeneratedGeometry::Tessellation` | an implicit exact-to-mesh conversion not requested by the caller |

A future dispatching exact constructor MUST use a structured `Result` error whose
semantics distinguish at least these classes:

1. **Unsupported exact capability** — a required carrier, pcurve, inverse,
   intersection, or certified validator is not implemented for the requested
   family. This is the correct outcome for an otherwise valid exact request that
   the current generator cannot construct.
2. **Invalid exact input** — the supplied profile/path/supports cannot describe
   the requested topology, or strict assembly fails with `ExactBRepError`.
3. **Certification failure** — an explicitly requested geometric or manifold
   claim cannot be proved under the supplied tolerance. This differs from an
   unsupported algorithm: a validator ran and rejected this input.
4. **Numerical failure** — a supported algorithm exhausted its defined numerical
   safeguards. It MUST carry diagnostics, not be relabelled as a successful mesh.

This is a semantic contract, not an unused premature provider trait. The current
crate has no common request-dispatch entry point yet; when one is added it must
map `ExactBRepError` and the four classes above into its public error vocabulary.
Existing explicit mesh functions keep their current result/error signatures.

### Acceptance scenarios

These scenarios are normative acceptance targets for the first exact generators;
they are examples of the required output, not a claim that those constructors
exist today.

#### 1. Planar-profile linear extrusion

Given a planar rectangular profile with an optional inner rectangular void and a
finite linear directrix, an exact extrusion is accepted only if it constructs:

- one owned vertex for every topological endpoint, shared through `VertexId`;
- line or analytic profile `Curve3` supports for the cap boundaries, line
  `Curve3` supports for longitudinal edges, and finite native spans for each;
- plane supports for both caps and ruled/analytic support surfaces for side faces;
- face-local `Curve2` pcurves and intervals for every edge use on every cap and
  side face, including separate pcurves for the same shared edge on adjacent
  faces;
- one selected outer shell whose cap inner bounds and side faces form the
  through-passage for a profile hole, and whose complete topology passes
  `BRepHealth::is_closed_manifold()`. An additional `Solid::voids` shell is used
  only for an enclosed internal cavity, not for a through-hole.

The result is `GeneratedGeometry::ExactBRep`. Producing the existing `TriMesh`
extrusion instead is valid only under an explicit `TessellationRequest`.

#### 2. Periodic cylindrical side face

A capped cylinder with a periodic cylindrical side support demonstrates why
native parameter intervals are mandatory. Its side-face boundary uses pcurves in
that cylinder's `(u, v)` chart; seam-adjacent uses may have distinct intervals
near opposite ends of a periodic domain even when they meet the same 3D edge.
The generator MUST preserve those supplied native intervals and face-use
orientations. It MUST NOT normalise them through endpoint coordinates, weld a
seam by sampling, or substitute a faceted cylinder for an exact request.

#### 3. Unsupported inverse/intersection prerequisite

If a sweep needs B-spline surface inversion, curve/surface intersection, or a
pcurve that the generator cannot construct and certify, an `ExactBRep` request
produces the structured unsupported/certification failure. The caller may issue a
separate `GenerationRequest::Tessellation(...)` request to obtain the
already-supported discrete path; the exact request itself still never succeeds
with a mesh.

## Consequences

- `axiolid-construct` has a stable output contract for future exact extrusion,
  revolution, sweep, loft, profile, centre-line, and half-space construction.
- `axiolid-compile` remains L3 DAG/cache orchestration. Its cache can later store
  `ExactBRep` and perform a separate tolerance-bearing tessellation request;
  it must not collapse exact nodes to `TriMesh`.
- The current mesh construction path remains supported and clearly discrete.
- New exact construction must populate 3D supports, pcurves, and native spans as
  it constructs topology. It must refuse when it cannot do so exactly.
- No ISO/source-format terminology, schema types, or adapter dependency enters
  topology, B-rep, or generation layers.

## Alternatives rejected

### Store only `TriMesh`

A mesh cannot preserve analytic supports, trim intervals, topology identity, or
future CAD/editability operations. Higher sampling density reduces but never
eliminates that loss.

### Put curve/surface catalogs in `axiolid-topology`

That would make the generic topology container depend on particular representation
crates and break graph/import reuse. `axiolid-brep` is the correct composition
boundary.

### Add a generic `Sweeper`/result trait

There is no second working exact provider yet. A trait would be an empty seam;
the concrete value contract is real and immediately testable.

### Use mesh fallback for unavailable exact generation

It violates ADR 0020 and makes caller intent unobservable. Explicit tessellation
is already a supported request when a discrete result is desired.

## Follow-up

1. Add certified geometric validation for support endpoint residuals, pcurve/
   surface agreement, loop containment, seams, shells, and solid orientation.
2. Implement analytic exact constructors one family at a time, beginning only
   where all supports and trims are constructible without approximation.
3. Refactor graph compilation cache/result typing around `ExactBRep`; add a
   separate B-rep tessellation adapter with topology-owned shared boundaries.
4. Add curve/curve, curve/surface, surface/surface, inverse, projection, and
   closest-point capability contracts before operations that require them.
