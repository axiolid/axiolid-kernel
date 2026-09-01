# 0023 — Solid generation is an L2 crate, not part of the compiler

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Friedrich Schrödter
- **Supersedes:** [0021](./0021-capability-seams-live-in-the-kernel.md)

## Context

[ADR 0021](./0021-capability-seams-live-in-the-kernel.md) retired the empty
`axiolid-sweep` crate. That part was right: the crate held one trait with no
implementors, no tests, and no references.

The rest of it was wrong. It asserted that sweep construction "belongs with the
compiler" because it "depends on the compiler's evaluation context, its
tolerance policy and its profile machinery". That claim was never measured. It
is false.

Measuring the actual import graph of `axiolid-mesh-compile`:

| module        | internal deps    | external deps                                  |
|---------------|------------------|------------------------------------------------|
| `brep`        | —                | core, curve, kernel, mesh, **model**, surface, topology |
| `compiler`    | extrude, profile | core, kernel, mesh, **model**                  |
| `directrix`   | —                | core, kernel, **model**                        |
| `loft`        | profile          | core, kernel, mesh                             |
| `sweep`       | loft, profile    | core, kernel, mesh                             |
| `revolve`     | profile          | core, kernel, mesh                             |
| `extrude`     | —                | core, kernel, mesh, scalar                     |
| `profile`     | —                | core, curve, kernel, profile, scalar           |
| `center_line` | profile          | core, curve, kernel, profile                   |
| `half_space`  | loft, profile    | core, kernel, mesh, primitive                  |

The bottom seven modules form a closed cluster. They reference nothing outside
themselves, they never touch `axiolid-model`, and they perform no dispatch —
no kernel trait is called anywhere in the cluster. Their only use of
`axiolid-kernel` is the error vocabulary (`GeomError`/`GeomResult`, plus
`BackendId`, `Operation` and `Sign`, which appear *inside* those error
variants). Only `compiler` consumes them.

That is not "part of the compiler". That is a self-contained subsystem of
roughly 1,400 lines that happened to be born inside an L3 crate.

The distinction matters beyond tidiness. `axiolid-mesh-compile` is L3 because it
walks a DAG, caches by node, and reports as a provider. A caller who has an
exact profile and a path and wants a solid should not have to adopt a graph,
a cache, and a model vocabulary to get one — which, before this change, was
the only way to reach `extrude`.

## Decision

Extract the seven modules into a new crate, **`axiolid-construct`**, at **L2**.

`axiolid-mesh-compile` keeps exactly what its name implies: `compiler` (DAG walk,
caching, provider identity), `brep` (B-rep face tessellation) and `directrix`
(model-driven path extraction). It depends on `axiolid-construct` like any
other consumer.

L2 is the correct tier by the workspace's own rule — algorithms over L1
representations. `axiolid-construct` sits beside `axiolid-reference`,
`axiolid-tessellation-contract` and `axiolid-measure`: it consumes representation types
and produces meshes, and it solves rather than merely describes. It is not L1
(it is not a value vocabulary) and not L3 (it owns no execution context,
selects no backend, and implements no provider trait).

The crate reports its own `BACKEND_ID` of `scalar-generate`. Previously the
moved code identified its failures as `scalar-compile`, which is now simply
untrue: a refusal raised while building a swept solid did not come from the
compiler.

The facade exposes it as an opt-in `generate` feature and an
`axiolid::generate` module. `axiolid-mesh-compile` remains unexposed — the graph
compiler is an implementation detail; generation is a capability.

### On the retired `Sweeper` trait

ADR 0021's conclusion stands, for a better reason than it gave. A trait is
warranted when there is something to swap. `axiolid-construct` is a concrete
scalar implementation with no competing implementor, so it exports plain
functions. When a second implementation appears — a GPU tessellator, or an
exact B-rep sweep under [ADR 0020](./0020-exact-brep-kernel-model.md) — the
seam belongs in `axiolid-kernel` beside `MeshBoolean`, which is the one part
of 0021 that was correct.

Under ADR 0020 the eventual exact sweep returns a B-rep rather than a mesh.
That reinforces this placement rather than undermining it: an exact sweep is
still an algorithm over representations, so it belongs at L2 too, and having
generation already separated from graph evaluation is what makes adding it a
local change.

## Consequences

- `axiolid-mesh-compile` drops from 4,595 to roughly 3,200 lines and loses its
  direct `earcut` dependency in the generation path.
- Solid generation is usable without the DAG, the node cache, or
  `axiolid-model`. The facade's `generate` feature is verified by a test that
  builds a solid through `axiolid::generate` alone.
- Two functions that were `pub(crate)` — `sweep::linear_extrusion_normals` and
  `half_space::for_subject` — become public API. The extraction converted an
  implicit intra-crate coupling into an explicit, reviewable interface, which
  is the point.
- Seven test files moved with their modules. Two tests that cross the
  generation/boolean seam stayed at L3 in `axiolid-mesh-compile`, because
  `axiolid-mesh-boolean-boolmesh` is L3 and an L2 crate must not depend on it. The layering
  test caught this: the first attempt moved them wholesale and failed with
  `crates/algorithms/construction/construct (tier 2) depends on axiolid-mesh-boolean-boolmesh (tier 3)`.
- Test count is unchanged at 570, confirming the split moved work rather than
  dropping it.

## Alternatives considered

**Leave it in `axiolid-mesh-compile`.** Rejected: it makes generation unreachable
without the graph, and it was the unexamined position ADR 0021 defended.

**Move only `sweep` and `loft`.** Rejected: `sweep` depends on `profile`, and
`half_space` depends on both. Splitting the cluster would leave a
crate-crossing dependency where there is currently none.

**Put it at L3 beside the compiler.** Rejected: L3 is for execution contexts
and provider implementations. `axiolid-construct` selects no backend and
implements no operation trait.

**Reinstate a `Sweeper` trait in `axiolid-kernel` now.** Rejected: one
implementor. This is the mistake `axiolid-sweep` already made once, and
[ADR 0017](./0017-solid-boolean-contract-before-implementation.md) warns
against contracts written before a second implementation exists.
