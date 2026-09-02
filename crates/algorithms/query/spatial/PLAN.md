# axiolid-spatial implementation plan

Status: BVH implemented; octree and uniform-grid providers remain planned.

## Established

- Crate boundary and dependency direction are executable in the layering gate.
- [`Bvh`](src/bvh.rs) is a read-only, deterministic median-split broad-phase
  provider. It rejects malformed bounds, preserves accepted input pair order,
  supports callback AABB/ray traversal, pair candidates, and filtered nearest
  queries.
- It is intentionally serial today. The public `SpatialIndex` callback seam
  leaves room for parallel CPU and GPU providers without coupling the contract
  to either execution strategy.

## Next implementation wave

- Benchmark this BVH against an external reference implementation on
  representative sparse, dense, and adversarial distributions before adding
  parallel build/query code.
- Add octree or uniform-grid providers only where a measured workload justifies
  their different update/query trade-offs.

## Exit evidence

Targeted differential tests, feature-isolated compile where applicable,
mutation-verified architecture/validation gates, and benchmarks before
performance claims.
