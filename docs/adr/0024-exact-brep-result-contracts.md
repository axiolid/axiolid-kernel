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

`axiolid-generate` currently returns `TriMesh`. It must retain that working,
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

### Explicit output request

`axiolid-generate` exposes a capability-explicit result boundary:

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

## Consequences

- `axiolid-generate` has a stable output contract for future exact extrusion,
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
