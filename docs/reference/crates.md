# Crate map

The facade is convenient; the leaf crates are the actual boundaries. Consumers should select the smallest crate set that satisfies their use case.

| Layer | Crates | Responsibility |
| --- | --- | --- |
| Facade | `axiolid` | Feature-gated re-exports; default is intentionally small |
| Values | `axiolid-core`, `axiolid-mesh`, `axiolid-primitive`, `axiolid-profile`, `axiolid-curve`, `axiolid-surface`, `axiolid-topology` | Neutral geometry and topology vocabulary |
| Graph | `axiolid-model` | Immutable typed geometry DAG |
| Algorithms | `axiolid-scalar`, `axiolid-nurbs`, `axiolid-generate`, `axiolid-tessellate`, `axiolid-spatial`, `axiolid-measure`, `axiolid-overlay`, `axiolid-field`, `axiolid-heal` | Scalar/reference algorithms and focused operations; `axiolid-generate` builds the current discrete solid result directly from profiles and paths |
| Graph compilation | `axiolid-compile` | L3 DAG traversal, node caching, model-driven directrices, and current B-rep face tessellation; consumes algorithms rather than owning generation |
| Contracts | `axiolid-kernel` | Operation contracts, policy, backend identity and errors |
| Execution | `axiolid-backend-cpu`, `axiolid-backend-gpu`, `axiolid-boolmesh` | CPU context, GPU seam, and an optional mesh Boolean provider |

## Selecting a crate

- Need points, vectors, transforms, bounds, and tolerance? Start with `axiolid-core`.
- Need mesh values without a facade? Use `axiolid-mesh`.
- Need conservative numerical predicates? Use `axiolid-scalar`.
- Need general NURBS analysis, bounded inverse queries, or exact shape-preserving transformations? Use `axiolid-nurbs`.
- Need neutral shared graph storage? Add `axiolid-model`.
- Need the current scalar profile/path-to-mesh construction without a graph? Use `axiolid-generate` or the facade's `generate` feature. It is deliberately a discrete reference path today, not an exact B-rep result claim; see [ADR 0020](/adr/0020-exact-brep-kernel-model) and [ADR 0023](/adr/0023-solid-generation-is-an-l2-crate).
- Need execution? Depend on the operation contract and a provider explicitly; do not assume the facade selects an algorithm for you.

Read individual manifests for feature dependencies. Feature bundles are documented in [Getting started](/guide/getting-started).
