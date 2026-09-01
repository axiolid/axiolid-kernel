# Crate map

The facade is convenient; the leaf crates are the actual boundaries. Consumers should select the smallest crate set that satisfies their use case.

| Layer | Crates | Responsibility |
| --- | --- | --- |
| Facade | `axiolid` | Feature-gated re-exports; default is intentionally small |
| Values | `axiolid-core`, `axiolid-mesh`, `axiolid-primitive`, `axiolid-profile`, `axiolid-curve`, `axiolid-surface`, `axiolid-topology`, `axiolid-brep` | Neutral geometry, generic topology, and strict analytic B-rep result vocabulary |
| Graph | `axiolid-model` | Immutable typed geometry DAG |
| Algorithms | `axiolid-reference`, `axiolid-nurbs`, `axiolid-construct`, `axiolid-tessellate`, `axiolid-spatial`, `axiolid-measure`, `axiolid-overlay`, `axiolid-field`, `axiolid-heal` | Scalar/reference algorithms and focused operations; `axiolid-construct` builds the current discrete solid result directly from profiles and paths |
| Graph compilation | `axiolid-compile` | L3 DAG traversal, node caching, model-driven directrices, and current B-rep face tessellation; consumes algorithms rather than owning generation |
| Contracts | `axiolid-kernel` | Operation contracts, policy, backend identity and errors |
| Execution | `axiolid-backend-cpu`, `axiolid-backend-gpu`, `axiolid-mesh-boolean-boolmesh` | CPU context, GPU seam, and an optional mesh Boolean provider |

## Selecting a crate

- Need points, vectors, transforms, bounds, and tolerance? Start with `axiolid-core`.
- Need mesh values without a facade? Use `axiolid-mesh`.
- Need conservative numerical predicates? Use `axiolid-reference`.
- Need general NURBS analysis, bounded inverse queries, or exact shape-preserving transformations? Use `axiolid-nurbs`.
- Need neutral shared graph storage? Add `axiolid-model`.
- Need an owned exact analytic B-rep result—typed 3D supports, pcurves, surfaces,
  topology, and native trim intervals? Use `axiolid-brep` or the facade's `brep`
  feature. This is a representation contract, not a tessellator; see [ADR 0024](/adr/0024-exact-brep-result-contracts).
- Need the current scalar profile/path-to-mesh construction without a graph? Use `axiolid-construct` or the facade's `generate` feature. It is deliberately a discrete reference path today; new callers must request exact B-rep or explicit tolerance-bearing tessellation rather than infer one from the other. See [ADR 0020](/adr/0020-exact-brep-kernel-model), [ADR 0023](/adr/0023-solid-generation-is-an-l2-crate), and [ADR 0024](/adr/0024-exact-brep-result-contracts).
- Need execution? Depend on the operation contract and a provider explicitly; do not assume the facade selects an algorithm for you.

Read individual manifests for feature dependencies. Feature bundles are documented in [Getting started](/guide/getting-started).
