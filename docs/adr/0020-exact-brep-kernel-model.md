# 0020 — Exact B-rep is the kernel model; tessellation is an output

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

Axiolid is intended to be a multipurpose B-rep kernel: usable for CAD
construction, for rule checking over building models, and for the analytical
work those applications need (clash and containment reasoning, section curves,
exact quantities). It is not intended to be a tessellation pipeline.

Today the implementation is a tessellation pipeline. The evidence is one line
in `crates/axiolid-compile/src/compiler.rs`:

```rust
type Cache = std::collections::HashMap<usize, TriMesh>;
```

Every node of the geometry graph is memoised **as triangles**. `compile()`
returns `GeomResult<TriMesh>`. `MeshBoolean` consumes and produces `TriMesh`.
No code path anywhere in the workspace produces a B-rep result. So a surface
the kernel can represent and evaluate exactly is discretised at the first graph
edge, and every downstream operation sees only the discretisation.

The cost is measurable rather than theoretical. A unit cylinder of radius 1 and
height 1, meshed as an inscribed prism with `n` angular segments:

| `n` | mesh volume | relative error |
| --- | --- | --- |
| 32 | 3.121445152 | 6.41e-03 |
| 64 | 3.136548491 | 1.61e-03 |
| 128 | 3.140331157 | 4.02e-04 |
| 256 | 3.141277251 | 1.00e-04 |
| 512 | 3.141513801 | 2.51e-05 |

against an exact value of `3.141592654`.

Two properties of that table drive this decision:

1. **The error is one-signed.** An inscribed mesh is strictly inside the exact
   solid, so volumes are systematically *under*-reported. Errors from chained
   operations accumulate; they do not cancel. Refinement buys accuracy at
   O(n) cost and never reaches the exact answer.
2. **It is unrecoverable.** Once the cylinder is a prism, no later stage can
   restore the fact that it was a cylinder. Exactness is lost at the earliest
   stage and every consumer inherits the loss.

For rule checking this is the difference between "this clearance is 2.4998 m,
which fails a 2.5 m rule" and a correct pass. A discretisation artefact that
changes a compliance verdict is a correctness defect, not a quality setting.

The kernel already contains the exact machinery this decision needs — validated
polynomial/rational B-spline evaluation with analytic derivatives
(ADR 0018, ADR 0019), analytic surface inversion for the elementary surfaces,
exact orientation and in-sphere predicates, and a neutral topology vocabulary
in `axiolid-topology`. What is missing is a model that *keeps* exact geometry
across an operation, and the intersection algorithms an exact model requires.

## Decision

We will treat **exact B-rep as the kernel's model of geometry**, and treat
tessellation as one requested *output* of that model rather than as the model
itself.

Concretely:

- The geometry model carries exact curves, surfaces, and topology through
  operations. A shape that entered as a cylinder is still a cylinder after an
  operation that did not cut it.
- Tessellation becomes an explicit, caller-requested projection out of the
  exact model, with a stated tolerance. It stops being the implicit currency
  between graph nodes.
- Operations that cannot yet be performed exactly must **refuse**, in the
  established fail-closed style, rather than silently substituting a
  discretised approximation. A mesh-only path may be offered explicitly, but
  never as an unannounced fallback for an exact request.
- Surface/surface intersection and geometric inversion (point→parameter,
  projection, closest point) are in scope and required. They are the
  prerequisites for exact booleans, section curves, offsets, and fillets, and
  they are removed from the non-goals list by this ADR.
- Mesh booleans remain a supported provider for mesh inputs and for callers who
  explicitly want a discrete result. They stop being the only way to combine
  two solids.

This ADR states direction and scope. It deliberately does not specify the
intersection algorithms, the B-rep result type, or the migration order for
`Cache`; those need their own ADRs with their own evidence, and are listed as
follow-ups below.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Stay a tessellation pipeline, document it honestly | Honest, and it is what the docs said before this ADR, but it cannot serve CAD construction or rule checking. A compliance verdict must not depend on segment count. |
| Keep meshes, raise the default resolution | Reduces the error without bounding it, costs memory and time on every node, and leaves it one-signed. `n=512` still under-reports a unit cylinder by 2.5e-05, and chained operations compound it. |
| Keep meshes, attach exact metadata for measurement | Splits truth across two representations that can disagree, and answers only measurement questions. Section curves, offsets, and exact booleans still have nothing to operate on. |
| Adopt an existing exact kernel (OpenCascade) | Excluded by ADR 0001 and the project's pure-Rust, vendor-neutral constraint. Also imports a C++ dependency graph and its tolerance model wholesale. |
| Exact model, but tessellate eagerly for every operation | This is the current behaviour under a different name. The exactness would be unobservable. |

## Consequences

**Positive**

- Measurements, containment tests, and clearances can be answered exactly for
  shapes the kernel represents exactly, rather than to within a mesh artefact.
- Chained operations stop compounding one-signed discretisation error.
- The existing exact evaluation and inversion work becomes load-bearing instead
  of being discarded at the first graph edge.
- Rule checking gets verdicts that do not depend on tessellation settings.
- Tessellation quality becomes a presentation concern with an explicit
  tolerance, which is where it belongs.

**Negative / costs**

- Surface/surface intersection is a large, genuinely hard body of work, and it
  is on the critical path for exact booleans.
- Robustness burden rises sharply. Exact B-rep booleans are where kernels
  classically fail: tangential contact, near-coincident faces, and seam
  degeneracies all need explicit policies rather than luck.
- `Cache = TriMesh` is load-bearing today. Changing it touches the compiler's
  evaluation core, and the change gets more expensive with every operation
  added meanwhile.
- The capability surface will honestly *shrink* first: operations that
  currently return an approximate mesh will refuse until an exact path exists.

**Follow-ups / risks to watch**

- A follow-up ADR must define the B-rep result type and the migration path for
  the compiler cache, including how a mixed exact/discrete graph is evaluated
  during the transition.
- A follow-up ADR must define the surface/surface intersection contract:
  supported surface pairs, degeneracy policy, tolerance model, and the
  independent oracles used to validate it.
- Point→parameter inversion exists today only for the elementary analytic
  surfaces. Spline inversion needs its own decision, and must refuse rather
  than fabricate parameters until it exists.
- Risk: partial migration leaves two competing models. Mitigation is that an
  exact request must refuse rather than silently fall back, so the boundary is
  always observable to the caller.

## Relation to existing code

- `crates/axiolid-compile/src/compiler.rs` — `type Cache = HashMap<usize, TriMesh>`
  and `compile() -> GeomResult<TriMesh>`; the concrete site this decision
  changes.
- `crates/axiolid-kernel/src/boolean.rs`, `crates/axiolid-kernel/src/solid.rs` —
  the mesh boolean contract and operand admissibility, which remain valid for
  mesh inputs.
- `crates/axiolid-topology` — the neutral B-rep vocabulary an exact result will
  populate.
- `crates/axiolid-scalar/src/surface.rs` — exact surface evaluation, normals,
  and analytic inversion for plane/cylinder/cone/sphere/torus.
- `crates/axiolid-scalar/src/curve.rs` — exact curve evaluation, derivatives,
  and adaptive flattening; flattening is an output path under this decision.
- `crates/axiolid-scalar/src/nurbs.rs` — validated spline evaluation from
  ADR 0019.
- `docs/capabilities.md`, `docs/ROADMAP.md` — updated alongside this ADR so the
  stated ambition and the stated status stay distinguishable.
