# Crate map

The facade is convenient; the leaf crates are the actual boundaries. Consumers should select the smallest crate set that satisfies their use case.

| Layer | Crates | Responsibility |
| --- | --- | --- |
| Facade | `axiolid` | Feature-gated re-exports; default is intentionally small |
| Values | `axiolid-core`, `axiolid-mesh`, `axiolid-primitive`, `axiolid-profile`, `axiolid-curve`, `axiolid-surface`, `axiolid-topology` | Neutral geometry and topology vocabulary |
| Graph | `axiolid-model` | Immutable typed geometry DAG |
| Algorithms | `axiolid-scalar`, `axiolid-nurbs`, `axiolid-compile`, `axiolid-tessellate`, `axiolid-spatial`, `axiolid-measure`, `axiolid-overlay`, `axiolid-field`, `axiolid-heal` | Reference paths and focused operations |
| Contracts | `axiolid-kernel` | Operation contracts, policy, backend identity and errors |
| Execution | `axiolid-backend-cpu`, `axiolid-backend-gpu`, `axiolid-boolmesh` | CPU context, GPU seam, and an optional mesh Boolean provider |

## Selecting a crate

- Need points, vectors, transforms, bounds, and tolerance? Start with `axiolid-core`.
- Need mesh values without a facade? Use `axiolid-mesh`.
- Need conservative numerical predicates? Use `axiolid-scalar`.
- Need general NURBS analysis, bounded inverse queries, or exact shape-preserving transformations? Use `axiolid-nurbs`.
- Need neutral shared graph storage? Add `axiolid-model`.
- Need execution? Depend on the operation contract and a provider explicitly; do not assume the facade selects an algorithm for you.

Read individual manifests for feature dependencies. Feature bundles are documented in [Getting started](/guide/getting-started).
