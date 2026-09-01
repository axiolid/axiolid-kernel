# IFC geometry upstream gaps: plane section first

Status: active
Owner boundary: Axiolid implements geometry operations; format adapters only lower/select intent.

## Goal

Make the current end-to-end Body-to-plan path executable without adding geometry algorithms to a source-format adapter:

```text
Body representation -> neutral GeometryGraph -> application-selected compiler
-> validated oriented TriMesh -> backend-neutral plane section
-> plane-local deterministic linework -> application-manufactured Plan
```

The first slice is explicitly a section of the compiled mesh. It preserves the compiler/tessellation approximation provenance and must never be described as an exact analytic section. A later tier accepts a validated closed `ExactBRep`/`ExactSolid` and returns analytic section curves.

## Constraints

- Pure Rust, format/vendor neutral, no source-format names in `crates/`.
- `axiolid-kernel` owns an open executable operation trait; no implementation.
- `axiolid-reference` lands the portable correctness oracle before optimized providers.
- Topology decisions use certified exact orientation against the represented affine plane, never a constant epsilon.
- Tolerance is only for validating the requested frame and output quality contracts.
- Input must be a finite, consistently wound, closed two-manifold triangle solid.
- Coplanar triangles represent a 2D overlap, not a section curve; refuse explicitly.
- Tangent-only point/edge contact emits no manufactured area. An on-plane mesh edge is retained only when its two incident third vertices lie on opposite sides.
- Connectivity is keyed by source mesh vertex/edge identity, never distance welding.
- Nonempty output must be disjoint degree-two cycles; open/branched results fail closed.
- Every allocation/work count uses checked arithmetic and an explicit caller budget.
- Cancellation is checked incrementally during triangle/edge scans and loop assembly.
- Deterministic ordering: canonical node keys, canonical cycle start, sorted contours.
- Do not modify or rely on the protected canonical checkout.

## Contract

Add an opt-in `mesh-section` feature to `axiolid-kernel`.

Input:
- `TriMesh` solid;
- right-handed orthonormal `Frame3`; its XY plane is the cut and its Z axis is the positive plane normal;
- `ExecutionOptions` for tolerance, memory budget, determinism, cancellation;
- explicit `SectionLimits` bounding source vertices/triangles, section nodes/segments, and contours.

Output:
- validated section frame;
- closed `SectionContour` values containing plane-local `Point2` vertices without a duplicated terminal point;
- `SectionEvidence` recording source and emitted counts plus input-mesh approximation provenance.

This first contract is linework. Beyond removing redundant collinear tessellation vertices, it does not classify outer rings versus holes, infer filled regions or cut heights, or author source-format records. Those are separate operations/application policy.

## Workstreams

### 1. Kernel contract

- Add `Operation::MeshPlaneSection`.
- Add section value/evidence/limits types and `MeshPlaneSection` trait.
- Add a registry with the established fallback rule: only `Unsupported` and `Unavailable` fall through; invalid, degenerate, budget, cancellation, and contract failures fail fast.
- Registry pre-validates input/frame/limits and post-validates provider output and evidence.
- Add contract tests for invalid frames, budget preflight, provider fallback/fail-fast, and malformed provider output.

### 2. Scalar oracle

- Classify each vertex with certified `orient3d` against the affine plane represented by the frame, while retaining one finite approximate distance for interpolation.
- Generate section nodes from exact-on-plane vertices and proper opposite-sign source edges.
- Resolve exact-on-plane source edges using both incident third-vertex signs; refuse coplanar triangles and non-two-manifold incidence.
- Assemble source-identity segments into degree-two cycles.
- Convert 3D section points to plane-local XY using the validated frame.
- Canonicalize and sort cycles deterministically; reject zero-area/duplicate-point output.
- Tests: empty cut, box cut, multiple components, hole-like nested loops, arbitrary frame, near-plane exact-sign regression, through-vertex, through-edge, coplanar refusal, open/non-manifold refusal, limits, cancellation, determinism.
- Independent oracle: analytic axis-aligned boxes with known rectangles and sampled 3D-on-plane reconstruction.

### 3. Facade and documentation

- Add facade `mesh-section` feature and include it in discrete/advanced/full bundles.
- Update capability table, roadmap, changelog, feature matrix, `AGENTS.md` ownership lists, and an ADR defining approximation and degeneracy semantics.
- Document the generic recipe DAG -> compiler -> mesh section -> application linework authoring; do not claim exact B-rep sectioning.

### 4. Future exact tier (not claimed by this slice)

- Define and validate `ExactSolid` as a closed, oriented `ExactBRep` with connected-shell semantics.
- Complete certified plane/surface and surface/surface intersection-curve construction for all admitted support families.
- Trim/stitch analytic section curves by face topology and classify tangencies/overlaps.
- Return analytic `Curve2` contours with proof/refusal evidence.
- Preserve provider separation so exact and mesh section capabilities are independently selectable.

## Validation strategy

1. RED-GREEN focused contract and scalar tests.
2. Exact targeted mutations: bypass exact sign, manifold gate, coplanar refusal, degree-two gate, budget precharge, and approximation evidence validation.
3. Strict crate and workspace Clippy.
4. Release workspace/all-feature tests.
5. Feature-isolation and layering probes.
6. Docs production build and canonical `scripts/gate.sh`.
7. Freeze exact tree, run two independent immutable reviews, re-fetch remote, and CAS-publish only an approved fast-forward successor.

## Risks and rollback

- Vertex/edge degeneracies are the highest topology risk; fail closed rather than weld.
- The mesh tier can unblock current execution but cannot improve source tessellation accuracy. Evidence must keep that visible.
- Adding output-region semantics prematurely would mix sectioning, polygon overlay, and application policy. Keep first output as closed linework.
- Rollback is one additive commit/feature; no neutral representation behavior changes.

## Next concrete action

Freeze the bounded-audit successor, rerun mutation/resource probes and both immutable reviews, then CAS-publish before updating the downstream Axiolid pin.
