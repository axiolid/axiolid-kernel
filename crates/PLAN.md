# Geometry package plan

Status: active architecture scaffold
Last updated: 2026-08-30

This is implementation planning, not ambient agent context. Read `AGENTS.md`
for standing rules. Read this file only when planning or implementing geometry
capabilities.

## 1. Goal

Build a pure-Rust, IFC-agnostic geometry package family that:

- is useful as small independent crates;
- supports every geometric concept needed by `ifc-geometry` without importing
  IFC names or schema types;
- defaults to a portable, deterministic mesh-first path;
- makes exact topology, NURBS, robust booleans, parallel execution, SIMD, and
  GPU execution additive capabilities rather than mandatory dependencies;
- can absorb useful algorithms from the Solibri sibling through adapters and
  ports, without preserving its domain vocabulary or monolithic crate shape;
- permits future x86_64, AArch64, discrete-GPU, integrated-GPU, and foreign
  kernel implementations behind stable contracts.

## 2. Non-negotiable constraints

- No C++ or system geometry library in the dependency graph.
- No IFC, rendering, persistence, product-domain, GUID, or rule-engine types in
  `crates/`.
- Units and tolerances are explicit values. No process-global epsilon.
- Every optimized provider requires a portable scalar correctness oracle; until
  that oracle exists, the operation trait is not implemented or registered.
- Feature-disabled configurations compile and test independently; workspace
  feature unification is not accepted as proof.
- Capability absence is a structured `Unsupported` result, never a panic,
  descriptor flag, or silent approximation.
- Invalid and dirty geometry remains diagnosable and, where safe, representable.
- Performance claims require benchmarks. Optimized backends require
  differential tests against the scalar oracle.
- Public data types implement the standard traits clients reasonably expect;
  floating-point types do not claim `Eq` or `Hash` unless canonicalized.

## 3. Audited baseline

- IFC4 ADD2 TC1 contributes exactly 163 declarations: 112 entities, 23 types,
  and 28 functions. The committed normative manifest and tests guard all 163.
- `ifc-geometry` has owners for every declaration. Its 89 concrete entity views
  or family views are present; the 23 types are 13 selects, seven enums, and
  three defined types. Six vector helpers delegate to standard geometry
  primitives and 22 EXPRESS functions are explicitly scaffolded, not falsely
  reported as implemented.
- Exact profiles and extrusion/revolution lower into `GeometryGraph`. Remaining
  representation families have neutral node/type owners but still need lowering
  and algorithm implementations.
- Backend implementations are separate crates. This closes the Cargo feature-
  unification leak and lets format adapters depend on representations only.
- GPU integration is batch-oriented through open `GpuGraphExecutor`; no API or
  hardware vendor is selected by the contract.
- Robust 2D boolean and pure-Rust mesh CSG dependencies remain implementation-
  wave decisions and require measured dependency/quality evidence.

### Normative per-declaration checklist

The checked per-entity/type/function map is the committed
[`ifc4-add2-tc1-geometry-support.tsv`](../ifc/ifc-geometry/references/ifc4-add2-tc1-geometry-support.tsv).
Each of its 163 rows names the IFC bridge owner, neutral geometry owner, and
current support status. It is the PLAN checklist rather than a duplicated prose
table, so the executable coverage test and human plan cannot drift apart.

- [x] `IfcGeometryResource`: 98 declarations mapped.
- [x] `IfcGeometricModelResource`: 48 declarations mapped.
- [x] `IfcGeometricConstraintResource`: 17 declarations mapped.
- [x] Every bridge owner is unique and every neutral owner is non-empty.
- [x] Every non-`axiolid_core` function owner resolves to a Rust module on disk.

`Scaffolded` means ownership and a compiling target exist; it does not mean the
geometry is evaluated. Capability claims advance only when executable provider
traits and behavior tests exist.

## 4. Prior-art decisions

Detailed source pins and observations live in
`docs/research/geometry-prior-art-synthesis.md`.

### IfcOpenShell

Adopt its explicit IFC-to-neutral taxonomy boundary, broad representation
coverage, corpus/oracle testing, and batching of openings. Avoid its OpenCascade
coupling, process-wide tolerance conventions, and a build where advanced exact
machinery is inseparable from simple data use.

### IFC-Lite

Adopt the pure-Rust exact-predicate cascade, operation routing, profile-level 2D
cuts, union-before-subtract strategy, mapped-geometry cache, relative-to-center
coordinates, property tests, and independent triangulation oracle. Improve on
its current monolithic IFC-coupled geometry crate: schema parsing, texture
serialization, Rayon, and exact CSG must remain independently selectable here.

### That Open

Adopt the reusable-geometry/placed-instance split, lazy derived meshes, compact
typed buffers, batched worker protocol, cancellation, and memory-budgeted cache
policy. Avoid copying web-ifc's parser/schema/geometry processor coupling, early
mesh flattening, global segment/tolerance settings, recursive flag-heavy shape
types, rendering colors in geometry, and API-specific WASM/Three.js types.

### Solibri sibling

Adopt the narrow subtractor seam, cheap 2D common-case path, invariant-based
validation, Python oracle generation, and cross-process determinism tests. Do
not import BIM query vocabulary, rendering/serialization concerns, C++
`manifold3d`, stale placeholder modules, or vendor compatibility constraints.

### Resulting Rust design

Use immutable typed-handle DAG composition, narrow open operation traits,
operation-specific executable registries, builders only for validated
configuration, explicit policy values, borrowed views, and batch-first APIs.
Trait implementation is capability proof; descriptor booleans are forbidden.

## 5. Implemented layers

```text
L0 values            axiolid-core
L1 representations   axiolid-mesh, axiolid-profile, axiolid-curve, axiolid-surface,
                     axiolid-topology, axiolid-brep, axiolid-primitive, axiolid-model
L2 algorithms/traits axiolid-tessellate, axiolid-spatial, axiolid-measure,
                     axiolid-overlay, axiolid-field, axiolid-heal, axiolid-nurbs,
                     axiolid-reference, axiolid-construct, axiolid-kernel
L3 execution/adapters axiolid-mesh-boolean-boolmesh, axiolid-compile, axiolid-backend-cpu,
                     axiolid-backend-gpu
L4 facade             axiolid
L5 format bridges     ifc-geometry and future adapters (outside this directory)
```

The graph direction is executable in `axiolid-core/tests/layering.rs`.
`axiolid-kernel` contains no implementation features. Active `ifc-geometry`
lowering emits the neutral graph rather than IFC-local profiles, primitives, or
CSG requests; its legacy `kernel` namespace only preserves pre-DAG source
compatibility and is not accepted by execution providers. The names remain
warning-clean because additive compatibility includes clients that deny warnings.
Where an established legacy root name collides with a neutral value, the legacy
type keeps the short name and the neutral value uses an explicit alias such as
`AnalyticPrimitive`, `ExactProfile`, or `GeometryBooleanOperator`.

## 6. Facade capabilities

`axiolid` defaults to `mesh + cpu`: f64 core values, discrete mesh values, and the
portable CPU execution context. It does not pull operation contracts, the exact
model graph, Rayon, or GPU code.

Additive facade features:

| Family | Features | Pulls |
| --- | --- | --- |
| representation | `mesh`, `profiles`, `curves`, `surfaces`, `topology`, `brep`, `primitives`, `model` | exact/data crates only |
| algorithms/contracts | `nurbs`, `generate`, `tessellation`, `spatial`, `measure`, `heal`, `kernel`, `mesh-boolean`, `graph-compile` | selected traits/algorithms |
| execution | `cpu`, `parallel`, `simd`, `gpu` | context or operation adapter only |
| bundles | `discrete`, `parametric`, `advanced`, `full` | named additive sets |

`parallel` and `simd` are opt-in. `gpu` exposes the executor adapter but claims no
working API-specific compute kernels. Leaf crates remain directly consumable.

Measured 2026-08-30 as unique Cargo package identities, including the `axiolid`
facade and excluding repeated `(*)` display rows (not binary size):

```bash
cargo +1.88.0 tree -p axiolid -e normal --prefix none <feature flags> |
  python3 -c "import sys; print(len({line.strip().removesuffix(' (*)') for line in sys.stdin if line.strip()}))"
```

| Build | Feature flags | Unique packages |
| --- | --- | ---: |
| core-only | `--no-default-features` | 3 |
| default (`mesh + cpu`) | *(none)* | 5 |
| `parametric` | `--no-default-features --features parametric` | 21 |
| `discrete` | `--no-default-features --features discrete` | 21 |
| `full` bundle | `--no-default-features --features full` | 31 |
| every facade feature | `--all-features` | 39 |

These are regression baselines, not permanent targets; dependencies must justify
any increase.

## 7. Implementation waves

Completed scaffold:

- [x] Authoritative 163-declaration manifest and executable owner coverage.
- [x] One canonical active neutral vocabulary; pre-DAG IFC-local request names
      retained only as legacy source-compatibility values.
- [x] Growth-shaped modules plus progressive `AGENTS.md`/`PLAN.md` boundaries.
- [x] Narrow operation traits, batch-capable executable mesh-boolean registry,
      graph-owned family- and dimension-validated DAG references, runtime-gated
      CPU context, and policy-validating GPU graph-compiler adapter.
- [x] Rust-AST active-lowering isolation, facade feature matrix, architecture
      gates, API-trait checks, and mutation verification.

Implementation waves:

1. Portable providers: profile triangulation and 2D subtraction first, then
   tessellation and a separately justified pure-Rust mesh CSG provider.
2. Complete IFC graph lowering by family: points/transforms, curves, surfaces,
   topology/B-rep, tessellated sets, half-spaces/CSG, mapped representations.
3. Add spatial, measurement, and explicit heal providers with corpus invariants.
4. Add bounded caches, diagnostics, cancellation, and workload/budget reporting.
5. Add measured SIMD and local-Rayon paths with differential scalar tests.
6. Add a concrete GPU provider only for proven batch-friendly operations; keep
   CPU fallbacks explicit and precision-correct.
7. Add benchmark harnesses; make no performance claim before measurements.

## 8. Validation strategy

- `cargo build/test/clippy/doc` for the full workspace and relevant feature
  combinations, checking command exit codes.
- Isolated builds for every bridge, contract, and backend crate.
- `cargo tree -e features` assertions for lean configurations.
- Compile-fail or manifest/source architecture tests for forbidden edges.
- Generated normative manifest plus explicit owner ledger: 112 entities + 23
  types + 28 functions; missing or duplicate declarations fail the gate.
- Stub/foreign kernel implementation proving contracts are implementable.
- Differential scalar vs SIMD/parallel/GPU tests once implementations exist.
- Determinism tests across process boundaries for output ordering and caches.

## 9. Risks and rollback

- Too many crates: keep crates at independently useful compile/dependency
  boundaries; use modules for ordinary code organization.
- Feature explosion: features name additive capabilities, never products or
  hardware models. Prefer separate backend crates over mutually interacting
  feature flags.
- False 100% claim: distinguish `represented`, `interpreted`, `evaluated`, and
  `validated`; report each separately.
- Premature exact B-rep: keep the seam, implement mesh-first, and require corpus
  evidence before funding surface/surface intersection machinery.
- Shared master: stage paths narrowly and verify HEAD before every commit.

## 10. Next concrete action

Implement the first complete vertical slice: exact profile -> invariant-checked
triangulation -> 2D subtract provider -> extrusion mesh. Verify against Solibri
area invariants, an independent triangulator, and IFC corpus fixtures before
starting general 3D CSG.
