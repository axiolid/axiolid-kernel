# Crate map

The facade is convenient; leaf packages are the enforceable boundaries. Select the smallest package set that satisfies the use case. The generated, metadata-checked inventory is the [architecture crate map](/architecture/crate-map).

| Layer | Packages | Responsibility |
| --- | --- | --- |
| Foundation | `axiolid-core` | Numeric policy, identity, errors, bounds, tolerance |
| Representations | `axiolid-curve`, `axiolid-surface`, `axiolid-primitive`, `axiolid-profile`, `axiolid-topology`, `axiolid-brep`, `axiolid-mesh`, `axiolid-field`, `axiolid-model` | Portable geometry values and authored graph |
| Guarantees/common contracts | `axiolid-guarantees`, `axiolid-contracts`, `axiolid-mesh-contracts` | Proof/refusal vocabulary, execution diagnostics, shared mesh admissibility |
| Operation contracts | `axiolid-tessellation-contract`, `axiolid-mesh-boolean-contract`, `axiolid-mesh-section-contract`, `axiolid-mesh-compile-contract` | Typed provider-neutral request/result/evidence seams |
| Algorithms | `axiolid-reference`, `axiolid-nurbs`, `axiolid-construct`, `axiolid-spatial`, `axiolid-measure`, `axiolid-overlay`, `axiolid-field-ops`, `axiolid-heal` | Format-neutral implementations over values/contracts |
| Providers | `axiolid-mesh-boolean-boolmesh` | Concrete optional operation provider |
| Execution | `axiolid-dispatch`, `axiolid-mesh-compile`, `axiolid-backend-cpu`, `axiolid-backend-gpu` | Registration, fallback/device policy, graph execution, contexts/adapters |
| Facade | `axiolid` | Additive capability features and re-exports |

## Selecting a package

- Core scalar/vector/transform/tolerance values: `axiolid-core`.
- Mesh or sampled-field values without algorithms: `axiolid-mesh` or `axiolid-field`.
- Sampling/morphology/navigation over fields: `axiolid-field-ops`.
- Conservative numerical predicates/reference behavior: `axiolid-reference`.
- NURBS analysis and exact shape-preserving transformations: `axiolid-nurbs`.
- Neutral authored graph storage: `axiolid-model`.
- Exact analytic B-rep results: `axiolid-brep`; this is not a tessellator.
- Mesh Boolean or plane-section portability: depend on the operation contract, then choose a provider/dispatch policy explicitly.
- Graph-to-mesh execution: `axiolid-mesh-compile-contract` for the seam and `axiolid-mesh-compile` for the reference implementation.

Representation-only facade use remains:

```toml
axiolid = { default-features = false, features = ["model"] }
```

This must not resolve compiler, field-operation, mesh-Boolean-provider, source-format, or GPU dependencies.
